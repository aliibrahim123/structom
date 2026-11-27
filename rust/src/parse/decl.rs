#[cfg(feature = "rich-bitfield")]
use core::ops::BitOrAssign;
use core::{
	default,
	fmt::Display,
	iter::repeat_n,
	num::NonZeroUsize,
	ops::ControlFlow::{self, Break, Continue},
};

use alloc_crate::{format, vec::Vec};
use gramex::Matcher;

use crate::{
	decl::{
		AssignError, DeclNs,
		items::{
			Body, DeclFileBuilder, Field, FieldId, Fields, Item, ItemId, TagType, TypeId,
			Union, Variant, VariantTag,
		},
		private::unit_def_typeid,
		provider::{LocalScope, Void},
		rich::{BitField, BitFieldType, Metadata, NotationError, Unit},
	},
	parse::{
		Span,
		base::{
			self, Directive, MetaContext, directiveless_meta, meta_loop, try_directive,
		},
		head::{self, Scope},
		str_cursor::Ident,
		token_cursor::*,
		tokens::Block,
		value::parse_value,
	},
	utils::{BoolExt, IntFromStr},
	value::Value,
};

struct Context<'src, 'other, 'prov> {
	file: &'other mut DeclFileBuilder,
	post_process: &'other mut Vec<PostProcess>,
	scope: &'other mut Scope<'src, 'prov>,
	cur_inner_def_id: &'other mut u16,
}

#[derive(Debug)]
struct ItemLoc {
	id: ItemId,
	kind: ItemKind,
	start: usize,
}

#[derive(Debug)]
struct VarPostProc {
	tag: VariantTag,
	directives: PostDirectives,
	fields_defaults: Vec<(FieldId, usize)>,
}
#[derive(Debug)]
enum PostProcKinded {
	None,
	Struct { fields_defaults: Vec<(FieldId, usize)> },
	Union { variants_procs: Vec<VarPostProc> },
}

