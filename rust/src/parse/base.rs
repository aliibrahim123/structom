use core::ops::ControlFlow::{self, Break, Continue};

use alloc_crate::{format, string::ToString};

use thin_vec::ThinVec;

use crate::{
	decl::{
		DeclNs,
		items::{ItemId, TypeId, builtin_by_name},
		rich::Metadata,
	},
	parse::{
		Span,
		head::{Scope, UnresolvedKind},
		str_cursor::Ident,
		token_cursor::*,
		tokens::Block,
		value::{parse_map, parse_value},
	},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Directive {
	Import,
	Profile,
	Fixed,
	Default,
	Tree,
	Tag,
	Rich,
	Bitfield,
	Unitfield,
}
impl Directive {
	pub fn from_str(str: &str) -> Option<Self> {
		match str {
			"import" => Some(Self::Import),
			"profile" => Some(Self::Profile),
			"fixed" => Some(Self::Fixed),
			"default" => Some(Self::Default),
			"tree" => Some(Self::Tree),
			"rich" => Some(Self::Rich),
			"bitfield" => Some(Self::Bitfield),
			"unitfield" => Some(Self::Unitfield),
			_ => None,
		}
	}
	pub fn invalid_place(self, cur: &mut Cursor, span: Span) {
		let msg = match self {
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
			Directive::Tag => {
				"the tag directive can only be applied to items, variants and fields"
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
		};

		cur.error(msg.to_string(), span);
		if self == Directive::Tag {
			cur.try_eat(Int);
		} else {
			cur.skip_block(Block::Paren);
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetaContext {
	Root,
	Normal,
	RichItem,
}

#[inline]
fn try_metadata(cur: &mut Cursor, ctx: MetaContext, metadata: &mut Metadata) {
	loop {
		let start = cur.off;
		let skip = cur.try_symbol(UnitComment);
		if !cur.try_symbol(At) || ctx == MetaContext::Root && !cur.try_symbol(Bang) {
			cur.rewind(start);
			break;
		}
		if ctx != MetaContext::Root && cur.try_symbol(Bang) {
			cur.try_ident();
			cur.skip_block(Block::Paren);
			cur.error(
				"root level metadata outside root".into(),
				cur.tokens.0[start].span.join(cur.prev().span),
			);
		}
		let Some(ident) = cur.ident() else { continue };
		if skip || !cur.options.metadata {
			cur.skip_block(Block::Paren);
			continue;
		}
		let items: ThinVec<_> =
			cur.try_delim_list(Block::Paren, |cur, _| todo!()).unwrap_or_default();
		_ = metadata.add(ident.val, items);
	}
}
pub fn try_directive(cur: &mut Cursor, ctx: MetaContext) -> Option<(Directive, Span)> {
	loop {
		let start = cur.off;
		let skip = cur.try_symbol(UnitComment);
		let span = cur.peek().map_or(Span::EMPTY, |t| t.span);
		if !cur.try_symbol(Hash) || ctx == MetaContext::Root && !cur.try_symbol(Bang) {
			cur.rewind(start);
			return None;
		}
		if ctx != MetaContext::Root && cur.try_symbol(Bang) {
			_ = cur.try_ident().is_some() || cur.try_symbol(Int);
			cur.skip_block(Block::Paren);
			cur.error(
				"root level directive outside root".into(),
				cur.tokens.0[start].span.join(cur.prev().span),
			);
			continue;
		}
		if cur.test(Int) {
			if skip {
				cur.skip();
				continue;
			}
			return Some((Directive::Tag, span.join(cur.peek().unwrap().span)));
		}
		let Some(ident) = cur.ident() else { continue };
		let dir = Directive::from_str(ident.val);
		if dir.is_none() && ctx != MetaContext::RichItem {
			cur.error(format!("invalid directive \"{ident}\""), span.join(ident.span));
		}
		match dir {
			Some(d) if !skip => return Some((d, span.join(ident.span))),
			_ => cur.skip_block(Block::Paren),
		};
	}
}

pub fn meta_loop(
	cur: &mut Cursor, ctx: MetaContext, metadata: &mut Metadata,
) -> Option<(Directive, Span)> {
	loop {
		let start = cur.off;
		try_metadata(cur, ctx, metadata);
		if let Some(res) = try_directive(cur, ctx) {
			return Some(res);
		} else if cur.off == start {
			return None;
		}
	}
}

pub fn directiveless_meta(cur: &mut Cursor) -> Metadata {
	let mut metadata = Metadata::new();
	while let Some((directive, span)) = meta_loop(cur, MetaContext::Normal, &mut metadata)
	{
		directive.invalid_place(cur, span);
	}
	metadata
}

macro_rules! head_type {
	($csrc:lifetime, $psrc:lifetime) => { &mut impl FnMut(
		&mut Cursor<$csrc, '_>, &mut Scope<'psrc, '_>
	) -> ControlFlow<TypeId, Metadata> };
}

fn parse_arr_typespec<'csrc, 'psrc>(
	cur: &mut Cursor<'csrc, '_>, scope: &mut Scope<'psrc, '_>,
	head: head_type!('csrc, 'psrc),
) -> TypeId {
	let typeid = cur.delim(Block::Angle, |cur| {
		let item = parse_typespec(cur, scope, false, head).ok()?;
		cur.eat(Comma).ok()?;
		let nb = cur.int("`arr` length")?.0;
		Some(TypeId::arr(item, nb))
	});
	typeid.unwrap_or(TypeId::ANY)
}
fn resolve_primary<'csrc, 'psrc>(
	primary: &Ident, cur: &mut Cursor<'csrc, '_>, scope: &mut Scope<'psrc, '_>,
	head: head_type!('csrc, 'psrc),
) -> ControlFlow<TypeId, (TypeId, u16)> {
	if *primary == "arr" {
		return Break(parse_arr_typespec(cur, scope, head));
	} else if let Some(typeid) = builtin_by_name(primary.val, scope.profile) {
		let params = match typeid.item {
			ItemId::LIST | ItemId::FAR => 1,
			ItemId::MAP => 2,
			_ => 0,
		};
		Continue((typeid, params))
	} else if let Some(id) = scope.generics.iter().position(|name| primary == name) {
		Continue((TypeId::new_unchecked(DeclNs::GENERIC_PARAMS, ItemId(id as u16)), 0))
	} else if let Some(res) = scope.resolve_top(primary.val) {
		Continue(res)
	} else if scope.unresolved.contains_key(primary.val) {
		Break(TypeId::ANY)
	} else {
		cur.error(format!("undefined item \"{primary}\""), primary.span);
		return Break(TypeId::ANY);
	}
}
fn resolve_prim_dot_sec(
	primary: &Ident, cur: &mut Cursor, scope: &mut Scope, in_value: bool,
) -> ControlFlow<TypeId, (TypeId, u16)> {
	let Some(secondary) = cur.try_ident() else {
		return Break(TypeId::ANY);
	};
	if let Some(file) = scope.namespaces.get(primary.val) {
		let Some(item) = file.item_by_name(secondary.val) else {
			cur.error(
				format!("undefined item \"{primary}.{secondary}\""),
				primary.span.join(secondary.span),
			);
			return Break(TypeId::ANY);
		};
		Continue((
			TypeId::new_unchecked(file.ns, item.id),
			item.generic_params().len() as u16,
		))
	} else if in_value && let Some((typeid, _)) = scope.resolve_top(primary.val) {
		cur.off -= 2;
		return Break(typeid);
	} else {
		if !matches!(scope.unresolved.get(primary.val), Some(UnresolvedKind::NameSpace)) {
			cur.error(format!("undefined namespace \"{primary}\""), primary.span);
		}
		return Break(TypeId::ANY);
	}
}
fn parse_typepec_generic<'csrc, 'psrc>(
	params: u16, cur: &mut Cursor<'csrc, '_>, scope: &mut Scope<'psrc, '_>, start: Span,
	head: head_type!('csrc, 'psrc),
) -> ThinVec<TypeId> {
	let mut list: ThinVec<_> = cur
		.try_delim_list(Block::Angle, |cur, _| {
			parse_typespec(cur, scope, false, head).ok()
		})
		.unwrap_or_default();
	if list.len() != params as usize {
		cur.error(
			format!("expected {params} generic parameters, got {}", list.len()),
			start.join(cur.prev().span),
		);
		list.resize(params as usize, TypeId::ANY);
	}
	list
}

pub fn try_parse_typespec<'csrc, 'psrc>(
	cur: &mut Cursor<'csrc, '_>, scope: &mut Scope<'psrc, '_>, in_value: bool,
	head: head_type!('csrc, 'psrc),
) -> Result<TypeId, Metadata> {
	let metadata = match head(cur, scope) {
		Break(typeid) => return Ok(typeid),
		Continue(metadata) => metadata,
	};

	let Some(primary) = cur.try_ident() else {
		return Err(metadata);
	};

	let (mut typeid, params) = match match cur.try_symbol(Dot) {
		false => resolve_primary(&primary, cur, scope, head),
		true => resolve_prim_dot_sec(&primary, cur, scope, in_value),
	} {
		Continue(res) => res,
		Break(typeid) => {
			cur.skip_block(Block::Angle);
			return Ok(TypeId { metadata, ..typeid });
		}
	};

	typeid.metadata = metadata;
	typeid.params = parse_typepec_generic(params, cur, scope, primary.span, head);

	Ok(typeid)
}

pub fn parse_typespec<'csrc, 'psrc>(
	cur: &mut Cursor<'csrc, '_>, scope: &mut Scope<'psrc, '_>, in_value: bool,
	head: head_type!('csrc, 'psrc),
) -> Result<TypeId, Metadata> {
	try_parse_typespec(cur, scope, in_value, head)
		.inspect_err(|_| cur.expected("a type specifier"))
}
