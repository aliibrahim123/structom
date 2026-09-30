use std::collections::HashMap;

use chrono::{DateTime, TimeDelta, Utc};
use num_bigint::{BigInt, Sign};

use crate::{Key, Value};

fn test_fixtures() -> (BigInt, DateTime<Utc>, TimeDelta, [u8; 16]) {
	let bigint = BigInt::new(Sign::Plus, vec![1, 2, 3]);
	let inst = DateTime::from_timestamp(1234, 5678).unwrap();
	let dur = TimeDelta::milliseconds(1234);
	let uuid = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
	(bigint, inst, dur, uuid)
}

#[test]
fn conv_primitive() {
	macro_rules! check_primitive {
		($input:expr, $variant:path, $accessor:ident, $cast_type:ty) => {
			let val = Value::from($input);
			let expected = $input;
			assert_eq!(val, $variant(expected));
			assert_eq!(val.$accessor(), Some(expected));
			assert_eq!(val.cast::<$cast_type>(), Some(expected));
		};
	}

	check_primitive!(true, Value::Bool, as_bool, bool);
	check_primitive!(1u64, Value::Uint, as_uint, u64);
	check_primitive!(1i64, Value::Int, as_int, i64);
	check_primitive!(1.0f64, Value::Float, as_float, f64);

	let val = Value::from("abc");
	assert_eq!(val, Value::Str("abc".to_string()));
	assert_eq!(val.as_str(), Some("abc"));
	assert_eq!(val.cast::<String>(), Some("abc".to_string()));

	assert_eq!(Key::from(1).as_bool(), None);
}

#[test]
fn conv_primitive_key() {
	macro_rules! check_primitive_key {
		($input:expr, $variant:path, $accessor:ident, $cast_type:ty) => {
			let key = Key::from($input);
			let expected = $input;
			assert_eq!(key, $variant(expected));
			assert_eq!(key.$accessor(), Some(expected));
			assert_eq!(key.cast::<$cast_type>(), Some(expected));
		};
	}

	check_primitive_key!(true, Key::Bool, as_bool, bool);
	check_primitive_key!(1u64, Key::Uint, as_uint, u64);
	check_primitive_key!(1i64, Key::Int, as_int, i64);

	let key = Key::from("abc");
	assert_eq!(key, Key::Str("abc".to_string()));
	assert_eq!(key.as_str(), Some("abc"));
	assert_eq!(key.cast::<String>(), Some("abc".to_string()));

	assert_eq!(Key::from(1).as_bool(), None);
}

#[test]
fn conv_rich() {
	let (bigint, inst, dur, uuid) = test_fixtures();

	macro_rules! check_rich {
		($input:expr, $variant:path, $accessor:ident) => {
			let val = Value::from($input.clone());
			assert_eq!(val, $variant($input.clone()));
			assert_eq!(val.$accessor().unwrap(), &$input);
			assert_eq!(val.cast(), Some($input.clone()));
		};
		($input:expr, $variant:path, $accessor:ident, copy) => {
			let val = Value::from($input);
			assert_eq!(val, $variant($input));
			assert_eq!(val.$accessor(), Some($input));
			assert_eq!(val.cast(), Some($input));
		};
	}

	check_rich!(bigint, Value::BigInt, as_bigint);
	check_rich!(inst, Value::Inst, as_inst, copy);
	check_rich!(dur, Value::Dur, as_dur, copy);
	check_rich!(uuid, Value::UUID, as_uuid, copy);
}

#[test]
fn conv_rich_key() {
	let (bigint, inst, dur, uuid) = test_fixtures();

	macro_rules! check_rich_key {
		($input:expr, $variant:path, $accessor:ident) => {
			let key = Key::from($input.clone());
			assert_eq!(key, $variant($input.clone()));
			assert_eq!(key.$accessor().unwrap(), &$input);
			assert_eq!(key.cast(), Some($input.clone()));
		};
		($input:expr, $variant:path, $accessor:ident, copy) => {
			let key = Key::from($input);
			assert_eq!(key, $variant($input));
			assert_eq!(key.$accessor(), Some($input));
			assert_eq!(key.cast(), Some($input));
		};
	}

	check_rich_key!(bigint, Key::BigInt, as_bigint);
	check_rich_key!(inst, Key::Inst, as_inst, copy);
	check_rich_key!(dur, Key::Dur, as_dur, copy);
	check_rich_key!(uuid, Key::UUID, as_uuid, copy);
}

