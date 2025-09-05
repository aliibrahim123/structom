use std::mem::discriminant;

use crate::{
	Key, Value,
	builtins::*,
	encoding::{
		decode_arr, decode_bool, decode_dur, decode_f32, decode_f64, decode_i8, decode_i16,
		decode_i32, decode_i64, decode_inst, decode_instN, decode_map, decode_str, decode_u8,
		decode_u8_arr, decode_u16, decode_u32, decode_u64, decode_uuid, decode_vint, decode_vuint,
		encode_arr, encode_bool, encode_dur, encode_f64, encode_instN, encode_map, encode_str,
		encode_u8_arr, encode_uuid, encode_vint, encode_vuint,
	},
};

macro_rules! encode_typeid_commons {
	($enum:ident, $value:ident, $data:ident) => {
		match $value {
			$enum::Bool(_) => $data.push(BOOL_TYPEID),
			$enum::Uint(_) => $data.push(VUINT_TYPEID),
			$enum::Int(_) => $data.push(VINT_TYPEID),
			$enum::BigInt(_) => $data.push(BINT_TYPEID),
			$enum::Str(_) => $data.push(STR_TYPEID),
			$enum::Inst(_) => $data.push(INSTN_TYPEID),
			$enum::Dur(_) => $data.push(DUR_TYPEID),
			$enum::UUID(_) => $data.push(UUID_TYPEID),
			_ => (),
		}
	};
}
macro_rules! encode_value_commons {
	($enum:ident, $value:ident, $data:ident) => {
		match $value {
			$enum::Bool(b) => encode_bool($data, *b),
			$enum::Uint(nb) => encode_vuint($data, *nb),
			$enum::Int(nb) => encode_vint($data, *nb),
			$enum::BigInt(nb) => encode_u8_arr($data, nb, false),
			$enum::Str(str) => encode_str($data, str, false),
			$enum::Inst(inst) => encode_instN($data, inst),
			$enum::Dur(dur) => encode_dur($data, dur),
			$enum::UUID(uuid) => encode_uuid($data, uuid),
			_ => (),
		}
	};
}

fn enocde_value_typeid(data: &mut Vec<u8>, value: &Value) {
	encode_typeid_commons!(Value, value, data);
	if value.is_float() {
		data.push(F64_TYPEID);
	}
}
// encode values of typed containers
fn encode_value(data: &mut Vec<u8>, value: &Value) {
	encode_value_commons!(Value, value, data);
	if let Value::Float(nb) = value {
		encode_f64(data, *nb);
	}
}
pub fn encode_any(data: &mut Vec<u8>, value: &Value) {
	encode_typeid_commons!(Value, value, data);
	encode_value_commons!(Value, value, data);

	match value {
		Value::Float(nb) => {
			data.push(F64_TYPEID);
			encode_f64(data, *nb);
		}
		Value::Arr(arr) => {
			data.push(ARR_TYPEID);
			// all elements of the same type (except arrays and maps)
			if let Some(first) = arr.first()
				&& arr.iter().all(|v| discriminant(v) == discriminant(first))
				&& !(first.is_array() || first.is_map())
			{
				enocde_value_typeid(data, first);
				encode_arr(data, arr, false, encode_value);
			} else {
				// it is arr<any>
				data.push(ANY_TYPEID);
				encode_arr(data, arr, false, encode_any);
			}
		}
		Value::Map(map) => {
			// typeid
			data.push(MAP_TYPEID);
			// are keys of the same type
			let key_encoder: fn(&mut Vec<u8>, &Key) = if let Some(first) = map.keys().next()
				&& map.keys().all(|key| discriminant(key) == discriminant(first))
			{
				encode_typeid_commons!(Key, first, data);
				|data, key| encode_value_commons!(Key, key, data)
			} else {
				// else keys are of type any
				data.push(ANY_TYPEID);
				|data, key| {
					encode_typeid_commons!(Key, key, data);
					encode_value_commons!(Key, key, data)
				}
			};
			// is values of the same type (except arrays and maps)
			let value_encoder = if let Some(first) = map.values().next()
				&& map.values().all(|value| discriminant(value) == discriminant(first))
				&& !(first.is_array() || first.is_map())
			{
				enocde_value_typeid(data, first);
				encode_value
			} else {
				// else values are of type any
				data.push(ANY_TYPEID);
				encode_any
			};
			encode_map(data, map, false, key_encoder, value_encoder);
		}
		_ => (),
	}
}

pub fn decode_any(data: &[u8], ind: &mut usize) -> Option<Value> {
	let typeid = *data.get(*ind)?;
	*ind += 1;
	decode_value(data, ind, typeid)
}
pub fn decode_value(data: &[u8], ind: &mut usize, id: u8) -> Option<Value> {
	match id {
		ANY_TYPEID => decode_any(data, ind),
		BOOL_TYPEID => Some(Value::Bool(decode_bool(data, ind)?)),

		U8_TYPEID => Some(Value::Uint(decode_u8(data, ind)? as u64)),
		U16_TYPEID => Some(Value::Uint(decode_u16(data, ind)? as u64)),
		U32_TYPEID => Some(Value::Uint(decode_u32(data, ind)? as u64)),
		U64_TYPEID => Some(Value::Uint(decode_u64(data, ind)?)),

		I8_TYPEID => Some(Value::Int(decode_i8(data, ind)? as i64)),
		I16_TYPEID => Some(Value::Int(decode_i16(data, ind)? as i64)),
		I32_TYPEID => Some(Value::Int(decode_i32(data, ind)? as i64)),
		I64_TYPEID => Some(Value::Int(decode_i64(data, ind)?)),

		F32_TYPEID => Some(Value::Float(decode_f32(data, ind)? as f64)),
		F64_TYPEID => Some(Value::Float(decode_f64(data, ind)?)),

		VUINT_TYPEID => Some(Value::Uint(decode_vuint(data, ind)?)),
		VINT_TYPEID => Some(Value::Int(decode_vint(data, ind)?)),
		BINT_TYPEID => Some(Value::BigInt(decode_u8_arr(data, ind)?)),

		STR_TYPEID => Some(Value::Str(decode_str(data, ind)?)),
		ARR_TYPEID => {
			let itemid = *data.get(*ind)?;
			*ind += 1;
			Some(Value::Arr(decode_arr(data, ind, false, |data, ind| {
				decode_value(data, ind, itemid)
			})?))
		}
		MAP_TYPEID => {
			let keyid = *data.get(*ind)?;
			let valueid = *data.get(*ind + 1)?;
			*ind += 2;
			Some(Value::Map(decode_map(
				data,
				ind,
				false,
				|data, ind| Some(decode_value(data, ind, keyid)?.into_key()),
				|data, ind| decode_value(data, ind, valueid),
			)?))
		}

		UUID_TYPEID => Some(Value::UUID(decode_uuid(data, ind)?)),
		INST_TYPEID => Some(Value::Inst(decode_inst(data, ind)?)),
		INSTN_TYPEID => Some(Value::Inst(decode_instN(data, ind)?)),
		DUR_TYPEID => Some(Value::Dur(decode_dur(data, ind)?)),
		_ => None,
	}
}
