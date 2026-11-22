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
			Body, DeclFileBuilder, Field, FieldId, Fields, Item, ItemId, TagType, TypeId,
			Union, Variant, VariantTag,
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
	utils::IntFromStr,
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
struct Directives<'src, T: IntFromStr> {
	tag: Option<(T, Span)>,
	fixed: Option<Span>,
	tree: Option<(Ident<'src>, Span)>,
	rich: bool,
	post: PostDirectives,
}

fn tag_only_meta<T: IntFromStr + Display>(
	cur: &mut Cursor, kind: &str,
) -> Option<(T, Span)> {
	let mut tag = None;
	loop {
		let skip = cur.try_symbol(UnitComment);
		if cur.try_symbol(At) {
			cur.try_ident();
			cur.skip_block(Block::Paren);
			continue;
		}
		if !cur.try_symbol(Hash) {
			return tag;
		}
		if cur.test(Int) {
			tag = cur.int(kind).filter(|_| !skip).or(tag)
		} else {
			cur.try_ident();
			cur.skip_block(Block::Paren);
		}
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
struct ItemLoc {
	id: ItemId,
	kind: ItemKind,
	start: usize,
}

#[derive(Debug)]
struct VarPostProcess {
	tag: VariantTag,
	directives: PostDirectives,
	fields_defaults: Vec<(FieldId, usize)>,
}
#[derive(Debug)]
enum PostProcKinded {
	None,
	Struct { fields_defaults: Vec<(FieldId, usize)> },
	Union { variants_procs: Vec<VarPostProcess> },
}

#[derive(Debug)]
struct PostProcess {
	id: ItemId,
	directives: PostDirectives,
	kinded: PostProcKinded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ItemKind {
	Struct,
	Union,
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
	name: &Ident, tag: Option<(u16, Span)>, generic_params: Vec<&str>,
	file: &mut DeclFileBuilder, cur: &mut Cursor, scope: &mut Scope,
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
fn discover_item(
	cur: &mut Cursor, file: &mut DeclFileBuilder, locations: &mut Vec<ItemLoc>,
	scope: &mut Scope,
) -> ControlFlow<()> {
	let start = cur.off;
	let tag = tag_only_meta(cur, "item id");

	let kind = if cur.try_symbol(KwStruct) {
		ItemKind::Struct
	} else if cur.try_symbol(KwUnion) {
		ItemKind::Union
	} else {
		return Break(());
	};

	let name = cur.ident().unwrap_or_default();
	let generic_params = cur
		.try_delim_list::<_, Vec<_>>(Block::Angle, |cur| cur.ident().map(|i| i.val))
		.unwrap_or_default();

	if kind == ItemKind::Struct {
		cur.skip_block(Block::Paren) || cur.skip_block(Block::Brace);
	} else {
		cur.skip_block(Block::Paren);
		cur.skip_block(Block::Brace);
	}

	if let Ok(id) = reserve_item(&name, tag, generic_params, file, cur, scope) {
		locations.push(ItemLoc { id, start, kind });
	}
	Continue(())
}

fn parse_typespec(cur: &mut Cursor, scope: &mut Scope) -> TypeId {
	base::parse_typespec(cur, scope, false, &mut |cur| Break(TypeId::ANY))
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

fn parse_field(
	cur: &mut Cursor, fields: &mut Fields, parent: Option<&Ident>,
	fields_defaults: &mut Vec<(FieldId, usize)>, scope: &mut Scope,
) -> Option<()> {
	let (metadata, id) = get_metadata_tag(cur, "field id");

	let Some(name) = cur.ident() else { return None };
	let is_opt = cur.try_symbol(Question);
	_ = cur.eat(Colon);
	let typeid = parse_typespec(cur, scope);

	let mut default_off = None;
	if cur.try_symbol(Equal) {
		default_off = Some(cur.off);
		cur.skip_until(|cur| cur.test(Comma));
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
		Ok(id) => {
			if let Some(default_off) = default_off {
				fields_defaults.push((id, default_off))
			}
		}
		Err((_, err)) => resport_assign_err(err, cur, "fields", &name, id, parent),
	};
	Some(())
}

fn parse_body(
	cur: &mut Cursor, fixed: Option<Span>, tree: Option<(Ident, Span)>,
	scope: &mut Scope, parent: Option<&Ident>,
) -> Option<(Body, Vec<(FieldId, usize)>)> {
	if let Some(fields) = cur.delim(Block::Paren, |cur| Some(parse_typespec(cur, scope)))
	{
		report_fielded_directives(cur, fixed, tree);
		Some((Body::Tagged(fields), Vec::new()))
	} else if cur.test(BraceOpen) {
		let mut fields = Fields::new(fixed.is_some());
		let mut fields_defaults = Vec::new();
		cur.try_delim_list::<(), ()>(Block::Brace, |cur| {
			parse_field(cur, &mut fields, parent, &mut fields_defaults, scope)
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

		Some((Body::Fields(fields), fields_defaults))
	} else {
		report_fielded_directives(cur, fixed, tree);
		None
	}
}

fn parse_union_body(cur: &mut Cursor, scope: &mut Scope) -> (Union, Vec<VarPostProcess>) {
	let tag_type = cur
		.try_delim(Block::Paren, |cur| {
			if cur.try_ident_kw("u32") {
				Some(TagType::U32)
			} else if cur.try_ident_kw("u16") {
				Some(TagType::U16)
			} else if cur.try_ident_kw("u8") {
				Some(TagType::U8)
			} else {
				cur.expected(["`u8`, `u16`, `u32`"]);
				None
			}
		})
		.unwrap_or(TagType::Auto);

	let mut union = Union::new(tag_type);
	let mut post_process = Vec::new();
	cur.delim_list::<(), ()>(Block::Brace, |cur| {
		let (Directives { tag, fixed, tree, rich, post }, metadata) =
			collect_directives(cur, "variant tag");
		let Some(name) = cur.ident() else { return None };
		let (body, fields_defualts) =
			parse_body(cur, fixed, tree, scope, Some(&name)).unzip();
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
					post_process.push(VarPostProcess {
						tag,
						directives: post,
						fields_defaults,
					});
				}
			}
			Err((_, AssignError::IdOutOfRange)) => {
				let (tag, span) = tag.unwrap();
				let (ty, min, max) = match tag_type {
					TagType::U8 => ("`u8`", u8::MIN as u64, u8::MAX as u64),
					TagType::U16 => ("`u16`", u16::MIN as u64, u16::MAX as u64),
					_ => ("`u32`", u32::MIN as u64, u32::MAX as u64),
				};
				cur.error(format!(
					"number `{tag}` is out of rage for {ty} variant tag, valid range is `{min}`..`{max}`"
				), span)
			}
			Err((_, err)) => {
				resport_assign_err(err, cur, "variants", &name, tag, Some(&name))
			}
		};
		Some(())
	});
	(union, post_process)
}

fn parse_item(
	cur: &mut Cursor, loc: ItemLoc, file: &mut DeclFileBuilder,
	post_process: &mut Vec<PostProcess>, scope: &mut Scope,
) {
	let ItemLoc { id, start, kind } = loc;
	cur.off = start;
	let (Directives { fixed, tree, rich, post, .. }, metadata) =
		collect_directives::<u64>(cur, "item");

	cur.skip();
	let name = cur.try_ident().unwrap_or_default();
	cur.skip_block(Block::Angle);

	let mut item = if kind == ItemKind::Struct {
		let (body, fields_defaults) = parse_body(cur, fixed, tree, scope, Some(&name))
			.unwrap_or_else(|| {
				cur.expected(["`{`", "`(`"]);
				(Body::Fields(Fields::new(false)), Vec::new())
			});

		if post.is_some() || !fields_defaults.is_empty() {
			let kinded = PostProcKinded::Struct { fields_defaults };
			post_process.push(PostProcess { id, directives: post, kinded });
		}

		Item::struct_("_", body).unwrap()
	} else {
		report_fielded_directives(cur, fixed, tree);
		let (union, variants_procs) = parse_union_body(cur, scope);
		if post.is_some() && !variants_procs.is_empty() {
			let kinded = PostProcKinded::Union { variants_procs };
			post_process.push(PostProcess { id, directives: post, kinded });
		}
		Item::union("_", union)
	};

	*item.metadata_mut() = metadata;
	if rich {
		item.rich_notation_mut().mark_true();
	}
	file.define_reserved(id, item).unwrap();
}

pub fn parse_decl<'src>(
	cur: &mut Cursor<'src, '_>, path: &str, scope: &mut Scope<'src, '_>,
) -> DeclFileBuilder {
	let mut file = DeclFileBuilder::new(path);
	let mut locations = Vec::new();
	while discover_item(cur, &mut file, &mut locations, scope).is_continue() {}

	let mut post_process = Vec::new();
	for loc in locations {
		parse_item(cur, loc, &mut file, &mut post_process, scope);
	}
	file
}
