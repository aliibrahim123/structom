#[cfg(feature = "rich-bitfield")]
use core::ops::BitOrAssign;
use core::{
	fmt::Display,
	num::NonZeroUsize,
	ops::ControlFlow::{self, Break, Continue},
};

use alloc_crate::{format, vec::Vec};

use crate::{
	decl::{
		AssignError,
		items::{
			Body, DeclFileBuilder, Field, FieldId, Fields, Item, ItemId, TypeId,
			VariantTag,
		},
		rich::Metadata,
	},
	parse::{
		Span,
		base::{self, Directive, MetaContext, meta_loop, try_directive},
		head::Scope,
		str_cursor::Ident,
		token_cursor::*,
		tokens::Block,
	},
};

#[derive(Debug, Default)]
struct PostDirectives {
	default: Option<NonZeroUsize>,
	#[cfg(feature = "rich-bitfield")]
	bitfield: Option<NonZeroUsize>,
	#[cfg(feature = "rich-unitfield")]
	unitfield: Option<NonZeroUsize>,
}
impl PostDirectives {
	fn is_some(&self) -> bool {
		let mut res = self.default.is_some();
		#[cfg(feature = "rich-bitfield")]
		res.bitor_assign(self.bitfield.is_some());
		#[cfg(feature = "rich-unitfield")]
		res.bitor_assign(self.unitfield.is_some());
		res
	}
}

#[derive(Debug, Default)]
struct Directives<'src> {
	tag: Option<(u16, Span)>,
	fixed: Option<Span>,
	tree: Option<(Ident<'src>, Span)>,
	rich: bool,
	post: PostDirectives,
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
			Default => arg_directive(cur, &mut dirs.post.default, "`default`", span),
			Tree => {
				check_multiple_directive(cur, dirs.tree.is_some(), "`tree`", span);
				dirs.tree = cur
					.eat_delim(Block::Paren, |cur| cur.ident().map(|f| (f, span)))
					.or(dirs.tree);
			}
			Rich => {
				check_multiple_directive(cur, dirs.rich, "`rich`", span);
				dirs.rich = true;
				meta_ctx = MetaContext::RichItem;
			}
			Bitfield => {
				dirs.rich = true;
				#[cfg(feature = "rich-bitfield")]
				arg_directive(cur, &mut dirs.post.bitfield, "`bitfield`", span);
			}
			Unitfield => {
				dirs.rich = true;
				#[cfg(feature = "rich-unitfield")]
				arg_directive(cur, &mut dirs.post.unitfield, "`unitfield`", span);
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
	rich: bool,
	post: PostDirectives,
}

#[derive(Debug)]
struct VarPostProcess {
	tag: VariantTag,
	directives: PostDirectives,
	fields_defaults: Vec<(FieldId, usize)>,
}
#[derive(Debug)]
enum PostProcessKinded {
	Struct { fields_defaults: Vec<(FieldId, usize)> },
	Union { variants: Vec<VarPostProcess> },
}
impl PostProcessKinded {
	fn default_of(is_struct: bool) -> Self {
		match is_struct {
			true => PostProcessKinded::Struct { fields_defaults: Vec::new() },
			false => PostProcessKinded::Union { variants: Vec::new() },
		}
	}
}
#[derive(Debug)]
struct PostProcess {
	id: ItemId,
	directives: PostDirectives,
	kinded: PostProcessKinded,
}

#[derive(Debug)]
enum ItemKind<'src> {
	Struct { fixed: bool, tree: Option<Ident<'src>> },
	Union,
}

fn discover_item_specific<'src>(
	cur: &mut Cursor<'src, '_>, is_struct: bool, fixed: Option<Span>,
	tree: Option<(Ident<'src>, Span)>,
) -> Result<(ItemKind<'src>, usize), ()> {
	let fielded = is_struct && !cur.test(ParenOpen);
	if !fielded {
		if let Some(span) = fixed {
			Directive::Fixed.invalid_place(cur, span);
		}
		if let Some((_, span)) = tree {
			Directive::Tree.invalid_place(cur, span);
		}
	}

	let body = cur.off;
	let kind = if is_struct {
		if !cur.skip_block(Block::Paren) && !cur.skip_block(Block::Brace) {
			cur.expected(["{", "("]);
			return Err(());
		}
		ItemKind::Struct { fixed: fixed.is_some(), tree: tree.map(|f| f.0) }
	} else {
		cur.skip_block(Block::Paren);
		if !cur.skip_block(Block::Brace) {
			cur.expected("{");
			return Err(());
		}
		ItemKind::Union
	};

	Ok((kind, body))
}

fn resport_assign_err(
	err: AssignError, cur: &mut Cursor, kinds: &str, name: &Ident,
	tag: Option<(impl Display, Span)>, parent: Option<&Ident>,
) {
	match err {
		AssignError::DuplicateName => {
			cur.error(
				format!("multiple {kinds} with the same name \"{name}\""),
				name.span,
			);
		}
		AssignError::UsedId => {
			let (tag, span) = tag.unwrap();
			cur.error(format!("the specified id `{tag}` is used at \"{name}\""), span);
		}
		AssignError::FullCapacity => {
			let at = parent.map_or_default(|p| format!("at \"{p}\""));
			cur.error(
				format!("all {kinds} ids has been used {at}"),
				parent.map_or(Span::EMPTY, |ident| ident.span),
			);
		}
		_ => unreachable!(),
	}
}

fn reserve_item<'src>(
	name: &Ident<'src>, tag: Option<(u16, Span)>, generic_params: Vec<&str>,
	file: &mut DeclFileBuilder, cur: &mut Cursor<'src, '_>, scope: &mut Scope<'src, '_>,
) -> Result<ItemId, ()> {
	let generic_params_len = generic_params.len() as u16;
	let id = match file.reserve(Some(name.val), tag.map(|(tag, _)| tag), generic_params) {
		Ok(id) => id,
		Err(err) => return Err(resport_assign_err(err, cur, "items", name, tag, None)),
	};

	if scope.top.insert(name.val.into(), (file.ns(), id, generic_params_len)).is_some() {
		cur.error(
			format!("local item \"{name}\" conflict with an imported item"),
			name.span,
		);
	};
	if scope.namespaces.contains_key(name.val) {
		cur.error(format!("local item \"{name}\" conflict with a namespace"), name.span);
	}
	Ok(id)
}
fn discover_item<'src>(
	cur: &mut Cursor<'src, '_>, file: &mut DeclFileBuilder,
	items: &mut Vec<ItemBuild<'src>>, scope: &mut Scope<'src, '_>,
) -> ControlFlow<Metadata> {
	let start = cur.off;
	let (Directives { tag, fixed, rich, tree, post }, metadata) = collect_directives(cur);

	let is_struct = cur.try_symbol(KwStruct);
	if !is_struct && !cur.try_symbol(KwUnion) {
		report_directives(cur, start, cur.off);
		return Break(metadata);
	};

	let name = cur.ident().unwrap_or(Ident { val: "_", span: Span::EMPTY });
	let generic_params = cur
		.try_delim_list::<_, Vec<_>>(Block::Angle, |cur| cur.ident().map(|i| i.val))
		.unwrap_or_default();

	if let Ok((kind, body)) = discover_item_specific(cur, is_struct, fixed, tree)
		&& let Ok(id) = reserve_item(&name, tag, generic_params, file, cur, scope)
	{
		items.push(ItemBuild { id, kind, body, rich, post });
	}
	Continue(())
}

