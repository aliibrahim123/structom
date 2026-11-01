use core::ops::{BitAnd, BitOrAssign};

use alloc_crate::{format, string::ToString, vec::Vec};
use hashbrown::{HashSet, hash_map::Entry};
use lean_string::LeanStr;

use crate::{
	decl::{
		DeclFile, DeclFileBox, DeclNs, DeclProv,
		items::{ItemId, TypeId},
		provider::{ImportError, ImportResult},
		rich::Metadata,
	},
	parse::{
		Span,
		base::{Directive, meta_loop},
		str_cursor::{Cursor as StrCursor, Ident},
		token_cursor::*,
		tokens::{
			Block, brace_open, bracket_close, bracket_open, paren_close, paren_open,
		},
		value::parse_map,
	},
	profile::{Bin, Decl, Profile, Rich},
	utils::HashMap,
	value::Value,
};

#[derive(Debug)]
struct Import<'src> {
	path: Str<'src>,
	ns: Option<Ident<'src>>,
	items: Option<Vec<Ident<'src>>>,
	meta: Option<Value>,
}

fn parse_import<'src>(cur: &mut Cursor<'src, '_>) -> Option<Import<'src>> {
	cur.eat_delim(paren_open, paren_close, |cur| {
		let path = cur.str()?;
		cur.comma_unless_end();

		let (ns, items) = if cur.try_ident_kw("as") {
			(cur.ident(), None)
		} else {
			(None, cur.try_delim_list(bracket_open, bracket_close, Cursor::ident))
		};
		cur.comma_unless_end();

		let metamap = cur.test(brace_open).then(|| parse_map(cur, true)).flatten();
		Some(Import { path, ns, items, meta: metamap })
	})
}

enum UnknownFeat<'set, '_ref> {
	Ignore,
	Error,
	Collect(&'_ref mut HashSet<&'set str>),
}
pub fn unkown_feat<'src>(
	unkown: &mut UnknownFeat<'src, '_>, feat: &Ident<'src>, group_ident: &Ident<'_>,
	cur: &mut StrCursor<'src, '_>,
) -> bool {
	match unkown {
		UnknownFeat::Ignore => false,
		UnknownFeat::Error => {
			cur.error(
				format!("profile feature \"{group_ident}.{feat}\" is not supported"),
				feat.span,
			);
			true
		}
		UnknownFeat::Collect(set) => {
			set.insert(feat.val);
			false
		}
	}
}
pub fn parse_group<
	'src,
	G: Default + BitOrAssign + BitAnd<Output = G> + PartialEq + Copy,
>(
	cur: &mut StrCursor<'src, '_>, group_ident: &Ident<'_>, group: &mut Option<G>,
	features: &[(&str, G)], unknown: &mut UnknownFeat<'src, '_>,
) -> bool {
	if group.is_some() {
		cur.error(format!("duplicate profile group \"{group_ident}\""), group_ident.span);
	}
	let group = group.get_or_insert_default();
	let mut unsupported = false;
	if cur.try_char('{') {
		while !cur.test('}') && !cur.is_end() {
			let Some(feat) = cur.eat_ident() else {
				cur.skip();
				continue;
			};
			if let Some(&(name, feat_on)) = features.iter().find(|(n, _)| feat == n) {
				if *group & feat_on == feat_on {
					cur.error(
						format!("profile feature \"{group_ident}.{name}\" is duplicated"),
						feat.span,
					);
				}
				*group |= feat_on;
			} else {
				unsupported |= unkown_feat(unknown, &feat, group_ident, cur)
			}
			if !cur.try_char(',') && !cur.test('}') {
				cur.expected("`,`");
			}
		}
		_ = cur.eat('}');
	}
	unsupported
}

const BIN_FEATS: &[(&str, Bin)] = &[("seg", Bin::SEG)];
const DECL_FEATS: &[(&str, Decl)] = &[("gen", Decl::GEN)];
const RICH_FEATS: &[(&str, Rich)] = &[
	("inst", Rich::INST),
	("dur", Rich::DUR),
	("uuid", Rich::UUID),
	("bint", Rich::BINT),
	("dec", Rich::DEC),
	("color", Rich::COLOR),
	("bitfield", Rich::BITFIELD),
	("unitfield", Rich::UNITFIELD),
];

