use core::ops::ControlFlow::{self, Break, Continue};

use alloc_crate::{format, vec::Vec};

use crate::{
	decl::{
		AssignError,
		items::{DeclFileBuilder, Item, ItemId},
		rich::Metadata,
	},
	parse::{
		Span,
		base::{Directive, MetaContext, meta_loop, try_directive},
		head::Scope,
		str_cursor::Ident,
		token_cursor::*,
		tokens::{
			Block, angle_close, angle_open, hash, kw_struct, kw_union, paren_close,
			paren_open,
		},
	},
};

#[derive(Debug, Default)]
struct Directives<'src> {
	tag: Option<(u16, Span)>,
	fixed: Option<Span>,
	default: Option<usize>,
	tree: Option<Ident<'src>>,
	rich: bool,
	bitfield: Option<usize>,
	unitfield: Option<usize>,
}

fn check_multiple_directive(cur: &mut Cursor, emit: bool, name: &str, span: Span) {
	if emit {
		let msg = format!("multiple {name} directives applied on the same item");
		cur.error(msg, span)
	}
}
fn arg_directive(cur: &mut Cursor, res: &mut Option<usize>, name: &str, span: Span) {
	check_multiple_directive(cur, res.is_some(), name, span);
	if cur.test(paren_open) {
		*res = Some(cur.off + 1);
		cur.skip_block(Block::Paren);
	} else {
		cur.expected("`(`");
	}
}
fn collect_directives<'src>(cur: &mut Cursor<'src, '_>) -> (Directives<'src>, Metadata) {
	let mut metadata = Metadata::new();
	let mut dirs = Directives::default();
	let mut meta_ctx = MetaContext::Normal;
	while let Some((directive, span)) = meta_loop(cur, meta_ctx, &mut metadata) {
		use Directive::*;
		match directive {
			Tag => {
				check_multiple_directive(cur, dirs.tag.is_some(), "tag", span);
				dirs.tag = cur.eat_int("item id").or(dirs.tag);
			}
			Fixed => {
				check_multiple_directive(cur, dirs.fixed.is_some(), "`fixed`", span);
				dirs.fixed = Some(span);
			}
			Default => arg_directive(cur, &mut dirs.default, "`default`", span),
			Tree => {
				check_multiple_directive(cur, dirs.tree.is_some(), "`tree`", span);
				dirs.tree = cur
					.eat_delim(paren_open, paren_close, |cur| cur.ident())
					.or(dirs.tree);
			}
			Rich => {
				check_multiple_directive(cur, dirs.rich, "`rich`", span);
				dirs.rich = true;
				meta_ctx = MetaContext::RichItem;
			}
			Bitfield => arg_directive(cur, &mut dirs.bitfield, "`bitfield`", span),
			Unitfield => arg_directive(cur, &mut dirs.unitfield, "`unitfield`", span),
			_ => directive.invalid_place(cur, span),
		}
	}
	(dirs, metadata)
}

fn report_directives(cur: &mut Cursor, start: usize, end: usize) {
	cur.rewind(start);
	while cur.off < end {
		cur.skip_until(|cur| cur.off >= end || cur.test(hash));
		if let Some((directive, span)) = try_directive(cur, MetaContext::Normal) {
			directive.invalid_place(cur, span);
		}
	}
}

#[derive(Debug)]
struct ItemBuild<'src> {
	id: ItemId,
	default: Option<usize>,
	tree: Option<Ident<'src>>,
	bitfield: Option<usize>,
	unitfield: Option<usize>,
}

fn parse_generic_params<'src>(cur: &mut Cursor<'src, '_>) -> Vec<&'src str> {
	cur.try_delim_list::<_, Vec<_>>(angle_open, angle_close, |cur| {
		cur.ident().map(|ident| ident.val)
	})
	.unwrap_or_default()
}

fn parse_struct_head<'src>(cur: &mut Cursor<'src, '_>) -> Option<Item> {
	todo!()
}

fn parse_item_head<'src>(
	cur: &mut Cursor<'src, '_>, scope: &mut Scope<'src, '_>, file: &mut DeclFileBuilder,
	items: &mut Vec<ItemBuild<'src>>,
) -> ControlFlow<Metadata> {
	let start = cur.off;
	let (directives, metadata) = collect_directives(cur);

	let (name, generic_params) = if cur.try_symbol(kw_struct) {
		let name = cur.ident().unwrap_or(Ident { val: "_", span: Span::EMPTY });
		let generic_params = parse_generic_params(cur);
		(name, generic_params)
	} else if cur.try_symbol(kw_union) {
		todo!()
	} else {
		report_directives(cur, start, cur.off);
		return Break(metadata);
	};
	let tag = directives.tag.map(|(tag, _)| tag);
	let id = match file.reserve(Some(name.val), tag, generic_params) {
		Ok(id) => id,
		Err(AssignError::DuplicateName) => {
			cur.error(format!("multiple items with the same name \"{name}\""), name.span);
			return Continue(());
		}
		Err(AssignError::UsedId) => {
			let tag = tag.unwrap();
			cur.error(
				format!("the specified id `{tag}` is used at \"{name}\""),
				directives.tag.unwrap().1,
			);
			return Continue(());
		}
		Err(AssignError::FullCapacity) => {
			cur.error("all ids has been used".into(), Span::EMPTY);
			return Continue(());
		}
		Err(_) => unreachable!(),
	};
	Continue(())
}

pub fn parse_items(cur: &mut Cursor, scope: &mut Scope) -> Option<Metadata> {
	todo!()
}
