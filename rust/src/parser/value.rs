use std::collections::HashMap;

use crate::{
	DeclProvider, Error, Key, ParseOptions, Value,
	builtins::BUILT_INS_IDS,
	declaration::{DeclItem, EnumVariant, StructDef, TypeId, resolve_typeid},
	errors::{end_of_input, unexpected_token},
	parser::{
		declaration::{DeclContext, parse_metadata},
		rich_types::{parse_dur, parse_inst, parse_uuid},
		tokenizer::Token,
		utils::{consume_ident, consume_str, consume_symbol, struct_like_end, struct_like_start},
	},
};

fn mismatch_types<T>(expected: &str, found: &str, ind: usize) -> Result<T, Error> {
	Err(Error::TypeError(format!("expected type {expected}, found {found} at {ind}",)))
}
fn check_range_nb(nb: i64, signed: bool, bits: u8, ind: usize) -> Result<i64, Error> {
	// compute range
	let (min, max) = match signed {
		false => (0, (1 << bits) - 1),
		true => (-(1 << (bits - 1)), (1 << (bits - 1)) - 1),
	};
	// check range
	if nb < min || nb > max {
		return Err(Error::TypeError(format!(
			"number ({nb}) is out of range for {}{bits} number at {ind}",
			if signed { "i" } else { "u" }
		)));
	}
	Ok(nb)
}

fn parse_small_ints(nb: i64, typeid: &TypeId, ind: usize) -> Result<Value, Error> {
	Ok(match typeid.id {
		0x10 => Value::Uint(check_range_nb(nb, false, 8, ind)? as u64),
		0x11 => Value::Uint(check_range_nb(nb, false, 16, ind)? as u64),
		0x12 => Value::Uint(check_range_nb(nb, false, 32, ind)? as u64),
		0x14 => Value::Int(check_range_nb(nb, true, 8, ind)?),
		0x15 => Value::Int(check_range_nb(nb, true, 16, ind)?),
		0x16 => Value::Int(check_range_nb(nb, true, 32, ind)?),
		_ => unreachable!(),
	})
}

fn parse_typeid(
	tokens: &[Token], ind: &mut usize, loc: &impl Fn() -> String, ctx: &DeclContext<'_>,
	options: &ParseOptions,
) -> Result<TypeId, Error> {
	let metadata = parse_metadata(tokens, ind, loc, options)?;

	let typename = consume_ident(tokens, ind)?;

	crate::parse_typeid_general!((tokens, ind, typename, loc, metadata, ctx, options), |_| loc())
}

fn parse_arr(
	tokens: &[Token], ind: &mut usize, typeid: &TypeId, ctx: &DeclContext,
	provider: &dyn DeclProvider, options: &ParseOptions,
) -> Result<Value, Error> {
	consume_symbol('[', tokens, ind)?;

	// replace any typeid with arr<any>
	let typeid = if typeid.is_any() {
		&TypeId::with_variant(0, 0x22, 0, Some(TypeId::ANY), None)
	} else {
		typeid
	};

	let mut arr = Vec::new();
	let mut watched_comma = true;
	let itemid = typeid.item.as_ref().unwrap().as_ref();

	// loop through items
	loop {
		if struct_like_start(tokens, ind, &mut watched_comma, ']')? {
			break;
		}

		arr.push(parse_value(tokens, ind, itemid, ctx, provider, options)?);

		struct_like_end(tokens, ind, &mut watched_comma);
	}

	Ok(Value::Array(arr))
}
fn parse_map(
	tokens: &[Token], ind: &mut usize, typeid: &TypeId, ctx: &DeclContext,
	provider: &dyn DeclProvider, options: &ParseOptions,
) -> Result<Value, Error> {
	consume_symbol('{', tokens, ind)?;

	// replace any typeid with map<any, any>
	let typeid = if typeid.is_any() {
		&TypeId::with_variant(0, 0x23, 1, Some(TypeId::ANY), None)
	} else {
		typeid
	};

	let mut map = HashMap::new();
	let keyid = &TypeId::new(0, typeid.variant, None);
	let itemid = typeid.item.as_ref().unwrap().as_ref();
	let mut watched_comma = true;
	println!("typeid: {keyid:?}");

	// loop through items
	loop {
		if struct_like_start(tokens, ind, &mut watched_comma, '}')? {
			break;
		}

		let key_ind = tokens[*ind].ind();
		*ind += 1; // skip key
		let key = match tokens.get(*ind - 1) {
			Some(Token::Identifier(key, _)) => Key::from(*key),
			Some(Token::Str(key, _)) => Key::Str(key.clone()),
			// [key]
			Some(Token::Symbol('[', _)) => {
				let key = parse_value(tokens, ind, keyid, ctx, provider, options)?;
				consume_symbol(']', tokens, ind)?;
				// Value => Key
				key.try_into().map_err(|_| {
					Error::TypeError(format!("map key can only be a primitive at {key_ind}"))
				})?
			}
			_ => return Err(unexpected_token(tokens[*ind - 1].to_string(), key_ind)),
		};

		if let Key::Str(key) = &key
			&& !matches!(keyid.id, 1 | 0x20)
		{
			mismatch_types(&keyid.name(provider), "str", key_ind)?
		}

		// check for collision
		if map.contains_key(&key) {
			return Err(Error::TypeError(format!("duplicated map key {key:?} at {key_ind}",)));
		}

		consume_symbol(':', tokens, ind)?;

		let value = parse_value(tokens, ind, itemid, ctx, provider, options)?;
		map.insert(key, value);

		struct_like_end(tokens, ind, &mut watched_comma);
	}

	Ok(Value::Map(map))
}

