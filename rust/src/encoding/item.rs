use std::collections::HashMap;

use crate::{
	builtins::{ARR_TYPEID, BINT_TYPEID, MAP_TYPEID, STR_TYPEID}, declaration::{resolve_typeid, DeclItem, StructDef, TypeId}, encoding::{decode_arr, decode_key, decode_map, decode_value, decode_vuint}, DeclProvider, Key, Value
};

pub fn decode_item(
	data: &[u8], ind: &mut usize, item: &DeclItem, provider: &dyn DeclProvider,
) -> Option<Value> {
	match item {
		DeclItem::Struct { def, .. } => decode_struct(data, ind, def, provider),
		DeclItem::Enum { variants, .. } => {
			let variant = variants.get(decode_vuint(data, ind)? as usize)?.as_ref()?;

			// case has fields
			if let Some(def) = &variant.def {
				let mut value = decode_struct(data, ind, def, provider)?;
				value.as_map_mut()?.insert(Key::enum_variant_key().clone(), variant.name.clone().into());
				return Some(value);
			};

			// case unit enum variant
			Some(Value::UnitVar(variant.name.clone()))
		}
	}
}

fn decode_field_value(
	data: &[u8], ind: &mut usize, typeid: &TypeId, in_field: bool, provider: &dyn DeclProvider,
) -> Option<Value> {
	// case user defined type
	Some(if typeid.ns != 0 {
		decode_item(data, ind, resolve_typeid(typeid, provider), provider)?

	// case array
	} else if typeid.id == ARR_TYPEID as u16 {
		let itemid = typeid.item.as_ref()?.as_ref();

		Value::Arr(decode_arr(data, ind, in_field, |data, ind| {
			decode_field_value(data, ind, itemid, false, provider)
		})?)

	// case map
	} else if typeid.id == MAP_TYPEID as u16 {
		let keyid = typeid.variant as u8;
		let itemid = typeid.item.as_ref()?.as_ref();

		Value::Map(Box::new(decode_map(
			data, ind, in_field,
			|data, ind| decode_key(data, ind, keyid),
			|data, ind| decode_field_value(data, ind, itemid, false, provider),
		)?))

	// case builtins
	} else {
		decode_value(data, ind, typeid.id as u8)?
	})
}
pub fn decode_struct(
	data: &[u8], ind: &mut usize, def: &StructDef, provider: &dyn DeclProvider,
) -> Option<Value> {
	let mut map = HashMap::new();
	let mut required = def.required_fields;

	// loop through fields
	for _ in 0..(decode_vuint(data, ind)?) {
		let header = decode_vuint(data, ind)?;
		let field = def.get_field_by_id((header as u32) >> 3);

		// skip undefined tags
		if field.is_none() {
			skip_field(data, ind, header)?;
			continue;
		}

		let field = field.unwrap();
		let name = Key::from(field.name.clone());
		// duplicate fields
		if map.contains_key(&name) {
			return None;
		}
		required -= if field.is_optional { 0 } else { 1 };

		// skip len field for types that dont use it
		#[rustfmt::skip] 
		if header & 0b111 == 0b101 && (field.typeid.ns != 0
			|| !matches!(field.typeid.id as u8,	MAP_TYPEID | ARR_TYPEID | STR_TYPEID | BINT_TYPEID)) 
		{
			decode_vuint(data, ind)?;
		};

		let value = decode_field_value(data, ind, &field.typeid, true, provider)?;
		map.insert(name, value);
	}

	// case not all required fields are present
	if required != 0 {
		return None;
	}

	Some(Value::Map(Box::new(map)))
}
pub fn skip_field(data: &[u8], ind: &mut usize, header: u64) -> Option<()> {
	match header & 0b111 {
		0b000 => *ind += 1,
		0b001 => *ind += 2,
		0b010 => *ind += 4,
		0b011 => *ind += 8,
		// decode vuint and ignore
		0b100 => (decode_vuint(data, ind)?, ()).1,
		// len field is encoded
		0b101 => *ind += decode_vuint(data, ind)? as usize,
		_ => return None,
	};
	Some(())
}