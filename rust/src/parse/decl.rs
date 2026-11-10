use alloc_crate::vec::Vec;

use crate::{
	decl::rich::Metadata,
	parse::{
		base::{Directive, meta_loop},
		head::Scope,
		token_cursor::*,
		tokens::{Block, paren_open},
	},
};

pub fn parse_items(cur: &mut Cursor, scope: &mut Scope) -> Option<Metadata> {
	loop {
		let mut metadata = Metadata::new();
		let mut tag = None;
		let mut fixed = None;
		let mut default_value = None;
		let mut rich = None;
		let mut bitfield = None;
		let mut unitfield = None;
		let mut directive_spans = Vec::new();
		while let Some((directive, span)) = meta_loop(cur, false, &mut metadata) {
			match directive {
				Directive::Tag => {
					if tag.is_some() {
						cur.error("multiple tag directive on a single item".into(), span)
					}
					if let Some(nb) = cur.eat_int::<u16>("item id") {
						tag = Some(nb)
					}
				}
				Directive::Fixed => {
					if fixed.is_some() {
						cur.error(
							"multiple `fixed` directive on a single item".into(),
							span,
						)
					}
					fixed = Some(span)
				}
				Directive::Default => {
					if default_value.is_some() {
						cur.error(
							"multiple `default` directive on a single item".into(),
							span,
						)
					}
					if cur.test(paren_open) {
						default_value = Some(cur.off);
						cur.skip_block(Block::Paren);
					} else {
						cur.expected("`(`");
					}
				}
				Directive::Rich => {
					if rich.is_some() {
						cur.error(
							"multiple `rich` directive on a single item".into(),
							span,
						)
					}
					rich = Some(span)
				}
				Directive::Bitfield => {
					if bitfield.is_some() {
						cur.error(
							"multiple `bitfield` directive on a single item".into(),
							span,
						)
					}
					if cur.test(paren_open) {
						bitfield = Some(cur.off);
						cur.skip_block(Block::Paren);
					} else {
						cur.expected("`(`");
					}
				}
				Directive::Unitfield => {
					if unitfield.is_some() {
						cur.error(
							"multiple `unitfield` directive on a single item".into(),
							span,
						)
					}
					if cur.test(paren_open) {
						unitfield = Some(cur.off);
						cur.skip_block(Block::Paren);
					} else {
						cur.expected("`(`");
					}
				}
				_ => directive.invalid_place(cur, span),
			}
			directive_spans.push(span)
		}
	}
	todo!()
}