// parse structs / enums
enum ResolveDefResult<'a> {
	Norm(&'a StructDef, &'a str),
	CaseUnitVariant(&'a str),
}
fn resolve_item_def<'a>(
	tokens: &[Token], ind: &mut usize, map: &mut HashMap<Key, Value>, item: &'a DeclItem,
	variant: Option<&'a EnumVariant>, start_ind: usize,
) -> Result<ResolveDefResult<'a>, Error> {
	use ResolveDefResult::*;

	if let DeclItem::Enum { .. } = item {
		let variant = match variant {
			// only variant name is written
			Some(variant) => variant,
			// case Type.variant
			_ => {
				consume_symbol('.', tokens, ind)?;
				let variant = consume_ident(tokens, ind)?;
				item.get_variant_by_name(variant).ok_or_else(|| {
					Error::TypeError(format!(
						"variant \"{variant}\" not found in enum \"{}\" at {start_ind}",
						item.name()
					))
				})?
			}
		};

		// unit variant is parsed into a string
		if variant.def.is_none() {
			return Ok(CaseUnitVariant(&variant.name));
		}
		// variant with fields is parsed into a map with its fields
		map.insert("$enum_variant".into(), variant.name.clone().into());

		Ok(Norm(variant.def.as_ref().unwrap(), variant.name.as_str()))

	// it is struct
	} else if let DeclItem::Struct { def, .. } = item {
		Ok(Norm(def, ""))
	} else {
		unreachable!()
	}
}
fn parse_item(
	tokens: &[Token], ind: &mut usize, typeid: &TypeId, variant: Option<&EnumVariant>,
	start_ind: usize, ctx: &DeclContext, provider: &dyn DeclProvider, options: &ParseOptions,
) -> Result<Value, Error> {
	let item = resolve_typeid(typeid, provider);
	let mut map = HashMap::new();

	// resolve definition
	use ResolveDefResult::*;
	let (def, variant) = match resolve_item_def(tokens, ind, &mut map, item, variant, start_ind)? {
		Norm(def, variant) => (def, variant),
		// for unit enum variants, it is parsed as a str
		CaseUnitVariant(variant) => return Ok(Value::from(variant)),
	};

	// count required fields
	let fields = def.fields.iter();
	let mut required =
		fields.filter(|field| field.as_ref().is_some_and(|f| !f.is_optional)).count();

	consume_symbol('{', tokens, ind)?;
	let mut watched_comma = true;

	// loop through fields
	loop {
		if struct_like_start(tokens, ind, &mut watched_comma, '}')? {
			break;
		}

		let name = match tokens.get(*ind) {
			Some(Token::Identifier(key, _)) => *key,
			Some(Token::Str(key, _)) => key,
			_ => return Err(unexpected_token(tokens[*ind].to_string(), tokens[*ind].ind())),
		};

		// check for existence
		let field = &def.get_field_by_name(name).ok_or_else(|| {
			Error::TypeError(format!(
				"struct {}{} doesnt contain field \"{name}\" at {ind}",
				typeid.name(provider),
				if variant.is_empty() { "".to_string() } else { format!(".{variant}") }
			))
		})?;

		// check for collision
		let key = Key::from(name);
		if map.contains_key(&key) {
			return Err(Error::TypeError(format!(
				"duplicated field \"{name}\" at {}",
				tokens[*ind].ind()
			)));
		}
		*ind += 1;

		consume_symbol(':', tokens, ind)?;

		if !field.is_optional {
			required -= 1;
		}

		let value = parse_value(tokens, ind, &field.typeid, ctx, provider, options)?;
		map.insert(key, value);

		struct_like_end(tokens, ind, &mut watched_comma);
	}

	// case of missing required fields
	if required != 0 {
		return Err(Error::TypeError(format!(
			"struct {}{} is missing required fields at {start_ind}",
			typeid.name(provider),
			if variant.is_empty() { "".to_string() } else { format!(".{variant}") }
		)));
	}

	Ok(Value::Map(map))
}