#[test]
fn conv_arr() {
	let arr = vec![1u64, 2, 3];
	let value = Value::from(arr.clone());

	let expected_vec = vec![Value::Uint(1), Value::Uint(2), Value::Uint(3)];

	assert_eq!(value, Value::Arr(expected_vec.clone()));
	assert_eq!(value.as_slice(), Some(expected_vec.as_slice()));
	assert_eq!(value.cast(), Some(arr));
}

#[test]
fn conv_map() {
	let map =
		HashMap::from([("a".to_string(), 1u64), ("b".to_string(), 2u64), ("c".to_string(), 3u64)]);

	let map_value: HashMap<Key, Value> =
		map.iter().map(|(k, v)| (Key::from(k.as_str()), Value::from(*v))).collect();

	let value = Value::from(map.clone());

	assert_eq!(value.as_map(), Some(&map_value));
	assert_eq!(value, Value::Map(Box::new(map_value)));
	assert_eq!(value.cast(), Some(map));
}

#[test]
fn conv_int() {
	assert_eq!(Value::from(1u64).cast::<u32>(), Some(1u32));
	assert_eq!(Value::from(1u64).cast::<i16>(), Some(1i16));
	assert_eq!(Value::from(1u8), Value::Uint(1));
	assert_eq!(Value::from(1i32), Value::Uint(1));

	assert_eq!(Value::from(1i64).cast::<i32>(), Some(1i32));
	assert_eq!(Value::from(1i64).cast::<u16>(), Some(1u16));
	assert_eq!(Value::from(1i8), Value::Uint(1));
	assert_eq!(Value::from(1u32), Value::Uint(1));

	assert_eq!(Value::from(1f64).cast::<f32>(), Some(1f32));
	assert_eq!(Value::from(1f32), Value::Float(1f64));

	assert_eq!(Key::from(1u64).cast::<u32>(), Some(1u32));
	assert_eq!(Key::from(1u64).cast::<i16>(), Some(1i16));
	assert_eq!(Key::from(1u8), Key::Uint(1));
	assert_eq!(Key::from(1i32), Key::Uint(1));

	assert_eq!(Key::from(1i64).cast::<i32>(), Some(1i32));
	assert_eq!(Key::from(1i64).cast::<u16>(), Some(1u16));
	assert_eq!(Key::from(1i8), Key::Uint(1));
	assert_eq!(Key::from(1u32), Key::Uint(1));
}

#[test]
fn conv_extra() {
	assert_eq!(Value::Bool(true).into_key(), Some(Key::Bool(true)));
	assert!(Value::Float(1.0).into_key().is_none());

	assert_eq!(Value::from(true).into_inner(), Value::Bool(true));

	let complex_val = Value::from(HashMap::from([
		(Key::has_meta_key().clone(), Value::from(true)),
		(Key::inner_key().clone(), Value::from("abc")),
	]));

	assert_eq!(complex_val.into_inner(), Value::from("abc"));
}

#[test]
fn eq() {
	let (bigint, inst, dur, uuid) = test_fixtures();

	macro_rules! check_eq {
		($input:expr, $variant:path, $check_fn:ident) => {
			let val = Value::from($input.clone());
			assert_eq!(val, $variant($input.clone()));
			assert!(val.$check_fn());
			assert_eq!(val, $input);
		};
	}

	check_eq!(true, Value::Bool, is_bool);
	check_eq!(1u64, Value::Uint, is_uint);
	check_eq!(1i64, Value::Int, is_int);
	check_eq!(1.0f64, Value::Float, is_float);
	check_eq!("a".to_string(), Value::Str, is_str);

	check_eq!(inst, Value::Inst, is_inst);
	check_eq!(dur, Value::Dur, is_dur);
	check_eq!(uuid, Value::UUID, is_uuid);
	check_eq!(bigint, Value::BigInt, is_bigint);

	assert_ne!(Value::Uint(1), Value::Bool(true));
	assert!(Value::Int(1).as_bool().is_none());
}