pub fn parse_profile<'src>(
	cur: &mut StrCursor<'src, '_>, unsupported_rich: Option<&mut HashSet<&'src str>>,
	emit_unsupported: bool,
) -> (Profile, bool) {
	let (mut obj, mut bin, mut decl, mut rich) = (None, None, None, None);
	let (mut unkown_normal, mut unknown_rich) = if emit_unsupported {
		(UnknownFeat::Error, UnknownFeat::Collect(unsupported_rich.unwrap()))
	} else {
		(UnknownFeat::Ignore, UnknownFeat::Ignore)
	};
	let mut unsupported = false;
	while !cur.is_end() {
		let Some(group) = cur.eat_ident() else {
			cur.skip();
			continue;
		};
		unsupported |= match group.val {
			"obj" => parse_group(cur, &group, &mut obj, &[], &mut unkown_normal),
			"bin" => parse_group(cur, &group, &mut bin, BIN_FEATS, &mut unkown_normal),
			"decl" => parse_group(cur, &group, &mut decl, DECL_FEATS, &mut unkown_normal),
			"rich" => parse_group(cur, &group, &mut rich, RICH_FEATS, &mut unknown_rich),
			name => {
				if emit_unsupported {
					cur.error(format!("invalid profile group \"{name}\""), group.span);
				}
				emit_unsupported
			}
		};
		if !cur.try_char('_') && !cur.is_end() {
			cur.expected("`_`");
		}
	}
	let profile =
		Profile::new().with_obj(obj).with_bin(bin).with_decl(decl).with_rich(rich);
	(profile, unsupported)
}

pub struct Scope<'src, 'prov> {
	pub prov: &'prov dyn DeclProv,

	pub local_file: Option<DeclFileBox>,

	pub namespaces: HashMap<&'src str, DeclFileBox>,
	pub unnamespaced: Vec<DeclFileBox>,
	pub top: HashMap<LeanStr, (DeclNs, ItemId, u16)>,

	pub unsuported_rich: HashSet<&'src str>,
	pub profile: Profile,
}
impl<'prov> Scope<'_, 'prov> {
	pub fn new(local_file: Option<DeclFileBox>, prov: &'prov dyn DeclProv) -> Self {
		Self {
			local_file,
			prov,
			unnamespaced: Vec::new(),
			namespaces: HashMap::default(),
			top: HashMap::default(),
			unsuported_rich: HashSet::new(),
			profile: Profile::new(),
		}
	}
	pub fn resolve_top(&mut self, name: &str) -> Option<(TypeId, u16)> {
		if let Some(&(ns, item, params)) = self.top.get(name) {
			return Some((TypeId::new_unchecked(ns, item), params));
		}
		self.unnamespaced.iter().find_map(|file| {
			file.item_by_name(name).and_then(|item| {
				let params = item.generic_params().len() as u16;
				self.top.insert(name.into(), (file.ns, item.id, params));
				Some((TypeId::new_unchecked(file.ns, item.id), params))
			})
		})
	}
	pub fn resolve_file(&self, ns: DeclNs) -> Option<&DeclFile> {
		if let Some(file) = &self.local_file
			&& file.ns == ns
		{
			Some(file)
		} else {
			self.prov.get_file(ns)
		}
	}
}

fn parse_head<'src>(
	cur: &mut Cursor<'src, '_>, scope: &mut Scope<'src, '_>,
) -> Result<(Metadata, Vec<Import<'src>>), ()> {
	let mut imports = Vec::new();
	let mut metadata = Metadata::new();
	while let Some((directive, span)) = meta_loop(cur, true, &mut metadata) {
		match directive {
			Directive::Import => {
				if let Some(import) = parse_import(cur) {
					imports.push(import);
				}
			}
			Directive::Profile => {
				if cur.eat_delim(paren_open, paren_close, |cur| {
					let (request, unsuported) = parse_profile(
						&mut cur.enter_str()?,
						Some(&mut scope.unsuported_rich),
						true,
					);
					let supported = Profile::supported().with_bin(None);
					scope.profile |= request;
					Some(unsuported | !supported.supports(request))
				}) == Some(true)
				{
					return Err(());
				}
			}
			_ => directive.invalid_place(cur, span),
		}
	}
	Ok((metadata, imports))
}

