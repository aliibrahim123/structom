use std::{collections::HashMap, fmt::Display};

use crate::{
	DeclProvider, Key, ParseError, ParseOptions, Value,
	builtins::{
		ANY_TYPEID, ARR_TYPEID, BINT_TYPEID, BOOL_TYPEID, BUILT_INS_IDS, DUR_TYPEID, F32_TYPEID,
		F64_TYPEID, I8_TYPEID, I16_TYPEID, I32_TYPEID, I64_TYPEID, INST_TYPEID, INSTN_TYPEID,
		MAP_TYPEID, STR_TYPEID, U8_TYPEID, U16_TYPEID, U32_TYPEID, U64_TYPEID, UUID_TYPEID,
		VINT_TYPEID, VUINT_TYPEID,
	},
	declaration::{DeclItem, StructDef, TypeId, resolve_typeid},
	errors::err,
	parser::{
		declaration::{DeclContext, parse_metadata, parse_typeid_general},
		rich_types::{parse_dur, parse_inst, parse_uuid},
		tokenizer::{Pos, Token},
		utils::{
			consume_ident, consume_str, consume_symbol, end_of_input, parse_struct_like,
			try_consume_symbol, unexpected_token,
		},
	},
};

/// common variables used in parse_value
pub struct ValueCtx<'a> {
	pub file: &'a str,
	pub options: &'a ParseOptions,
	pub provider: &'a dyn DeclProvider,
	pub decl: &'a DeclContext<'a>,
}
/// create a mismatch types error
pub fn mismatch_types<T>(
	expected: impl Display, found: impl Display, pos: Pos, file: &str,
) -> Result<T, ParseError> {
	err!(format!("expected type {expected}, found {found}"), pos, file)
}

fn check_nb_range<T: Into<i64>>(
	nb: i64, kind: &str, min: T, max: T, pos: Pos, file: &str,
) -> Result<i64, ParseError> {
	if nb < min.into() || nb > max.into() {
		return err!(format!("number ({nb}) is out of range for {kind}"), pos, file);
	}
	Ok(nb)
}
fn downcast_small_ints(nb: i64, typeid: u16, pos: Pos, file: &str) -> Result<Value, ParseError> {
	Ok(match typeid {
		U8_TYPEID => Value::Uint(check_nb_range(nb, "u8", u8::MIN, u8::MAX, pos, file)? as u64),
		U16_TYPEID => Value::Uint(check_nb_range(nb, "u16", u16::MIN, u16::MAX, pos, file)? as u64),
		U32_TYPEID => Value::Uint(check_nb_range(nb, "u32", u32::MIN, u32::MAX, pos, file)? as u64),
		I8_TYPEID => Value::Int(check_nb_range(nb, "i8", i8::MIN, i8::MAX, pos, file)? as i64),
		I16_TYPEID => Value::Int(check_nb_range(nb, "i16", i16::MIN, i16::MAX, pos, file)? as i64),
		I32_TYPEID => Value::Int(check_nb_range(nb, "i32", i32::MIN, i32::MAX, pos, file)? as i64),
		_ => unreachable!(),
	})
}

fn parse_typeid(
	tokens: &[Token], ind: &mut usize, ctx: &DeclContext<'_>, options: &ParseOptions,
) -> Result<TypeId, ParseError> {
	let metadata = parse_metadata(tokens, ind, options, &ctx.file.name)?;
	parse_typeid_general!((tokens, ind, metadata, ctx, options))
}
/// grammer: "[]" | "[" value ("," value)* [","] "]"
fn parse_arr(
	tokens: &[Token], ind: &mut usize, typeid: &TypeId, ctx: &ValueCtx,
) -> Result<Value, ParseError> {
	let file = ctx.file;
	// default to arr<any>
	let typeid = if typeid.is_any() { &TypeId::arr(TypeId::ANY, None) } else { typeid };
	let itemid = typeid.item();

	let mut arr = Vec::new();
	parse_struct_like!((tokens, '[', ']'), file, ind => {
		arr.push(parse_value(tokens, ind, itemid, ctx)?);
	});
	Ok(Value::Arr(arr))
}

