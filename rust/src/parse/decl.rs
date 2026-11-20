use core::{
	num::NonZeroUsize,
	ops::ControlFlow::{self, Break, Continue},
};

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
		tokens::Block,
	},
};

#[derive(Debug, Default)]
struct ValueDirectives {
	rich: bool,
	default: Option<NonZeroUsize>,
	#[cfg(feature = "rich-bitfield")]
	bitfield: Option<NonZeroUsize>,
	#[cfg(feature = "rich-unitfield")]
	unitfield: Option<NonZeroUsize>,
}

#[derive(Debug, Default)]
struct Directives<'src> {
	tag: Option<(u16, Span)>,
	fixed: Option<Span>,
	tree: Option<Ident<'src>>,
	value: ValueDirectives,
}

fn check_multiple_directive(cur: &mut Cursor, emit: bool, name: &str, span: Span) {
	if emit {
		let msg = format!("multiple {name} directives applied on the same item");
		cur.error(msg, span)
	}
}
fn arg_directive(
	cur: &mut Cursor, res: &mut Option<NonZeroUsize>, name: &str, span: Span,
) {
	check_multiple_directive(cur, res.is_some(), name, span);
	if cur.test(ParenOpen) {
		*res = Some(NonZeroUsize::new(cur.off + 1).unwrap());
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
			Default => arg_directive(cur, &mut dirs.value.default, "`default`", span),
			Tree => {
				check_multiple_directive(cur, dirs.tree.is_some(), "`tree`", span);
				dirs.tree = cur.eat_delim(Block::Paren, |cur| cur.ident()).or(dirs.tree);
			}
			Rich => {
				check_multiple_directive(cur, dirs.value.rich, "`rich`", span);
				dirs.value.rich = true;
				meta_ctx = MetaContext::RichItem;
			}
			Bitfield => {
				dirs.value.rich = true;
				arg_directive(cur, &mut dirs.value.bitfield, "`bitfield`", span);
			}
			Unitfield => {
				dirs.value.rich = true;
				arg_directive(cur, &mut dirs.value.unitfield, "`unitfield`", span);
			}
			_ => directive.invalid_place(cur, span),
		}
	}
	(dirs, metadata)
}
fn report_directives(cur: &mut Cursor, start: usize, end: usize) {
	cur.rewind(start);
	while cur.off < end {
		cur.skip_until(|cur| cur.off >= end || cur.test(Hash));
		if let Some((directive, span)) = try_directive(cur, MetaContext::Normal) {
			directive.invalid_place(cur, span);
		}
	}
}

#[derive(Debug)]
struct ItemBuild<'src> {
	id: ItemId,
	kind: ItemKind<'src>,
	body: usize,
	value_dirs: ValueDirectives,
}

#[derive(Debug)]
enum ItemKind<'src> {
	Struct { fixed: bool, tree: Option<Ident<'src>> },
	Union,
}

fn parse_item_head<'src>(
	cur: &mut Cursor<'src, '_>, scope: &mut Scope<'src, '_>, file: &mut DeclFileBuilder,
	items: &mut Vec<ItemBuild<'src>>,
) -> ControlFlow<Metadata> {
	let start = cur.off;
	let (Directives { tag, fixed, tree, value }, metadata) = collect_directives(cur);

	let is_struct = if cur.try_symbol(KwStruct) {
		true
	} else if cur.try_symbol(KwUnion) {
		if let Some(span) = fixed {
			Directive::Fixed.invalid_place(cur, span);
		}
		false
	} else {
		report_directives(cur, start, cur.off);
		return Break(metadata);
	};

	let name = cur.ident().unwrap_or(Ident { val: "_", span: Span::EMPTY });
	let generic_params = cur
		.try_delim_list::<_, Vec<_>>(Block::Angle, |cur| {
			cur.ident().map(|ident| ident.val)
		})
		.unwrap_or_default();

	let body = cur.off;
	let kind = if is_struct {
		let is_tagged = cur.skip_block(Block::Paren);
		if is_tagged && let Some(span) = fixed {
			Directive::Fixed.invalid_place(cur, span);
		}
		if !is_tagged && !cur.skip_block(Block::Brace) {
			cur.expected(["{", "("]);
			return Continue(());
		}
		ItemKind::Struct { fixed: fixed.is_some(), tree }
	} else {
		cur.skip_block(Block::Paren);
		if !cur.skip_block(Block::Brace) {
			cur.expected("{");
			return Continue(());
		}
		ItemKind::Union
	};

	let generic_params_len = generic_params.len() as u16;
	let id = match file.reserve(Some(name.val), tag.map(|(tag, _)| tag), generic_params) {
		Ok(id) => id,
		Err(AssignError::DuplicateName) => {
			cur.error(format!("multiple items with the same name \"{name}\""), name.span);
			return Continue(());
		}
		Err(AssignError::UsedId) => {
			let (tag, span) = tag.unwrap();
			cur.error(format!("the specified id `{tag}` is used at \"{name}\""), span);
			return Continue(());
		}
		Err(AssignError::FullCapacity) => {
			cur.error("all ids has been used".into(), Span::EMPTY);
			return Continue(());
		}
		Err(_) => unreachable!(),
	};

	if scope.top.insert(name.val.into(), (file.ns(), id, generic_params_len)).is_some() {
		cur.error(
			format!("local item \"{name}\" conflict with an imported item",),
			name.span,
		);
	};
	if scope.namespaces.contains_key(name.val) {
		cur.error(format!("local item \"{name}\" conflict with a namespace"), name.span);
	}

	items.push(ItemBuild { id, kind, body, value_dirs: value });
	Continue(())
}

pub fn parse_items(cur: &mut Cursor, scope: &mut Scope) -> Option<Metadata> {
	todo!()
}