#[test]
fn eq_key() {
	let (bigint, inst, dur, uuid) = test_fixtures();

	macro_rules! check_eq_key {
		($input:expr, $variant:path, $check_fn:ident) => {
			let key = Key::from($input.clone());
			assert_eq!(key, $variant($input.clone()));
			assert!(key.$check_fn());
			assert_eq!(key, $input);
		};
	}

	check_eq_key!(true, Key::Bool, is_bool);
	check_eq_key!(1u64, Key::Uint, is_uint);
	check_eq_key!(1i64, Key::Int, is_int);
	check_eq_key!("a".to_string(), Key::Str, is_str);

	check_eq_key!(inst, Key::Inst, is_inst);
	check_eq_key!(dur, Key::Dur, is_dur);
	check_eq_key!(uuid, Key::UUID, is_uuid);
	check_eq_key!(bigint, Key::BigInt, is_bigint);

	assert_ne!(Key::Uint(1), Key::Bool(true));
}

#[test]
fn eq_int() {
	let v_u64 = Value::from(1u64);
	assert_eq!(v_u64, Value::Int(1));
	assert_eq!(v_u64, 1i64);
	assert_eq!(v_u64, 1u32);

	let v_i64 = Value::from(1i64);
	assert_eq!(v_i64, Value::Uint(1));
	assert_eq!(v_i64, 1u64);
	assert_eq!(v_i64, 1i32);

	let k_u64 = Key::from(1u64);
	assert_eq!(k_u64, Key::Int(1));
	assert_eq!(k_u64, 1i64);
	assert_eq!(k_u64, 1u32);

	let k_i64 = Key::from(1i64);
	assert_eq!(k_i64, Key::Uint(1));
	assert_eq!(k_i64, 1u64);
	assert_eq!(k_i64, 1i32);
}

#[test]
fn eq_bidir() {
	let (bigint, inst, dur, uuid) = test_fixtures();

	assert_eq!(Value::Bool(true), Key::Bool(true));
	assert_eq!(Value::Uint(1), Key::Uint(1));
	assert_eq!(Value::Int(1), Key::Int(1));
	assert_eq!(Value::Str("a".to_string()), Key::Str("a".to_string()));
	assert_eq!(Value::Inst(inst), Key::Inst(inst));
	assert_eq!(Value::Dur(dur), Key::Dur(dur));
	assert_eq!(Value::UUID(uuid), Key::UUID(uuid));
	assert_eq!(Value::BigInt(bigint.clone()), Key::BigInt(bigint));
}

#[test]
fn info() {
	let _enum = Value::from(HashMap::from([(Key::enum_variant_key().clone(), Value::from("abc"))]));
	assert!(_enum.is_enum());
	assert!(!Value::Int(1).is_enum());

	let meta_val = Value::from(HashMap::from([(Key::has_meta_key().clone(), Value::from(true))]));
	assert!(meta_val.has_meta());
	assert!(!Value::Int(1).has_meta());

	assert_eq!(_enum.enum_variant(), Some("abc"));
	assert_eq!(Value::UnitVar("abc".to_string()).enum_variant(), Some("abc"));
	assert_eq!(Value::Int(1).enum_variant(), None);
}

#[test]
fn index_arr() {
	let mut value = Value::from(vec![1, 2, 3]);
	assert_eq!(value[0], 1);

	value[1] = 4.into();
	*value.index_arr_mut(2).unwrap() = 5.into();

	assert_eq!(value.index_arr(0..2), Value::from(vec![1, 4]).as_slice());
}

#[test]
fn index_map() {
	let mut map = Value::from(HashMap::from([
		(Key::from("a"), Value::from(1)),
		(Key::from("b"), Value::from(2)),
		(Key::from("c"), Value::from(3)),
	]));

	assert_eq!(map[&Key::from("a")], 1);

	map[&Key::from("d")] = 4.into();
	*map.index_map_mut(&Key::from("b")).unwrap() = 5.into();

	assert_eq!(map.index_map(&Key::from("e")), None);
}
