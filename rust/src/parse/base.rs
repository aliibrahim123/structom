use alloc_crate::format;
use thin_vec::ThinVec;

use crate::{
	decl::rich::Metadata,
	parse::{
		Span,
		token_cursor::*,
		tokens::{Block, alt, bang, hash, paren_open},
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
		let Ok(ident) = cur.ident() else { return None };
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