/// grammer: "{}" | "{" map_item ("," map_item)* [","] "}"
/// map_item: (ident | str | "[" value "]") ":" value
fn parse_map(
	tokens: &[Token], ind: &mut usize, typeid: &TypeId, ctx: &ValueCtx,
) -> Result<Value, ParseError> {
	let file = ctx.file;
	// default to map<any, any>
	let typeid = if typeid.is_any() { &TypeId::map(ANY_TYPEID, TypeId::ANY, None) } else { typeid };
	let keyid = &TypeId::new(0, typeid.variant, None);
	let valueid = typeid.item();

	let mut map = HashMap::new();
	parse_struct_like!((tokens, '{', '}'), file, ind => {
		let pos = tokens[*ind].pos();
		*ind +=1;
		let key = match tokens.get(*ind -1) {
			Some(Token::Ident(key, _)) => Key::from(*key),
			Some(Token::Str(key, _)) => Key::Str(key.clone()),
			// [key]
			Some(Token::Symbol('[', _)) => {
				let key_value = parse_value(tokens, ind, keyid, ctx)?;
				consume_symbol(']', tokens, ind, file)?;
				let Ok(key) = key_value.try_into() else {
					return err!("map key can only be primitive".to_string(), pos, file)
				};
				key
			}
			Some(Token::Eof(_)) | None => return end_of_input(file),
			Some(token) => return unexpected_token(token, pos, file),
		};
		// the str path in keys doesnt check types
		if matches!(key, Key::Str(_),) && !matches!(keyid.id, ANY_TYPEID | STR_TYPEID) {
			mismatch_types(keyid.name(ctx.provider), "str", pos, file)?
		}
		if map.contains_key(&key) {
			return err!(format!("duplicate map key {key:?}"), pos, file);
		}

		consume_symbol(':', tokens, ind, file)?;

		let value = parse_value(tokens, ind, valueid, ctx)?;
		map.insert(key, value);
	});
	Ok(Value::Map(Box::new(map)))
}

// marco since it short circuit the caller
macro_rules! enum_prologue {
	($variant:ident, $map:ident) => {{
		let name = $variant.name.clone();
		let Some(def) = &$variant.def else {
			return Ok(Value::UnitVar(name));
		};
		$map.insert(Key::enum_variant_key().clone(), name.into());
		def
	}};
}
/// parse struct / enum
fn parse_item(
	tokens: &[Token], ind: &mut usize, typeid: &TypeId, start_pos: Pos, ctx: &ValueCtx,
) -> Result<Value, ParseError> {
	let ValueCtx { file, provider, .. } = ctx;
	let item = resolve_typeid(typeid, *provider);
	let mut map = HashMap::new();

	let (def, item_name) = match item {
		DeclItem::Struct { def, .. } => (def, format!("struct {}", item.name())),
		// type specified enum: enum.variant
		DeclItem::Enum { .. } => {
			consume_symbol('.', tokens, ind, file)?;
			let variant = consume_ident(tokens, ind, file)?;
			let Some(variant) = item.get_variant_by_name(variant) else {
				let msg =
					format!("variant \"{variant}\" does not exist in enum \"{}\"", item.name());
				return err!(msg, start_pos, file);
			};
			let def = enum_prologue!(variant, map);
			(def, format!("enum variant {}.{}", item.name(), variant.name))
		}
	};

	parse_fields(tokens, ind, def, map, &item_name, start_pos, ctx)
}
// grammer: "{" field ("," field)* [","] "}"
// field: (ident | str) ":" value
fn parse_fields(
	tokens: &[Token], ind: &mut usize, def: &StructDef, mut map: HashMap<Key, Value>,
	item_name: &str, start_pos: Pos, ctx: &ValueCtx,
) -> Result<Value, ParseError> {
	let file = ctx.file;
	let mut required = def.required_fields;

	parse_struct_like!((tokens, '{', '}'), file, ind => {
		let pos = tokens[*ind].pos();
		let name = match tokens.get(*ind) {
			Some(Token::Ident(key, _)) => *key,
			Some(Token::Str(key, _)) => key,
			Some(Token::Eof(_)) | None => return end_of_input(file),
			Some(token) => return unexpected_token(token, pos, file),
		};
		*ind+= 1;

		let Some(field) = def.get_field_by_name(name)else  {
			return err!(format!("{item_name} doesnt contain field \"{name}\""), pos, file);
		};
		let key = Key::from(name);
		if map.contains_key(&key) {
			return err!(format!("duplicated field \"{name}\""), pos, file);
		}

		consume_symbol(':', tokens, ind, file)?;

		let value = parse_value(tokens, ind, &field.typeid, ctx)?;
		if !field.is_optional {
			required -= 1;
		}
		map.insert(key, value);
	});

	if required != 0 {
		return err!(format!("{item_name} is missing required fields"), start_pos, file);
	}
	Ok(Value::Map(Box::new(map)))
}