fn parse_ident(
	ident: &str, tokens: &[Token], ind: &mut usize, typeid: &TypeId, provider: &dyn DeclProvider,
	ctx: &DeclContext<'_>, options: &ParseOptions,
) -> Result<Value, Error> {
	let start_ind = tokens[*ind - 1].ind();

	match ident {
		// bool
		"true" | "false" => {
			if typeid.ns != 0 || !matches!(typeid.id, 1 | 8) {
				mismatch_types(&typeid.name(provider), "bool", *ind)?;
			}
			Ok(Value::Bool(ident == "true"))
		}

		// float constants
		"nan" => {
			if typeid.ns != 0 || !matches!(typeid.id, 1 | 0x18..=0x1a) {
				mismatch_types(&typeid.name(provider), "f64", *ind)?;
			}
			Ok(Value::Float(f64::NAN))
		}
		"inf" => {
			if typeid.ns != 0 || !matches!(typeid.id, 1 | 0x18..=0x1a) {
				mismatch_types(&typeid.name(provider), "f64", *ind)?;
			}
			Ok(Value::Float(f64::INFINITY))
		}

		// rich types
		"uuid" => {
			if typeid.ns != 0 || !matches!(typeid.id, 1 | 0x33) {
				mismatch_types(&typeid.name(provider), "uuid", *ind)?;
			}
			parse_uuid(consume_str(tokens, ind)?, *ind)
		}
		"inst" => {
			if typeid.ns != 0 || !matches!(typeid.id, 1 | 0x30) {
				mismatch_types(&typeid.name(provider), "inst", *ind)?;
			}
			parse_inst(consume_str(tokens, ind)?, false, *ind)
		}
		"instN" => {
			if typeid.ns != 0 || !matches!(typeid.id, 1 | 0x31) {
				mismatch_types(&typeid.name(provider), "instN", *ind)?;
			}
			parse_inst(consume_str(tokens, ind)?, true, *ind)
		}
		"dur" => {
			if typeid.ns != 0 || !matches!(typeid.id, 1 | 0x32) {
				mismatch_types(&typeid.name(provider), "dur", *ind)?;
			}
			parse_dur(consume_str(tokens, ind)?, start_ind, tokens[*ind - 1].ind())
		}

		// maps, arrs, structs and enums
		_ => {
			// infered enums written only with variant names shortcut
			if typeid.ns != 0 {
				let item = resolve_typeid(typeid, provider);

				if let Some(variant) = item.get_variant_by_name(ident) {
					#[rustfmt::skip]
					return parse_item(
						tokens, ind, typeid, Some(variant), start_ind, ctx, provider, options,
					);
				}
			}

			*ind -= 1;

			// parse explicit type
			let explicit_type =
				parse_typeid(tokens, ind, &|| format!("at index {start_ind}"), ctx, options)?;

			// check against the implicit type
			if typeid != &explicit_type {
				return Err(mismatch_types(
					&typeid.name(provider),
					&explicit_type.name(provider),
					*ind,
				)?);
			}
			// replace with the explicit type if implicit is any
			let typeid = if typeid.is_any() { &explicit_type } else { typeid };

			// builtins
			if typeid.ns == 0 {
				match typeid.id {
					0x22 => parse_arr(tokens, ind, typeid, ctx, provider, options),
					0x23 => parse_map(tokens, ind, typeid, ctx, provider, options),
					_ => Err(unexpected_token(ident, start_ind)),
				}
			// user types
			} else {
				parse_item(tokens, ind, typeid, None, start_ind, ctx, provider, options)
			}
		}
	}
}

