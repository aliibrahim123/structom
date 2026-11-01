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
		head::Scope,
		str_cursor::Ident,
		token_cursor::*,
		tokens::{
			Block, alt, angle_close, angle_open, bang, comma, dot, hash, int, paren_open,
		},
		value::parse_map,
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
			cur.try_eat(int);
		} else {
			cur.skip_block(Block::Paren)
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
		let items: ThinVec<_> = cur
			.try_delim_list(paren_open, paren_open, |cur| parse_map(cur, true))
			.unwrap_or_default();
		_ = metadata.add(ident.val, items);
	}
}
fn try_directive(cur: &mut Cursor, in_root: bool) -> Option<(Directive, Span)> {
	cur.try_eat(hash).and_then(|_| {
		if in_root && !cur.try_symbol(bang) {
			cur.off -= 1;
			return None;
		}
		if cur.test(int) {
			return Some((Directive::Tag, cur.peek().unwrap().span));
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
				cur.skip_block(Block::Paren);
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

pub fn directiveless_meta(cur: &mut Cursor) -> Metadata {
	let mut metadata = Metadata::new();
	while let Some((directive, span)) = meta_loop(cur, false, &mut metadata) {
		directive.invalid_place(cur, span);
	}
	metadata
}

macro_rules! head_type {
	() => { &mut impl FnMut(&mut Cursor) -> Result<TypeId, Metadata> };
}

fn parse_arr_typeid(
	cur: &mut Cursor, scope: &mut Scope, metadata: Metadata, head: head_type!(),
) -> TypeId {
	let typeid = cur.eat_delim(angle_open, angle_close, |cur| {
		let item = parse_typespec(cur, scope, false, head).ok()?;
		cur.eat(comma).ok()?;
		let nb = cur.eat_int()?;
		Some(TypeId::arr(item, nb))
	});
	TypeId { metadata, ..typeid.unwrap_or(TypeId::ANY) }
}
fn skip_generics_and_end(
	cur: &mut Cursor, scope: &mut Scope, head: head_type!(), metadata: Metadata,
) -> TypeId {
	cur.try_delim_list::<_, ()>(angle_open, angle_close, |cur| {
		parse_typespec(cur, scope, false, head).ok()
	});
	TypeId { metadata, ..TypeId::ANY }
}
fn resolve_primary(
	primary: &Ident, cur: &mut Cursor, scope: &mut Scope, head: head_type!(),
	metadata: Metadata,
) -> ControlFlow<TypeId, (TypeId, u16)> {
	if *primary == "arr" {
		return Break(parse_arr_typeid(cur, scope, metadata, head));
	} else if let Some(mut typeid) = builtin_by_name(primary.val, scope.profile) {
		let params = match typeid.item {
			ItemId::LIST | ItemId::FAR => 2,
			ItemId::MAP => 2,
			_ => 0,
		};
		typeid.metadata = metadata;
		Continue((typeid, params))
	} else if let Some(mut res) = scope.resolve_top(primary.val) {
		res.0.metadata = metadata;
		Continue(res)
	} else {
		cur.error(format!("undefined item \"{primary}\""), primary.span);
		return Break(skip_generics_and_end(cur, scope, head, metadata));
	}
}
fn resolve_prim_dot_sec(
	primary: &Ident, cur: &mut Cursor, scope: &mut Scope, in_value: bool,
	head: head_type!(), metadata: Metadata,
) -> ControlFlow<TypeId, (TypeId, u16)> {
	let Some(secondary) = cur.try_ident() else {
		return Break(skip_generics_and_end(cur, scope, head, metadata));
	};
	if let Some(file) = scope.namespaces.get(primary.val) {
		let Some(item) = file.item_by_name(secondary.val) else {
			cur.error(
				format!("undefined item \"{primary}.{secondary}\""),
				primary.span.join(secondary.span),
			);
			return Break(skip_generics_and_end(cur, scope, head, metadata));
		};
		Continue((
			TypeId { metadata, ..TypeId::new_unchecked(file.ns, item.id) },
			item.generic_params().len() as u16,
		))
	} else if in_value && let Some((typeid, _)) = scope.resolve_top(primary.val) {
		cur.off -= 2;
		return Break(TypeId { metadata, ..typeid });
	} else {
		cur.error(format!("undefined namespace \"{primary}\""), primary.span);
		return Break(skip_generics_and_end(cur, scope, head, metadata));
	}
}
fn parse_typeid_generic(
	params: u16, cur: &mut Cursor, scope: &mut Scope, start: Span, head: head_type!(),
) -> ThinVec<TypeId> {
	let mut list: ThinVec<_> = cur
		.try_delim_list(angle_open, angle_close, |cur| {
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

pub fn parse_typespec(
	cur: &mut Cursor, scope: &mut Scope, in_value: bool, head: head_type!(),
) -> Result<TypeId, Metadata> {
	let metadata = match head(cur) {
		Ok(typeid) => return Ok(typeid),
		Err(metadata) => metadata,
	};

	let Some(primary) = cur.try_ident() else {
		if !in_value {
			cur.expected("a type specifier")
		}
		return Err(metadata);
	};

	let (mut typeid, params) = match match cur.try_symbol(dot) {
		false => resolve_primary(&primary, cur, scope, head, metadata),
		true => resolve_prim_dot_sec(&primary, cur, scope, in_value, head, metadata),
	} {
		Continue(res) => res,
		Break(typeid) => return Ok(typeid),
	};

	typeid.params = parse_typeid_generic(params, cur, scope, primary.span, head);

	Ok(typeid)
}