fn parse_ident(
	ident: &str, tokens: &[Token], ind: &mut usize, typeid: &TypeId, ctx: &ValueCtx,
) -> Result<Value, ParseError> {
	let pos = tokens[*ind - 1].pos();
	let ValueCtx { file, provider, .. } = ctx;
	/// ensure type, can be any
	macro_rules! check_builtin {
		($ty:literal, $pat:pat) => {
			if !typeid.is_builtin() || !matches!(typeid.id, 1 | $pat) {
				mismatch_types(&typeid.name(*provider), $ty, pos, file)?;
			}
		};
	}

	// builtins
	match ident {
		"true" | "false" => {
			check_builtin!("bool", BOOL_TYPEID);
			return Ok(Value::Bool(ident == "true"));
		}
		"nan" => {
			check_builtin!("f64", F32_TYPEID | F64_TYPEID);
			return Ok(Value::Float(f64::NAN));
		}
		"inf" => {
			check_builtin!("f64", F32_TYPEID | F64_TYPEID);
			return Ok(Value::Float(f64::INFINITY));
		}
		"uuid" => {
			check_builtin!("uuid", UUID_TYPEID);
			return parse_uuid(consume_str(tokens, ind, file)?, pos, file);
		}
		"inst" => {
			check_builtin!("inst", INST_TYPEID | INSTN_TYPEID);
			return parse_inst(consume_str(tokens, ind, file)?, false, pos, file);
		}
		"instN" => {
			check_builtin!("instN", INSTN_TYPEID);
			return parse_inst(consume_str(tokens, ind, file)?, true, pos, file);
		}
		"dur" => {
			check_builtin!("dur", DUR_TYPEID);
			return parse_dur(tokens, ind, file);
		}
		_ => (),
	}

	// variant only enum path
	if !typeid.is_builtin()
		&& let item = resolve_typeid(typeid, *provider)
		&& let Some(variant) = item.get_variant_by_name(ident)
	{
		let mut map = HashMap::new();
		let def = enum_prologue!(variant, map);
		let name = format!("enum variant {}.{}", item.name(), variant.name);
		return parse_fields(tokens, ind, def, map, &name, pos, ctx);
	}

	// typed specified containers
	*ind -= 1;
	let explicit_type = parse_typeid(tokens, ind, ctx.decl, ctx.options)?;
	if typeid != &explicit_type {
		return mismatch_types(typeid.name(*provider), explicit_type.name(*provider), pos, file);
	}
	let typeid = if typeid.is_any() { &explicit_type } else { typeid };

	if typeid.is_builtin() {
		match typeid.id {
			ARR_TYPEID => parse_arr(tokens, ind, typeid, ctx),
			MAP_TYPEID => parse_map(tokens, ind, typeid, ctx),
			_ => unexpected_token(ident, pos, file),
		}
	} else {
		parse_item(tokens, ind, typeid, pos, ctx)
	}
}

