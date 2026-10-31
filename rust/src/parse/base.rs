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
			Block, alt, angle_close, angle_open, bang, comma, dot, hash, paren_open,
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
		cur.skip_block(Block::Paren);
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

pub fn parse_typeid<'src>(
	cur: &mut Cursor<'src, '_>, scope: &mut Scope, in_value: bool,
	over_ride: &mut impl FnMut(&mut Cursor<'src, '_>) -> Option<TypeId>,
) -> Result<TypeId, Metadata> {
	let mut metadata = Metadata::new();
	while let Some((directive, span)) = meta_loop(cur, false, &mut metadata) {
		directive.invalid_place(cur, span);
	}

	if let Some(mut typeid) = over_ride(cur) {
		typeid.metadata = metadata;
		return Ok(typeid);
	}

	let mut skip_generics_and_end =
		|cur: &mut Cursor<'src, '_>, scope: &mut Scope, metadata| {
			cur.try_delim_list::<_, ()>(angle_open, angle_close, |cur| {
				parse_typeid(cur, scope, false, over_ride).ok()
			});
			Ok(TypeId { metadata, ..TypeId::ANY })
		};

	let Some(primary) = cur.try_ident() else {
		if !in_value {
			cur.expected("a type specifier")
		}
		return Err(metadata);
	};
	let (mut typeid, params) = if !cur.try_symbol(dot) {
		if primary == "arr" {
			let typeid = cur.eat_delim(angle_open, angle_close, |cur| {
				let item = parse_typeid(cur, scope, false, over_ride).ok()?;
				cur.eat(comma).ok()?;
				let nb = cur.eat_int()?;
				Some(TypeId::arr(item, nb))
			});
			return Ok(TypeId { metadata, ..typeid.unwrap_or(TypeId::ANY) });
		} else if let Some(typeid) = builtin_by_name(primary.val, scope.profile) {
			let params = match typeid.item {
				ItemId::LIST | ItemId::FAR => 2,
				ItemId::MAP => 2,
				_ => 0,
			};
			(typeid, params)
		} else if let Some(res) = scope.resolve_top(primary.val) {
			res
		} else {
			cur.error(format!("undefined item \"{primary}\""), primary.span);
			return skip_generics_and_end(cur, scope, metadata);
		}
	} else {
		let Some(secondary) = cur.try_ident() else {
			return skip_generics_and_end(cur, scope, metadata);
		};
		if let Some(file) = scope.namespaces.get(primary.val) {
			let Some(item) = file.item_by_name(secondary.val) else {
				cur.error(
					format!("undefined item \"{primary}.{secondary}\""),
					primary.span.join(secondary.span),
				);
				return skip_generics_and_end(cur, scope, metadata);
			};
			(TypeId::new_unchecked(file.ns, item.id), item.generic_params().len() as u16)
		} else if in_value && let Some((typeid, _)) = scope.resolve_top(primary.val) {
			cur.off -= 2;
			return Ok(TypeId { metadata, ..typeid });
		} else {
			cur.error(format!("undefined namespace \"{primary}\""), primary.span);
			return skip_generics_and_end(cur, scope, metadata);
		}
	};
	let span = primary.span.join(cur.prev().span);

	if params > 0 {
		let mut list: ThinVec<_> = cur.eat_delim_list(angle_open, angle_close, |cur| {
			parse_typeid(cur, scope, false, over_ride).ok()
		});
		if list.len() != params as usize {
			let got = list.len();
			cur.error(format!("expected {params} generic parameters, got {got}"), span);
			list.resize(params as usize, TypeId::ANY);
		}
		typeid.params = list;
	}

	typeid.metadata = metadata;
	Ok(typeid)
}