#[derive(Debug)]
struct PostProcess {
	item_id: ItemId,
	directives: PostDirectives,
	kinded: PostProcKinded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ItemKind {
	Struct,
	Union,
}

#[derive(Debug, Default)]
struct PostDirectives {
	default: Option<(Span, NonZeroUsize)>,
	#[cfg(feature = "rich-bitfield")]
	bitfield: Option<(Span, NonZeroUsize)>,
	#[cfg(feature = "rich-unitfield")]
	unitfield: Option<(Span, NonZeroUsize)>,
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
struct Directives<'src, T: IntFromStr> {
	tag: Option<(T, Span)>,
	fixed: Option<Span>,
	tree: Option<(Ident<'src>, Span)>,
	rich: bool,
	post: PostDirectives,
}

fn skip_metadata(cur: &mut Cursor) {
	loop {
		let start = cur.off;
		let skip = cur.try_symbol(UnitComment);
		if !cur.try_symbol(At) {
			return;
		};
		cur.try_ident();
		cur.skip_block(Block::Paren);
	}
}
fn tag_only_meta<T: IntFromStr + Display>(cur: &mut Cursor) -> Option<(T, Span)> {
	let mut tag = None;
	loop {
		skip_metadata(cur);
		let start = cur.off;
		let skip = cur.try_symbol(UnitComment);
		if !cur.try_symbol(Hash) {
			cur.rewind(start);
			return tag;
		}
		if cur.test(Int) {
			tag = cur.try_int().filter(|_| !skip).or(tag)
		} else {
			cur.try_ident();
			cur.skip_block(Block::Paren);
		}
	}
}
fn check_multiple_directive(cur: &mut Cursor, emit: bool, name: &str, span: Span) {
	if emit {
		let msg = format!("multiple {name} directives applied on the same item");
		cur.error(msg, span)
	}
}
fn arg_directive(
	cur: &mut Cursor, res: &mut Option<(Span, NonZeroUsize)>, name: &str, span: Span,
) {
	check_multiple_directive(cur, res.is_some(), name, span);
	if cur.test(ParenOpen) {
		*res = Some((span, NonZeroUsize::new(cur.off).unwrap()));
		cur.skip_block(Block::Paren);
	} else {
		cur.expected("`(`");
	}
}
fn get_metadata_tag<T: IntFromStr + Display>(
	cur: &mut Cursor, kind: &str,
) -> (Metadata, Option<(T, Span)>) {
	let mut metadata = Metadata::new();
	let mut tag = None;
	while let Some((dir, span)) = meta_loop(cur, MetaContext::Normal, &mut metadata) {
		if dir == Directive::Tag {
			check_multiple_directive(cur, tag.is_some(), "tag", span);
			tag = cur.int(kind).or(tag);
		} else {
			dir.invalid_place(cur, span);
		}
	}
	(metadata, tag)
}
fn collect_directives<'src, T: IntFromStr + Display + Default>(
	cur: &mut Cursor<'src, '_>, tag_kind: &str,
) -> (Directives<'src, T>, Metadata) {
	let mut metadata = Metadata::new();
	let mut dirs = Directives::default();
	let mut meta_ctx = MetaContext::Normal;
	while let Some((directive, span)) = meta_loop(cur, meta_ctx, &mut metadata) {
		use Directive::*;
		match directive {
			Tag => {
				check_multiple_directive(cur, dirs.tag.is_some(), "tag", span);
				dirs.tag = cur.int(tag_kind).or(dirs.tag);
			}
			Fixed => {
				check_multiple_directive(cur, dirs.fixed.is_some(), "`fixed`", span);
				dirs.fixed = Some(span);
			}
			Default => arg_directive(cur, &mut dirs.post.default, "`default`", span),
			Tree => {
				check_multiple_directive(cur, dirs.tree.is_some(), "`tree`", span);
				dirs.tree = cur
					.delim(Block::Paren, |cur| cur.ident().map(|f| (f, span)))
					.or(dirs.tree);
			}
			Rich => {
				check_multiple_directive(cur, dirs.rich, "`rich`", span);
				dirs.rich = true;
				meta_ctx = MetaContext::RichItem;
			}
			Bitfield => {
				if cfg!(not(feature = "rich-bitfield")) {
					dirs.rich = true;
				}
				#[cfg(feature = "rich-bitfield")]
				arg_directive(cur, &mut dirs.post.bitfield, "`bitfield`", span);
			}
			Unitfield => {
				if cfg!(not(feature = "rich-unitfield")) {
					dirs.rich = true;
				}
				#[cfg(feature = "rich-unitfield")]
				arg_directive(cur, &mut dirs.post.unitfield, "`unitfield`", span);
			}
			_ => directive.invalid_place(cur, span),
		}
	}
	(dirs, metadata)
}
fn report_directives(cur: &mut Cursor) {
	while let Some((directive, span)) = {
		skip_metadata(cur);
		try_directive(cur, MetaContext::Normal)
	} {
		directive.invalid_place(cur, span);
	}
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

fn reserve_item(
	name: &Ident, id: Option<(u16, Span)>, generic_params: Vec<&str>, cur: &mut Cursor,
	ctx: &mut Context,
) -> Result<ItemId, ()> {
	let Context { file, scope, .. } = ctx;
	let generic_params_len = generic_params.len() as u16;
	let id = match file.reserve(Some(name.val), id.map(|(tag, _)| tag), generic_params) {
		Ok(id) => id,
		Err(err) => return Err(resport_assign_err(err, cur, "items", name, id, None)),
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
fn parse_generic_params<'src>(cur: &mut Cursor<'src, '_>) -> Vec<&'src str> {
	let mut generic_params = Vec::new();
	cur.try_delim_list::<(), ()>(Block::Angle, |cur, skip| {
		cur.ident().map(|param| {
			if !skip && generic_params.contains(&param.val) {
				cur.error(format!("duplicate generic parameter \"{param}\""), param.span);
			} else if !skip {
				generic_params.push(param.val);
			}
		})
	});
	generic_params
}
fn parse_item_kw(cur: &mut Cursor) -> ControlFlow<(), ItemKind> {
	if cur.try_symbol(KwStruct) {
		Continue(ItemKind::Struct)
	} else if cur.try_symbol(KwUnion) {
		Continue(ItemKind::Union)
	} else {
		return Break(());
	}
}
fn skip_item_body(cur: &mut Cursor, kind: ItemKind) {
	if kind == ItemKind::Struct {
		_ = cur.skip_block(Block::Paren) || cur.skip_block(Block::Brace);
	} else {
		cur.skip_block(Block::Paren);
		cur.skip_block(Block::Brace);
	}
}
fn discover_item(
	cur: &mut Cursor, locations: &mut Vec<ItemLoc>, ctx: &mut Context,
) -> ControlFlow<()> {
	let start = cur.off;
	let tag = tag_only_meta(cur);

	let skip = cur.try_symbol(UnitComment);

	let kind = parse_item_kw(cur)?;

	let name = cur.ident().unwrap_or_default();
	let generic_params = parse_generic_params(cur);

	skip_item_body(cur, kind);
	if skip {
		return Continue(());
	}

	if let Ok(id) = reserve_item(&name, tag, generic_params, cur, ctx) {
		locations.push(ItemLoc { id, start, kind });
	}
	Continue(())
}

fn reserve_inner_item(
	name: &Ident, id_dir: Option<(u16, Span)>, cur: &mut Cursor, ctx: &mut Context,
) -> Result<ItemId, ()> {
	let id = match id_dir {
		Some((id, _)) => id,
		None => loop {
			let cur_id = &mut *ctx.cur_inner_def_id;
			if ctx.file.item_by_id(ItemId(*cur_id)).is_none() {
				break *cur_id;
			}
			if *cur_id == 0 {
				cur.error("all items id had been used".into(), Span::EMPTY);
				return Err(());
			}
			*cur_id -= 1;
		},
	};
	ctx.file
		.reserve(Some(name.val), Some(id), ctx.scope.generics.iter().copied())
		.map_err(|err| resport_assign_err(err, cur, "items", name, id_dir, None))
}

fn parse_typespec<'src>(
	cur: &mut Cursor<'src, '_>, ctx: &mut Context<'src, '_, '_>,
) -> TypeId {
	let Context { file, post_process, scope, cur_inner_def_id } = ctx;
	base::parse_typespec(cur, scope, false, &mut |cur, scope| {
		let ctx = &mut Context { file, post_process, scope, cur_inner_def_id };
		let item = match parse_item(cur, ctx, |cur, id, ctx| {
			let kind = parse_item_kw(cur)?;
			let name = cur.try_ident().unwrap_or_default();
			let Ok(id) = reserve_inner_item(&name, id, cur, ctx) else {
				skip_item_body(cur, kind);
				return Break(());
			};
			Continue((kind, name, id))
		}) {
			Continue(item) => item,
			Break(metadata) => return Continue(metadata),
		};
		let id = item.id();
		_ = file.define_reserved(id, item);
		let params = scope.generics.iter().enumerate().map(|(ind, _)| {
			TypeId::new_unchecked(DeclNs::GENERIC_PARAMS, ItemId(ind as u16))
		});
		Break(TypeId::with_params_unchecked(file.ns(), id, params))
	})
	.unwrap_or(TypeId::ANY)
}

fn report_fielded_directives(
	cur: &mut Cursor, fixed: Option<Span>, tree: Option<(Ident, Span)>,
) {
	if let Some(span) = fixed {
		Directive::Fixed.invalid_place(cur, span);
	}
	if let Some((_, span)) = tree {
		Directive::Tree.invalid_place(cur, span);
	}
}

fn parse_field<'src>(
	cur: &mut Cursor<'src, '_>, fields: &mut Fields, parent: Option<&Ident>,
	fields_defaults: &mut Vec<(FieldId, usize)>, ctx: &mut Context<'src, '_, '_>,
) -> Option<()> {
	let (metadata, id) = get_metadata_tag(cur, "field id");

	let Some(name) = cur.ident() else { return None };
	let is_opt = cur.try_symbol(Question);
	_ = cur.eat(Colon);
	let typeid = parse_typespec(cur, ctx);

	let mut default_off = None;
	if cur.try_symbol(Equal) {
		default_off = Some(cur.off);
		cur.skip_until(|cur| cur.test(Comma) || cur.test(BraceClose));
	}

	let mut field = Field::new(name.val, typeid);
	if is_opt || default_off.is_some() {
		field.set_optional(None);
	}
	*field.metadata_mut() = metadata;

	match match id {
		None => fields.add_field(field),
		Some((id, _)) => fields.add_field_at(field, id),
	} {
		Ok(id) if let Some(default_off) = default_off => {
			fields_defaults.push((id, default_off))
		}
		Ok(_) => {}
		Err((_, err)) => resport_assign_err(err, cur, "fields", &name, id, parent),
	};
	Some(())
}

fn parse_fields<'src>(
	cur: &mut Cursor<'src, '_>, fixed: Option<Span>, tree: Option<(Ident, Span)>,
	ctx: &mut Context<'src, '_, '_>, parent: Option<&Ident>,
) -> (Body, Vec<(FieldId, usize)>) {
	let mut fields = Fields::new(fixed.is_some());
	let mut defaults = Vec::new();
	cur.try_delim_list::<(), ()>(Block::Brace, |cur, skip| {
		if skip {
			cur.skip_until(|cur| cur.test(Comma) || cur.test(BraceClose));
			Some(())
		} else {
			parse_field(cur, &mut fields, parent, &mut defaults, ctx)
		}
	});

	let at = || {
		if let Some(name) = parent { format!(" at {name}") } else { format!("") }
	};
	if let Some((children, _)) = tree {
		match fields.field_by_name(children.val) {
			Some(field) => fields.as_tree(field.id()).unwrap(),
			None => cur.error(
				format!("field \"{children}\" is not defined{}", at()),
				children.span,
			),
		}
	}
	if fields.finalize().is_err() {
		cur.error(
			format!("detected holes in field idspace{}", at()),
			parent.map_or(Span::EMPTY, |i| i.span),
		);
	}
	(Body::Fields(fields), defaults)
}

fn parse_body<'src>(
	cur: &mut Cursor<'src, '_>, fixed: Option<Span>, tree: Option<(Ident, Span)>,
	ctx: &mut Context<'src, '_, '_>, parent: Option<&Ident>,
) -> Option<(Body, Vec<(FieldId, usize)>)> {
	if let Some(inner) = cur.delim(Block::Paren, |cur| Some(parse_typespec(cur, ctx))) {
		report_fielded_directives(cur, fixed, tree);
		Some((Body::Tagged(inner), Vec::new()))
	} else if cur.test(BraceOpen) {
		Some(parse_fields(cur, fixed, tree, ctx, parent))
	} else {
		report_fielded_directives(cur, fixed, tree);
		None
	}
}

