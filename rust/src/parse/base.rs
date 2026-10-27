use core::ops::{BitAnd, BitOr, BitOrAssign};

use alloc_crate::{borrow::Cow, format, string::ToString, vec::Vec};
use hashbrown::HashSet;
use thin_vec::ThinVec;

use crate::{
	decl::rich::Metadata,
	parse::{
		Span,
		str_cursor::{Cursor as StrCursor, Ident},
		token_cursor::*,
		tokens::{Block, alt, bang, comma, hash, paren_close, paren_open},
		value::parse_value,
	},
	profile::{Bin, Decl, Profile, Rich},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Directive {
	Import,
	Profile,
	Fixed,
	Default,
	Tree,
	Rich,
	Bitfield,
	Unitfield,
}
impl Directive {
	fn invalid_place(self) -> &'static str {
		match self {
			Directive::Import => {
				"the \"import\" directive can only be applied at the root level"
			}
			Directive::Profile => {
				"the \"profile\" directive can only be applied at the root level"
			}
			Directive::Fixed => {
				"the \"fixed\" directive can only be applied to a fielded structs or variants"
			}
			Directive::Default => {
				"the \"default\" directive can only be applied to items"
			}
			Directive::Tree => "the \"tree\" directive can only be applied to fields",
			Directive::Rich => {
				"the \"rich\" directive can only be applied to items and variants"
			}
			Directive::Bitfield => {
				"the \"bitfield\" directive can only be applied to tagged structs, variants and unit only unions"
			}
			Directive::Unitfield => {
				"the \"unitfield\" directive can only be applied to tagged structs and variants"
			}
		}
	}
}

fn try_metadata(cur: &mut Cursor, in_root: bool, metadata: &mut Metadata) {
	while cur.try_eat(alt).is_some() {
		if in_root && !cur.try_symbol(bang) {
			cur.off -= 1;
			break;
		}
		let Some(ident) = cur.try_ident() else { continue };
		if !cur.options.metadata {
			cur.skip_block(Block::Paren);
			continue;
		}
		let items: ThinVec<_> =
			cur.try_delim_list(paren_open, paren_open, |cur| parse_value(cur, true));
		_ = metadata.add(ident.val, items);
	}
}
fn try_directive(cur: &mut Cursor, in_root: bool) -> Option<(Directive, Span)> {
	cur.try_eat(hash).and_then(|_| {
		if in_root && !cur.try_symbol(bang) {
			cur.off -= 1;
			return None;
		}
		let Some(ident) = cur.ident() else { return None };
		let directive = match ident.val {
			"import" => Directive::Import,
			"profile" => Directive::Profile,
			"fixed" => Directive::Fixed,
			"default" => Directive::Default,
			"tree" => Directive::Tree,
			"rich" => Directive::Rich,
			"bitfield" => Directive::Bitfield,
			"unitfield" => Directive::Unitfield,
			name => {
				cur.error(format!("invalid directive \"{name}\""), ident.span);
				return None;
			}
		};
		Some((directive, ident.span))
	})
}

pub fn meta_loop(
	cur: &mut Cursor, in_root: bool, metadata: &mut Metadata,
) -> Option<(Directive, Span)> {
	try_metadata(cur, in_root, metadata);
	try_directive(cur, in_root)
}

fn parse_import<'src>(
	cur: &mut Cursor<'src, '_>,
) -> Option<(Str<'src>, Option<Ident<'src>>)> {
	cur.eat_delim(paren_open, paren_close, |cur| {
		let path = cur.str()?;
		cur.comma_unless_end();
		let ns = cur.try_ident_kw("as").then(|| cur.ident()).flatten();
		Some((path, ns))
	})
}

#[derive(Debug, Default)]
pub struct Head<'src> {
	imports: Vec<(Str<'src>, Option<Ident<'src>>)>,
	unsupported_rich: HashSet<&'src str>,
	unsuported: bool,
	metadata: Metadata,
}

pub fn parse_head<'src>(cur: &mut Cursor<'src, '_>) -> Head<'src> {
	let mut head = Head::default();
	while let Some((directive, span)) = meta_loop(cur, true, &mut head.metadata) {
		match directive {
			Directive::Import => {
				if let Some((path, ns)) = parse_import(cur) {
					head.imports.push((path, ns));
				}
			}
			Directive::Profile => {
				cur.eat_delim::<()>(paren_open, paren_close, |cur| {
					let (request, unsuported) =
						parse_profile(&mut cur.enter_str()?, &mut head.unsupported_rich);
					head.unsuported |=
						unsuported | Profile::supported().supports(request);
					None
				});
			}
			_ => cur.error(directive.invalid_place().to_string(), span),
		}
	}
	head
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
	cur: &mut StrCursor<'src, '_>, unsupported_rich: &mut HashSet<&'src str>,
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
			"obj" => parse_profile_group(cur, &group, &mut obj, &[], |_| true),
			"bin" => parse_profile_group(cur, &group, &mut bin, BIN_FEATS, |_| true),
			"decl" => parse_profile_group(cur, &group, &mut decl, DECL_FEATS, |_| true),
			"rich" => parse_profile_group(cur, &group, &mut rich, RICH_FEATS, |feat| {
				unsupported_rich.insert(feat.val);
				false
			}),
			name => {
				cur.error(format!("invalid profile group \"{name}\""), group.span);
				true
			}
		}
	}
	let mut profile = Profile::new();
	profile.set_obj(obj);
	profile.set_bin(bin);
	profile.set_decl(decl);
	profile.set_rich(rich);
	(profile, unsupported)
}
