use core::ops::{BitAnd, BitOrAssign};

use alloc_crate::{format, string::ToString, vec::Vec};
use hashbrown::{HashSet, hash_map::Entry};
use lean_string::LeanStr;

use crate::{
	decl::{
		DeclFile, DeclFileBox, DeclNs, DeclProv,
		items::ItemId,
		provider::{ImportError, ImportResult},
		rich::Metadata,
	},
	parse::{
		Span,
		base::{Directive, meta_loop},
		str_cursor::{Cursor as StrCursor, Ident},
		token_cursor::*,
		tokens::{
			Block, brace_close, brace_open, bracket_close, bracket_open, paren_close,
			paren_open,
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

pub fn parse_profile_group<
	'src,
	G: Default + BitOrAssign + BitAnd<Output = G> + PartialEq + Copy,
>(
	cur: &mut StrCursor<'src, '_>, group_ident: &Ident<'_>, group: &mut Option<G>,
	features: &[(&str, G)], mut is_unsupported: impl FnMut(&Ident<'src>) -> bool,
) -> bool {
	if group.is_none() {
		cur.error(format!("duplicate profile group \"{group_ident}\""), group_ident.span);
	}
	let group = group.get_or_insert_default();
	let mut unsupported = false;
	if cur.try_char('{') {
		while !cur.test('}') || cur.is_end() {
			let Some(feat) = cur.eat_ident() else {
				cur.skip();
				continue;
			};
			if let Some((name, feat_on)) = features.iter().find(|(n, _)| feat == n) {
				if *group & *feat_on == *feat_on {
					cur.error(
						format!("profile feature \"{group_ident}.{name}\" is duplicated"),
						feat.span,
					);
				}
				*group |= *feat_on
			} else if is_unsupported(&feat) {
				unsupported = true;
				cur.error(
					format!("profile feature \"{group_ident}.{feat}\" is not supported"),
					feat.span,
				);
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
	cur: &mut StrCursor<'src, '_>, mut unsupported_rich: Option<&mut HashSet<&'src str>>,
	emit_unsupported: bool,
) -> (Profile, bool) {
	let mut obj = None;
	let mut bin = None;
	let mut decl = None;
	let mut rich = None;
	let mut unsupported = false;
	while !cur.is_end() {
		let Some(group) = cur.eat_ident() else {
			cur.skip();
			continue;
		};
		unsupported |= match group.val {
			"obj" => {
				parse_profile_group(cur, &group, &mut obj, &[], |_| emit_unsupported)
			}
			"bin" => parse_profile_group(cur, &group, &mut bin, BIN_FEATS, |_| {
				emit_unsupported
			}),
			"decl" => parse_profile_group(cur, &group, &mut decl, DECL_FEATS, |_| {
				emit_unsupported
			}),
			"rich" => parse_profile_group(cur, &group, &mut rich, RICH_FEATS, |feat| {
				if let Some(unsupported_rich) = &mut unsupported_rich {
					unsupported_rich.insert(feat.val);
				}
				false
			}),
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
	pub top: HashMap<LeanStr, (DeclNs, ItemId)>,
	pub unsuported_rich: HashSet<&'src str>,
}
impl<'prov> Scope<'_, 'prov> {
	pub fn new(local_file: Option<DeclFileBox>, prov: &'prov dyn DeclProv) -> Self {
		Self {
			local_file,
			prov,
			namespaces: HashMap::default(),
			top: HashMap::default(),
			unsuported_rich: HashSet::new(),
		}
	}
}

fn parse_head<'src>(
	cur: &mut Cursor<'src, '_>, unsupported_rich: &mut HashSet<&'src str>,
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
					let cur = &mut cur.enter_str()?;
					let (request, unsuported) =
						parse_profile(cur, Some(unsupported_rich), true);
					let supported = Profile::supported().with_bin(None);
					Some(unsuported | !supported.supports(request))
				}) == Some(true)
				{
					return Err(());
				}
			}
			_ => {
				cur.error(directive.invalid_place().to_string(), span);
				cur.skip_block(Block::Paren);
			}
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

fn add_import<'src>(
	res: Result<DeclFileBox, ImportError>, import: &Import<'src>,
	scope: &mut Scope<'src, '_>, cur: &mut Cursor<'src, '_>,
) {
	let Import { path, ns, items, .. } = import;
	let Ok(file) = import_res_into_file(res, path, cur) else { return };

	let mut insert_top =
		|ident: LeanStr, item_id, span| match scope.top.entry(ident.clone()) {
			Entry::Occupied(entry) => cur.error(
				format!("multiple imported items with the same name \"{}\"", entry.key()),
				span,
			),
			Entry::Vacant(entry) => {
				entry.insert_entry((file.ns(), item_id));
			}
		};

	if let Some(ns) = ns {
		let Entry::Vacant(entry) = scope.namespaces.entry(&ns.val) else {
			cur.error(
				format!("multiple imports into the same namespace \"{ns}\""),
				ns.span,
			);
			return;
		};
		entry.insert_entry(file);
	} else if let Some(items) = items {
		for item in items {
			let Some(def) = file.item_by_name(&item.val) else {
				cur.error(format!("importing undefined item \"{item}\""), item.span);
				return;
			};
			insert_top(item.val.into(), def.id(), item.span);
		}
	} else {
		for item in file.items() {
			insert_top(item.name.clone(), item.id(), path.span);
		}
	}
}

pub fn resolve_head<'src, 'prov>(
	cur: &mut Cursor<'src, '_>, scope: &mut Scope<'src, 'prov>,
) -> Result<Metadata, ()> {
	let (metadata, imports) = parse_head(cur, &mut scope.unsuported_rich)?;
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
	let (metadata, imports) = parse_head(cur, &mut scope.unsuported_rich)?;
	for import in imports {
		match scope.prov.import_file(&import.path.val, import.meta.as_ref()) {
			ImportResult::Ok(file) => add_import(Ok(file), &import, scope, cur),
			ImportResult::Err(err) => add_import(Err(err), &import, scope, cur),
			ImportResult::Future(future) => add_import(future.await, &import, scope, cur),
		};
	}
	Ok(metadata)
}