fn var_tag_out_range(cur: &mut Cursor, tag: Option<(u32, Span)>, tag_type: TagType) {
	let (tag, span) = tag.unwrap();
	let (ty, min, max) = match tag_type {
		TagType::U8 => ("`u8`", u8::MIN as u64, u8::MAX as u64),
		TagType::U16 => ("`u16`", u16::MIN as u64, u16::MAX as u64),
		_ => ("`u32`", u32::MIN as u64, u32::MAX as u64),
	};
	cur.error(
		format!(
			"number `{tag}` is out of rage for {ty} variant tag, valid range is `{min}`..`{max}`",
		),
		span,
	)
}

fn parse_variant<'src>(
	cur: &mut Cursor<'src, '_>, union: &mut Union, vars_procs: &mut Vec<VarPostProc>,
	ctx: &mut Context<'src, '_, '_>,
) -> Option<()> {
	let (Directives { tag, fixed, tree, rich, post }, metadata) =
		collect_directives(cur, "variant tag");

	let Some(name) = cur.ident() else { return None };
	let (body, fields_defualts) = parse_body(cur, fixed, tree, ctx, Some(&name)).unzip();

	let mut variant = Variant::new(name.val, body).unwrap();
	*variant.metadata_mut() = metadata;
	if rich {
		variant.rich_notation_mut().mark_true();
	}

	match match tag {
		None => union.add_variant(variant),
		Some((id, _)) => union.add_variant_at(variant, id),
	} {
		Ok(tag) => {
			let fields_defaults = fields_defualts.unwrap_or_default();
			if post.is_some() || !fields_defaults.is_empty() {
				vars_procs.push(VarPostProc { tag, directives: post, fields_defaults });
			}
		}
		Err((_, AssignError::IdOutOfRange)) => {
			var_tag_out_range(cur, tag, union.tag_type())
		}
		Err((_, err)) => {
			resport_assign_err(err, cur, "variants", &name, tag, Some(&name))
		}
	};
	Some(())
}