fn parse_typespec(cur: &mut Cursor, scope: &mut Scope) -> TypeId {
	base::parse_typespec(cur, scope, false, &mut |cur| Break(TypeId::ANY))
		.unwrap_or(TypeId::ANY)
}

fn parse_body(
	cur: &mut Cursor, fixed: bool, tree: Option<Ident>, scope: &mut Scope,
	parent: Option<&Ident>,
) -> Option<(Body, Option<Vec<(FieldId, usize)>>)> {
	if let Some(fields) =
		cur.eat_delim(Block::Paren, |cur| Some(parse_typespec(cur, scope)))
	{
		Some((Body::Tagged(fields), None))
	} else if cur.test(BraceOpen) {
		let mut fields = Fields::new(fixed);
		let mut fields_defaults = None::<Vec<_>>;
		cur.try_delim_list::<(), ()>(Block::Brace, |cur| {
			let mut metadata = Metadata::new();
			let mut id = None;
			while let Some((directive, span)) =
				meta_loop(cur, MetaContext::Normal, &mut metadata)
			{
				if directive == Directive::Tag {
					check_multiple_directive(cur, id.is_some(), "tag", span);
					id = cur.eat_int("item id").or(id);
				} else {
					directive.invalid_place(cur, span);
				}
			}
			let name = cur.ident().unwrap_or(Ident { val: "_", span: Span::EMPTY });
			cur.eat(Colon);
			let typeid = parse_typespec(cur, scope);
			let mut default_off = None;
			if cur.try_symbol(Equal) {
				default_off = Some(cur.off);
				cur.skip_until(|cur| cur.test(Comma));
			}
			let field = Field::new(name.val, typeid);
			let id = match match id {
				None => fields.add_field(field),
				Some((id, _)) => fields.add_field_at(field, id),
			} {
				Ok(id) if let Some(default_off) = default_off => {
					fields_defaults.get_or_insert_default().push((id, default_off))
				}
				Ok(_) => {}
				Err((_, err)) => {
					resport_assign_err(err, cur, "fields", &name, id, parent)
				}
			};
			Some(())
		});
		if let Some(children) = tree {
			match fields.field_by_name(children.val) {
				Some(field) => fields.as_tree(field.id()).unwrap(),
				None => cur
					.error(format!("field \"{children}\" is not defined"), children.span),
			}
		}
		Some((Body::Fields(fields), fields_defaults))
	} else {
		None
	}
}

fn parse_item_body(
	cur: &mut Cursor, build: ItemBuild, file: &mut DeclFileBuilder, scope: &mut Scope,
) {
	let item = match build.kind {
		ItemKind::Struct { fixed, tree } => parse_body(cur, fixed, tree, scope, None),
		ItemKind::Union => todo!(),
	};
}

pub fn parse_decl<'src>(
	cur: &mut Cursor<'src, '_>, path: &str, scope: &mut Scope<'src, '_>,
) -> (DeclFileBuilder, Metadata) {
	let mut file = DeclFileBuilder::new(path);
	let mut items = Vec::new();
	//let mut value_stage = Vec::new();
	let post_metadata = loop {
		match discover_item(cur, &mut file, &mut items, scope) {
			Continue(()) => (),
			Break(metadata) => break metadata,
		}
	};
	(file, post_metadata)
}
