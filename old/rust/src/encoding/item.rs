use std::collections::HashMap;

use crate::{
	DeclProvider, Key, Value,
	builtins::{ARR_TYPEID, BINT_TYPEID, MAP_TYPEID, STR_TYPEID},
	declaration::{DeclItem, StructDef, TypeId, resolve_typeid},
	encoding::{decode_arr, decode_key, decode_map, decode_value, decode_vuint},
};

pub fn decode_item(
	data: &[u8], ind: &mut usize, item: &DeclItem, provider: &dyn DeclProvider,
) -> Option<Value> {
	match item {
		DeclItem::Struct { def, .. } => decode_struct(data, ind, def, provider),
		DeclItem::Enum { .. } => {
			let variant = item.get_variant_by_id(decode_vuint(data, ind)? as u32)?;
			let variant_name = variant.name.clone();

			if let Some(def) = &variant.def {
				let mut value = decode_struct(data, ind, def, provider)?;
				value.as_map_mut()?.insert(Key::enum_variant_key().clone(), variant_name.into());
				return Some(value);
			};

			Some(Value::UnitVar(variant_name))
		}
	}
}

fn decode_field_value(
	data: &[u8], ind: &mut usize, typeid: &TypeId, in_field: bool, provider: &dyn DeclProvider,
) -> Option<Value> {
	if !typeid.is_builtin() {
		return decode_item(data, ind, resolve_typeid(typeid, provider), provider);
	}

	Some(match typeid.id {
		id if id == ARR_TYPEID => {
			let itemid = typeid.item();
			Value::Arr(decode_arr(data, ind, in_field, |data, ind| {
				decode_field_value(data, ind, itemid, false, provider)
			})?)
		}
		id if id == MAP_TYPEID => {
			let keyid = typeid.variant as u8;
			let itemid = typeid.item();
			Value::Map(Box::new(decode_map(
				data,
				ind,
				in_field,
				|data, ind| decode_key(data, ind, keyid),
				|data, ind| decode_field_value(data, ind, itemid, false, provider),
			)?))

			// builtins
		}
		id => decode_value(data, ind, id as u8)?,
	})
}
pub fn decode_struct(
	data: &[u8], ind: &mut usize, def: &StructDef, provider: &dyn DeclProvider,
) -> Option<Value> {
	let mut map = HashMap::new();
	let mut required = def.required_fields;

	let field_count = decode_vuint(data, ind)?;
	for _ in 0..field_count {
		let header = decode_vuint(data, ind)?;
		let Some(field) = def.get_field_by_id((header as u32) >> 3) else {
			skip_field(data, ind, header)?;
			continue;
		};
		let name = Key::from(field.name.clone());
		if map.contains_key(&name) {
			return None;
		}
		required -= if field.is_optional { 0 } else { 1 };

		// skip len field for types that dont use it
		let typeid = &field.typeid;
		let use_len = typeid.is_builtin()
			&& matches!(typeid.id, MAP_TYPEID | ARR_TYPEID | STR_TYPEID | BINT_TYPEID);
		if header & 0b111 == 0b101 && !use_len {
			decode_vuint(data, ind)?;
		};

		let value = decode_field_value(data, ind, &field.typeid, true, provider)?;
		map.insert(name, value);
	}

	if required != 0 {
		return None;
	}
	Some(Value::Map(Box::new(map)))
}
pub fn skip_field(data: &[u8], ind: &mut usize, header: u64) -> Option<()> {
	match header & 0b111 {
		0b000 => *ind += 1,                                 // b8
		0b001 => *ind += 2,                                 // b16
		0b010 => *ind += 4,                                 // b32
		0b011 => *ind += 8,                                 // b64
		0b100 => (decode_vuint(data, ind)?, ()).1,          // vuint
		0b101 => *ind += decode_vuint(data, ind)? as usize, // len field
		_ => return None,                                   // reserved
	};
	Some(())
}