fn parse_union_body<'src>(
	cur: &mut Cursor<'src, '_>, ctx: &mut Context<'src, '_, '_>,
) -> (Union, Vec<VarPostProc>) {
	let tag_type = cur
		.try_delim(Block::Paren, |cur| match () {
			_ if cur.try_ident_kw("u32") => Some(TagType::U32),
			_ if cur.try_ident_kw("u16") => Some(TagType::U16),
			_ if cur.try_ident_kw("u8") => Some(TagType::U8),
			_ => {
				cur.expected(["`u8`, `u16`, `u32`"]);
				None
			}
		})
		.unwrap_or(TagType::Auto);

	let mut union = Union::new(tag_type);
	let mut vars_procs = Vec::new();
	cur.delim_list::<(), ()>(Block::Brace, |cur, skip| {
		if skip {
			cur.skip_until(|cur| cur.test(Comma) || cur.test(BraceClose));
			Some(())
		} else {
			parse_variant(cur, &mut union, &mut vars_procs, ctx)
		}
	});
	(union, vars_procs)
}

fn parse_item<'src>(
	cur: &mut Cursor<'src, '_>, ctx: &mut Context<'src, '_, '_>,
	mut head: impl FnMut(
		&mut Cursor<'src, '_>,
		Option<(u16, Span)>,
		&mut Context<'src, '_, '_>,
	) -> ControlFlow<(), (ItemKind, Ident<'src>, ItemId)>,
) -> ControlFlow<Metadata, Item> {
	let start = cur.off;
	let (Directives { tag: id, fixed, tree, rich, post, .. }, metadata) =
		collect_directives(cur, "item");

	let Continue((kind, name, id)) = head(cur, id, ctx) else {
		cur.rewind(start);
		report_directives(cur);
		return Break(metadata);
	};

	let mut item = if kind == ItemKind::Struct {
		let (body, fields_defaults) = parse_body(cur, fixed, tree, ctx, Some(&name))
			.unwrap_or_else(|| {
				cur.expected(["`{`", "`(`"]);
				(Body::Fields(Fields::new(false)), Vec::new())
			});

		if post.is_some() || !fields_defaults.is_empty() {
			let kinded = PostProcKinded::Struct { fields_defaults };
			ctx.post_process.push(PostProcess { item_id: id, directives: post, kinded });
		}

		Item::struct_(name.val, body).unwrap()
	} else {
		report_fielded_directives(cur, fixed, tree);
		let (union, variants_procs) = parse_union_body(cur, ctx);
		if post.is_some() || !variants_procs.is_empty() {
			let kinded = PostProcKinded::Union { variants_procs };
			ctx.post_process.push(PostProcess { item_id: id, directives: post, kinded });
		}
		Item::union(name.val, union)
	};

	*item.metadata_mut() = metadata;
	if rich {
		item.rich_notation_mut().mark_true();
	}
	Continue(item)
}