pub fn parse_value(
	tokens: &[Token], ind: &mut usize, typeid: &TypeId, ctx: &DeclContext<'_>,
	provider: &dyn DeclProvider, options: &ParseOptions,
) -> Result<Value, Error> {
	let start_ind = *ind;

	let metadata = parse_metadata(tokens, ind, &|| format!("at index {start_ind}"), options)?;
	println!("{} {:?}", typeid.name(provider), tokens[*ind]);
	*ind += 1;
	let value = match tokens.get(*ind - 1) {
		Some(Token::Identifier(ident, _)) => {
			parse_ident(ident, tokens, ind, typeid, provider, ctx, options)?
		}
		// ananonymous arrays
		Some(Token::Symbol('[', _)) => {
			if typeid.ns != 0 || !matches!(typeid.id, 1 | 0x22) {
				return mismatch_types(&typeid.name(provider), "arr", *ind);
			}
			*ind -= 1;
			parse_arr(tokens, ind, typeid, ctx, provider, options)?
		}
		// anonymous maps and structs
		Some(Token::Symbol('{', _)) => {
			*ind -= 1;
			if typeid.ns == 0 {
				if !matches!(typeid.id, 1 | 0x23) {
					return mismatch_types(&typeid.name(provider), "map", *ind);
				}
				parse_map(tokens, ind, typeid, ctx, provider, options)?
			} else {
				parse_item(tokens, ind, typeid, None, start_ind, ctx, provider, options)?
			}
		}
		// numbers
		Some(Token::Uint(nb, _)) => {
			if typeid.ns != 0 {
				return mismatch_types(&typeid.name(provider), "uint", *ind);
			}
			match typeid.id {
				0x10..=0x12 | 0x14..=0x16 => parse_small_ints(*nb as i64, typeid, *ind)?,
				0x13 | 0x1c | 1 => Value::Uint(*nb),
				// signed int types with unsigned nb literial
				0x17 | 0x1d => {
					if *nb > 1 << 63 {
						return Err(Error::TypeError(format!(
							"number ({nb}) is out of range for i64 nb at {ind}",
						)));
					}
					Value::Int(*nb as i64)
				}
				0x18..=0x1a => Value::Float(*nb as f64),
				_ => return mismatch_types(&typeid.name(provider), "uint", *ind),
			}
		}
		Some(Token::Int(nb, _)) => {
			if typeid.ns != 0 {
				return mismatch_types(&typeid.name(provider), "int", *ind);
			}
			match typeid.id {
				0x10..=0x12 | 0x14..=0x16 => parse_small_ints(*nb as i64, typeid, *ind)?,
				0x13 | 0x1c => {
					// unsigned int types with signed nb literial
					if *nb < 0 {
						return Err(Error::TypeError(format!(
							"number ({nb}) is out of range for u64 nb at {ind}",
						)));
					}
					Value::Uint(*nb as u64)
				}
				0x17 | 0x1d | 1 => Value::Int(*nb),
				0x18..=0x1a => Value::Float(*nb as f64),
				_ => return mismatch_types(&typeid.name(provider), "int", *ind),
			}
		}
		// +inf / -inf
		Some(Token::Symbol(symbol, _)) if matches!(symbol, '+' | '-') => {
			if typeid.ns != 0 || !matches!(typeid.id, 1 | 0x18..=0x1a) {
				mismatch_types(&typeid.name(provider), "f64", *ind)?;
			}

			let ident = consume_ident(tokens, ind)?;
			if ident != "inf" {
				return Err(unexpected_token(symbol, start_ind));
			}

			Value::Float(if *symbol == '+' { f64::INFINITY } else { f64::NEG_INFINITY })
		}
		Some(Token::Float(nb, _)) => {
			if typeid.ns != 0 || !matches!(typeid.id, 1 | 0x10..=0x1f) {
				return mismatch_types(&typeid.name(provider), "f64", *ind);
			}
			Value::Float(*nb)
		}
		Some(Token::BigInt(_, _)) => {
			if typeid.ns != 0 || typeid.id != 0x1e {
				return mismatch_types(&typeid.name(provider), "bint", *ind);
			}
			Value::BigInt(vec![])
		}
		// strings
		Some(Token::Str(str, _)) => {
			if typeid.ns != 0 || !matches!(typeid.id, 1 | 0x20) {
				return mismatch_types(&typeid.name(provider), "str", *ind);
			}
			Value::Str(str.clone())
		}
		_ => return Err(end_of_input(tokens.len())),
	};

	// add metadata wrapper around the value
	if options.metadata && (metadata.is_some() || typeid.metadata.is_some()) {
		let mut wrapper = HashMap::new();
		wrapper.insert("$has_meta".into(), Value::Bool(true));

		// declared metadata in declerations
		if let Some(metadata) = typeid.metadata.as_ref() {
			for (name, value) in metadata {
				wrapper.insert(Key::from(name.clone()), Value::from(value.clone()));
			}
		}
		// then declared ones in value source
		if let Some(metadata) = metadata {
			for (name, value) in metadata {
				wrapper.insert(Key::from(name), Value::from(value));
			}
		}

		wrapper.insert("value".into(), value);
		Ok(Value::Map(wrapper))
	} else {
		Ok(value)
	}
}
