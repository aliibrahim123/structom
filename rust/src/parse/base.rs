use alloc_crate::{borrow::Cow, format, string::ToString, vec::Vec};
use thin_vec::ThinVec;

use crate::{
	decl::rich::Metadata,
	parse::{
		Span,
		token_cursor::*,
		tokens::{Block, alt, bang, comma, hash, paren_close, paren_open},
		value::parse_value,
	},
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
		_ = metadata.add_lean(ident.val, items);
	}
}
fn try_directive(cur: &mut Cursor, in_root: bool) -> Option<(Directive, Span)> {
	cur.try_eat(hash).and_then(|_| {
		if in_root && !cur.try_symbol(bang) {
			cur.off -= 1;
			return None;
		}
		let Some(ident) = cur.ident() else { return None };
		let directive = match ident.val.as_str() {
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

#[derive(Debug, Default)]
pub struct Imports<'src>(Vec<(Str<'src>, Option<Ident>)>);

fn parse_import<'src>(cur: &mut Cursor<'src, '_>) -> Option<(Str<'src>, Option<Ident>)> {
	cur.eat_delim(paren_open, paren_close, |cur| {
		let path = cur.str_normal()?;
		cur.comma_unless_end();
		let ns = cur.try_ident_kw("as").then(|| cur.ident()).flatten();
		Some((path, ns))
	})
}

pub fn parse_head<'src>(cur: &mut Cursor<'src, '_>) -> (Imports<'src>, Metadata) {
	let mut metadata = Metadata::new();
	let mut imports = Imports::default();
	while let Some((directive, span)) = meta_loop(cur, true, &mut metadata) {
		match directive {
			Directive::Import => {
				if let Some((path, ns)) = parse_import(cur) {
					imports.0.push((path, ns));
				}
			}
			Directive::Profile => {}
			_ => cur.error(directive.invalid_place().to_string(), span),
		}
	}
	// imports
	// metadata
	// profile
	todo!()
}