fn parse_root_level_item<'src>(
	cur: &mut Cursor<'src, '_>, loc: ItemLoc, ctx: &mut Context<'src, '_, '_>,
) {
	let ItemLoc { id, start, kind } = loc;
	cur.off = start;
	let Continue(item) = parse_item(cur, ctx, |cur, _, ctx| {
		cur.skip();
		let name = cur.try_ident().unwrap_or_default();
		ctx.scope.generics = parse_generic_params(cur);
		Continue((kind, name, id))
	}) else {
		unreachable!()
	};
	ctx.scope.generics = Vec::new();
	ctx.file.define_reserved(id, item).unwrap();
}

fn parse_bitfield_field(
	cur: &mut Cursor, bitfield: &mut BitField, skip: bool, span: Span,
	file: &mut DeclFileBuilder, scope: &mut Scope,
) -> Option<()> {
	let Some(name) = cur.ident() else { return None };
	_ = cur.eat(Colon);
	let start = cur.peek().map_or(Span::EMPTY, |t| t.span);

	let type_ = if let Some((val, _)) = cur.try_int() {
		BitFieldType::Atomic(val)
	} else {
		let ty = base::try_parse_typespec(cur, scope, false, &mut |cur, _| {
			Continue(directiveless_meta(cur))
		});
		BitFieldType::Typed(ty.ok()?)
	};

	if !skip {
		match bitfield.add(name.val, type_, &LocalScope::new_build(&file, scope.prov)) {
			Ok(_) => {}
			Err(AssignError::DuplicateName) => cur.error(
				format!("multiple fields with the same name \"{name}\""),
				name.span,
			),
			Err(AssignError::FullCapacity) => {
				cur.error("all bitfield size has been used".into(), span)
			}
			Err(AssignError::InvalidType) => {
				let typespec_span = start.join(cur.prev().span);

				let ty = cur.src.source_of(typespec_span);
				cur.error(format!("type \"{ty}\" is not a bitfield"), typespec_span)
			}
			_ => unreachable!(),
		}
	}

	Some(())
}