fn import_res_into_file(
	res: Result<DeclFileBox, ImportError>, path: &Str, cur: &mut Cursor,
) -> Result<DeclFileBox, ()> {
	match res {
		Ok(file) => Ok(file),
		Err(ImportError::NotFound) => {
			cur.error(format!("file \"{path}\" not found"), path.span);
			Err(())
		}
		Err(ImportError::Parse(errors)) => {
			cur.errors.extend(errors);
			Err(())
		}
		Err(ImportError::Other(msg)) => {
			cur.error(format!("while importing \"{path}\": {msg}"), path.span);
			Err(())
		}
	}
}
fn add_specified_imports(
	items: &[Ident], file: DeclFileBox, scope: &mut Scope, cur: &mut Cursor,
) {
	for item in items {
		let Some(def) = file.item_by_name(&item.val) else {
			cur.error(format!("importing undefined item \"{item}\""), item.span);
			return;
		};
		let Entry::Vacant(entry) = scope.top.entry(item.val.into()) else {
			cur.error(
				format!("multiple imported items with the same name \"{item}\"",),
				item.span,
			);
			return;
		};
		if scope.namespaces.contains_key(item.val) {
			cur.error(
				format!("imported item \"{item}\" conflict with a namespace"),
				item.span,
			)
		}
		entry.insert_entry((file.ns(), def.id, def.generic_params().len() as u16));
	}
}
fn add_import<'src>(
	res: Result<DeclFileBox, ImportError>, import: &Import<'src>,
	scope: &mut Scope<'src, '_>, cur: &mut Cursor,
) {
	let Import { path, ns, items, .. } = import;
	let Ok(file) = import_res_into_file(res, path, cur) else { return };

	if let Some(ns) = ns {
		let Entry::Vacant(entry) = scope.namespaces.entry(&ns.val) else {
			cur.error(
				format!("multiple imports into the same namespace \"{ns}\""),
				ns.span,
			);
			return;
		};
		if scope.top.contains_key(ns.val) {
			cur.error(
				format!("namescape \"{ns}\" conflict with an imported item"),
				ns.span,
			);
		}
		entry.insert_entry(file);
	} else if let Some(items) = items {
		add_specified_imports(items, file, scope, cur);
	} else {
		scope.unnamespaced.push(file)
	}
}

pub fn resolve_head<'src, 'prov>(
	cur: &mut Cursor<'src, '_>, scope: &mut Scope<'src, 'prov>,
) -> Result<Metadata, ()> {
	let (metadata, imports) = parse_head(cur, scope)?;
	for import in imports {
		match scope.prov.import_file(&import.path.val, import.meta.as_ref()) {
			ImportResult::Ok(file) => add_import(Ok(file), &import, scope, cur),
			ImportResult::Err(err) => add_import(Err(err), &import, scope, cur),
			ImportResult::Future(_) => cur.error(
				"async imports are not supported in non async parse functions".into(),
				Span::EMPTY,
			),
		};
	}
	Ok(metadata)
}

pub async fn resolve_head_async<'src, 'prov>(
	cur: &mut Cursor<'src, '_>, scope: &mut Scope<'src, 'prov>,
) -> Result<Metadata, ()> {
	let (metadata, imports) = parse_head(cur, scope)?;
	for import in imports {
		match scope.prov.import_file(&import.path.val, import.meta.as_ref()) {
			ImportResult::Ok(file) => add_import(Ok(file), &import, scope, cur),
			ImportResult::Err(err) => add_import(Err(err), &import, scope, cur),
			ImportResult::Future(future) => add_import(future.await, &import, scope, cur),
		};
	}
	Ok(metadata)
}