/// parse any value
pub fn parse_value(
	tokens: &[Token], ind: &mut usize, typeid: &TypeId, ctx: &ValueCtx,
) -> Result<Value, ParseError> {
	let ValueCtx { file, provider, options, .. } = ctx;
	let pos = tokens[*ind].pos();
	/// ensure type, can be any
	macro_rules! check_builtin {
		($ty:literal, $pat:pat) => {
			if !typeid.is_builtin() || !matches!(typeid.id, 1 | $pat) {
				mismatch_types(&typeid.name(*provider), $ty, pos, file)?;
			}
		};
	}

	let metadata = parse_metadata(tokens, ind, options, file)?;

	*ind += 1;
	let value = match tokens.get(*ind - 1) {
		Some(Token::Ident(ident, _)) => parse_ident(ident, tokens, ind, typeid, ctx)?,
		Some(Token::Symbol('[', _)) => {
			check_builtin!("arr", ARR_TYPEID);
			*ind -= 1;
			parse_arr(tokens, ind, typeid, ctx)?
		}
		Some(Token::Symbol('{', _)) => {
			*ind -= 1;
			if typeid.is_builtin() {
				check_builtin!("map", MAP_TYPEID);
				parse_map(tokens, ind, typeid, ctx)?
			} else {
				parse_item(tokens, ind, typeid, pos, ctx)?
			}
		}
		Some(Token::Uint(nb, _)) => {
			check_builtin!("uint", U8_TYPEID..=F64_TYPEID | VUINT_TYPEID | VINT_TYPEID);
			match typeid.id {
				U8_TYPEID..=U32_TYPEID | I8_TYPEID..=I32_TYPEID => {
					downcast_small_ints(*nb as i64, typeid.id, pos, file)?
				}
				ANY_TYPEID | U64_TYPEID | VUINT_TYPEID => Value::Uint(*nb),
				I64_TYPEID | VINT_TYPEID => {
					if *nb > i64::MAX as u64 {
						return err!(format!("number ({nb}) is out of range for i64"), pos, file);
					}
					Value::Int(*nb as i64)
				}
				F32_TYPEID | F64_TYPEID => Value::Float(*nb as f64),
				_ => unreachable!(),
			}
		}
		Some(Token::Int(nb, _)) => {
			check_builtin!("int", U8_TYPEID..=F64_TYPEID | VUINT_TYPEID | VINT_TYPEID);
			match typeid.id {
				U8_TYPEID..=U32_TYPEID | I8_TYPEID..=I32_TYPEID => {
					downcast_small_ints(*nb, typeid.id, pos, file)?
				}
				ANY_TYPEID | I64_TYPEID | VINT_TYPEID => Value::Int(*nb),
				U64_TYPEID | VUINT_TYPEID => {
					if *nb < 0 {
						return err!(format!("number ({nb}) is out of range for u64"), pos, file);
					}
					Value::Uint(*nb as u64)
				}
				F32_TYPEID | F64_TYPEID => Value::Float(*nb as f64),
				_ => unreachable!(),
			}
		}
		Some(Token::Symbol(symbol @ ('+' | '-'), _)) => {
			check_builtin!("f64", F32_TYPEID | F64_TYPEID);
			let ident = consume_ident(tokens, ind, file)?;
			if ident != "inf" {
				return unexpected_token(ident, pos, file);
			}
			Value::Float(if *symbol == '+' { f64::INFINITY } else { f64::NEG_INFINITY })
		}
		Some(Token::Float(nb, _)) => {
			check_builtin!("f64", F32_TYPEID | F64_TYPEID);
			Value::Float(*nb)
		}
		Some(Token::BigInt(nb, _)) => {
			check_builtin!("bigint", BINT_TYPEID);
			Value::BigInt(nb.clone())
		}
		Some(Token::Str(str, _)) => {
			check_builtin!("str", STR_TYPEID);
			Value::Str(str.clone())
		}
		Some(Token::Eof(_)) | None => return end_of_input(file),
		Some(token) => return unexpected_token(token, pos, file),
	};

	if options.metadata && (metadata.is_some() || typeid.metadata.is_some()) {
		let mut wrapper = HashMap::new();
		wrapper.insert(Key::has_meta_key().clone(), Value::Bool(true));
		wrapper.insert(Key::inner_key().clone(), value);

		if let Some(metadata) = typeid.metadata.as_ref() {
			for (name, value) in metadata.as_ref() {
				wrapper.insert(Key::from(name.clone()), Value::from(value.clone()));
			}
		}
		if let Some(metadata) = metadata {
			for (name, value) in metadata {
				wrapper.insert(Key::from(name), Value::from(value));
			}
		}

		Ok(Value::Map(Box::new(wrapper)))
	} else {
		Ok(value)
	}
}