fn resolve_bitfield(
	cur: &mut Cursor, item_id: ItemId, body: NonZeroUsize, file: &mut DeclFileBuilder,
	scope: &mut Scope, span: Span,
) {
	cur.off = body.into();
	let Some((bitfield, has_size, size)) = cur.delim(Block::Paren, |cur| {
		let has_size = cur.test(Int);
		let size = has_size
			.and_then(|| cur.int("bitfield size").map(|(val, _)| val))
			.unwrap_or_else(|| {
				file.item_by_id(item_id).unwrap().rich_notation().caps.bitfield_size
			});
		if has_size {
			cur.comma_unless_end();
		}

		let mut bitfield = BitField::new(size);
		cur.try_delim_list::<(), ()>(Block::Brace, |cur, skip| {
			parse_bitfield_field(cur, &mut bitfield, skip, span, file, scope)
		});

		Some((bitfield, has_size, size))
	}) else {
		unreachable!()
	};

	let mut item = file.soft_mut_item(item_id).unwrap();
	match item.rich_notation_mut().add_bitfield(bitfield, !has_size) {
		Ok(_) => {}
		Err((_, NotationError::Disallowed)) => {
			Directive::Bitfield.invalid_place(cur, span)
		}
		Err((bitfield, NotationError::Invalid)) if bitfield.field_count() == 0 => {
			cur.error(
				"the `bitfield` directive can not have fields on unions".into(),
				span,
			);
		}
		Err((_, NotationError::Invalid)) => {
			let word_type = (*item).as_struct().unwrap();
			let word_type = word_type.as_tagged().unwrap();
			let word_type = word_type.display(&Void);
			cur.error(format!("\"{word_type}\" can not fit {size} bitfield"), span)
		}
		_ => unreachable!(),
	}
}

fn resolve_unitfield(
	cur: &mut Cursor, item_id: ItemId, body: NonZeroUsize, file: &mut DeclFileBuilder,
	scope: &mut Scope, span: Span,
) {
	let mut item = file.soft_mut_item(item_id).unwrap();
	let Ok(unitfield) = item.rich_notation_mut().add_unitfield() else {
		Directive::Unitfield.invalid_place(cur, span);
		return;
	};

	cur.off = body.into();
	let typeid =
		TypeId::map(TypeId::list(TypeId::STR), unit_def_typeid(unitfield.unit_type()));
	let units = cur.delim(Block::Paren, |cur| parse_value(cur, &typeid, scope)).unwrap();
	let Some(units) = units.into_map() else { return };
	for (names, mut def) in units.into_iter() {
		let names_value = names.as_list().unwrap();
		let mut names = Vec::with_capacity(names_value.len());
		for name in names_value {
			let name = name.as_str().unwrap();
			if unitfield.unit_by_name(name).is_some() {
				cur.error(format!("multiple units with the same name \"{name}\""), span);
				continue;
			}
			if names.contains(&name) {
				cur.error(format!("duplicate unit name \"{name}\""), span);
				continue;
			}
			names.push(name)
		}

		let def = def.as_struct_mut().unwrap();
		let Some(mult) = def.take_field(FieldId(0)) else { continue };
		let min = def.take_field(FieldId(1));
		let max = def.take_field(FieldId(2));
		let unit = Unit::with_bounds(names, mult, min, max);
		_ = unitfield.add(unit);
	}
}

fn resolve_rich_notation(
	cur: &mut Cursor, process: &PostProcess, file: &mut DeclFileBuilder,
	scope: &mut Scope,
) {
	#[cfg(feature = "rich-bitfield")]
	if let Some((span, body)) = process.directives.bitfield {
		resolve_bitfield(cur, process.item_id, body, file, scope, span);
	}
	#[cfg(feature = "rich-unitfield")]
	if let Some((span, body)) = process.directives.unitfield {
		resolve_unitfield(cur, process.item_id, body, file, scope, span);
	}
}

macro_rules! resolve_field_default {
	($cur:ident, $res: expr, $scope:ident, $file:expr, $process:expr, $($body_access:tt)+) => {
		let (field_id, offset) = $res;
		$cur.off = offset;
		let typeid = $file
			.item_by_id($process.item_id)
			.unwrap()
			.$($body_access)+
			.as_fields()
			.unwrap()
			.field_by_id(field_id)
			.unwrap()
			.typeid();
		let default = parse_value($cur, typeid, $scope).unwrap_or_default();
		$file.soft_mut_item($process.item_id)
			.unwrap()
			.$($body_access)+
			.as_fields()
			.unwrap()
			.field_mut(field_id)
			.unwrap()
			.set_default_value(default);
		if $cur.test(Comma) || $cur.test(BraceClose) {
			$cur.expected(Comma.expected())
		}
	};
}

fn resolve_defaults(
	cur: &mut Cursor, process: &PostProcess, file: &mut DeclFileBuilder,
	scope: &mut Scope,
) {
	if let Some((_, body)) = process.directives.default {
		let params = file.item_by_id(process.item_id).unwrap().generic_params().len();
		let params = repeat_n(TypeId::ANY, params);
		let typeid = TypeId::with_params_unchecked(file.ns(), process.item_id, params);
		cur.off = body.into();
		let default = cur
			.delim(Block::Paren, |cur| parse_value(cur, &typeid, scope))
			.unwrap_or_default();
		file.soft_mut_item(process.item_id).unwrap().set_default_value(default);
	}
	#[rustfmt::skip]
	match &process.kinded {
		PostProcKinded::None => {}
		PostProcKinded::Struct { fields_defaults } => {
			for &res in fields_defaults {
				resolve_field_default!(cur, res, scope,file, process, as_struct().unwrap());
			}
		}
		PostProcKinded::Union { variants_procs } => {
			for variant_proc in variants_procs {
				for &res in & variant_proc.fields_defaults {
					resolve_field_default!(cur, res, scope,file, process,
						as_union().unwrap().variant_by_tag(variant_proc.tag).unwrap().body().unwrap()
					);
				}
			}
		},
	};
}

pub fn parse_decl<'src>(
	cur: &mut Cursor<'src, '_>, path: &str, scope: &mut Scope<'src, '_>,
) -> DeclFileBuilder {
	let mut file = DeclFileBuilder::new(path);
	let mut locations = Vec::new();
	let mut post_process = Vec::new();
	let mut ctx = Context {
		file: &mut file,
		post_process: &mut post_process,
		scope,
		cur_inner_def_id: &mut u16::MAX,
	};
	while discover_item(cur, &mut locations, &mut ctx).is_continue() {}

	for loc in locations {
		parse_root_level_item(cur, loc, &mut ctx);
	}
	let end = cur.off;

	#[cfg(any(feature = "rich-bitfield", feature = "rich-unitfield"))]
	for process in &post_process {
		resolve_rich_notation(cur, process, &mut file, scope)
	}

	for process in &post_process {
		resolve_defaults(cur, process, &mut file, scope)
	}

	cur.rewind(end);
	file
}
