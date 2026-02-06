use std::collections::HashMap;

use chrono::{DateTime, TimeDelta};
use num_bigint::BigInt;

use crate::{
	FixedSetProvider, FixedSetProviderRef, Key, Value, VoidProvider,
	encoding::{self, *},
	parse_declaration_file,
};

macro_rules! encode {
	($buf:ident => $exp:expr) => {{
		let mut $buf = Vec::new();
		$exp;
		$buf
	}};
}

fn decode(buf: &[u8]) -> Option<Value> {
	encoding::decode(buf, &VoidProvider {})
}

#[test]
fn nbs() {
	assert_eq!(encode!(buf => encode_u8(&mut buf, 1)), [1]);
	assert_eq!(encode!(buf => encode_u16(&mut buf, 1)), [1, 0]);
	assert_eq!(encode!(buf => encode_u32(&mut buf, 1)), [1, 0, 0, 0]);
	assert_eq!(encode!(buf => encode_u64(&mut buf, 1)), [1, 0, 0, 0, 0, 0, 0, 0]);

	assert_eq!(encode!(buf => encode_i8(&mut buf, 1)), [1]);
	assert_eq!(encode!(buf => encode_i16(&mut buf, 1)), [1, 0]);
	assert_eq!(encode!(buf => encode_i32(&mut buf, 1)), [1, 0, 0, 0]);
	assert_eq!(encode!(buf => encode_i64(&mut buf, 1)), [1, 0, 0, 0, 0, 0, 0, 0]);

	assert_eq!(decode_u8(&[1], &mut 0), Some(1));
	assert_eq!(decode_u16(&[1, 0], &mut 0), Some(1));
	assert_eq!(decode_u32(&[1, 0, 0, 0], &mut 0), Some(1));
	assert_eq!(decode_u64(&[1, 0, 0, 0, 0, 0, 0, 0], &mut 0), Some(1));

	assert_eq!(decode_i8(&[1], &mut 0), Some(1));
	assert_eq!(decode_i16(&[1, 0], &mut 0), Some(1));
	assert_eq!(decode_i32(&[1, 0, 0, 0], &mut 0), Some(1));
	assert_eq!(decode_i64(&[1, 0, 0, 0, 0, 0, 0, 0], &mut 0), Some(1));

	assert_eq!(encode!(buf => encode_f32(&mut buf, 1.5)), [0, 0, 192, 63]);
	assert_eq!(encode!(buf => encode_f64(&mut buf, 1.5)), [0, 0, 0, 0, 0, 0, 248, 63]);

	assert_eq!(decode_f32(&[0, 0, 192, 63], &mut 0), Some(1.5));
	assert_eq!(decode_f64(&[0, 0, 0, 0, 0, 0, 248, 63], &mut 0), Some(1.5));
}

#[test]
fn vint() {
	assert_eq!(encode!(buf => encode_vuint(&mut buf, 1)), [1]);
	assert_eq!(encode!(buf => encode_vuint(&mut buf, 1000)), [232, 7]);
	assert_eq!(encode!(buf => encode_vuint(&mut buf, 127)), [127]);
	assert_eq!(encode!(buf => encode_vuint(&mut buf, 16384)), [128, 128, 1]);

	assert_eq!(decode_vuint(&[1], &mut 0), Some(1));
	assert_eq!(decode_vuint(&[232, 7], &mut 0), Some(1000));
	assert_eq!(decode_vuint(&[127], &mut 0), Some(127));
	assert_eq!(decode_vuint(&[128, 128, 1], &mut 0), Some(16384));

	assert_eq!(encode!(buf => encode_vint(&mut buf, 1)), [1]);
	assert_eq!(encode!(buf => encode_vint(&mut buf, -1)), [127]);
	assert_eq!(encode!(buf => encode_vint(&mut buf, -64)), [64]);
	assert_eq!(encode!(buf => encode_vint(&mut buf, -16384)), [128, 128, 127]);

	assert_eq!(decode_vint(&[1], &mut 0), Some(1));
	assert_eq!(decode_vint(&[127], &mut 0), Some(-1));
	assert_eq!(decode_vint(&[64], &mut 0), Some(-64));
	assert_eq!(decode_vint(&[128, 128, 127], &mut 0), Some(-16384));

	let mut buf = vec![1, 2, 3, 0, 0, 6, 7];
	encode_vuint_pre_aloc(&mut buf, 128, 3, 2);
	assert_eq!(buf, [1, 2, 3, 128, 1, 6, 7]);

	let mut buf = vec![1, 2, 3, 0, 0, 0, 7, 8];
	encode_vuint_pre_aloc(&mut buf, 1, 3, 3);
	assert_eq!(buf, [1, 2, 3, 129, 128, 0, 7, 8]);

	let mut buf = vec![1, 2, 3, 0, 0, 6, 7];
	encode_vuint_pre_aloc(&mut buf, 2097152, 3, 2);
	assert_eq!(buf, [1, 2, 3, 128, 128, 128, 1, 6, 7]);

	assert_eq!(encode!(buf => encode_bint(&mut buf, &BigInt::from(1))), [1, 1]);
	assert_eq!(encode!(buf => encode_bint(&mut buf, &BigInt::from(65536))), [3, 0, 0, 1]);
	assert_eq!(encode!(buf => encode_bint(&mut buf, &BigInt::from(-1))), [1, 255]);

	assert_eq!(decode_bint(&[1, 1], &mut 0), Some(BigInt::from(1)));
	assert_eq!(decode_bint(&[3, 0, 0, 1], &mut 0), Some(BigInt::from(65536)));
	assert_eq!(decode_bint(&[1, 255], &mut 0), Some(BigInt::from(-1)));
}

#[test]
fn general() {
	assert_eq!(encode!(buf => encode_bool(&mut buf, true)), [1]);
	assert_eq!(encode!(buf => encode_bool(&mut buf, false)), [0]);

	assert_eq!(decode_bool(&[1], &mut 0), Some(true));
	assert_eq!(decode_bool(&[0], &mut 0), Some(false));
	assert_eq!(decode_bool(&[2], &mut 0), Some(true));

	assert_eq!(encode!(buf => encode_str(&mut buf, "abc")), [3, b'a', b'b', b'c']);
	let str = "a".repeat(128);
	let mut buf = vec![128, 1];
	buf.extend_from_slice(&[b'a'; 128]);
	assert_eq!(encode!(buf => encode_str(&mut buf, &str)), buf);

	assert_eq!(decode_str(&[3, b'a', b'b', b'c'], &mut 0), Some("abc".to_string()));
	assert_eq!(decode_str(&buf, &mut 0), Some(str));
}

#[test]
fn arr() {
	assert_eq!(
		encode!(buf => encode_arr(&mut buf, &[1,2,3], false, |buf, val| encode_u8(buf, *val))),
		[3, 1, 2, 3]
	);
	assert_eq!(decode_arr(&[3, 1, 2, 3], &mut 0, false, decode_u8), Some(vec![1, 2, 3]));

	assert_eq!(
		encode!(buf => encode_arr(&mut buf, &[1,2,3], false, |buf, val| encode_u16(buf, *val))),
		[3, 1, 0, 2, 0, 3, 0]
	);
	assert_eq!(decode_arr(&[3, 1, 0, 2, 0, 3, 0], &mut 0, false, decode_u16), Some(vec![1, 2, 3]));

	assert_eq!(
		encode!(buf => encode_arr(&mut buf, &[1,2,3], true, |buf, val| encode_u16(buf, *val))),
		[1, 0, 2, 0, 3, 0]
	);
	assert_eq!(decode_arr(&[6, 1, 0, 2, 0, 3, 0], &mut 0, true, decode_u16), Some(vec![1, 2, 3]));
}

#[test]
fn map() {
	let map = HashMap::from([(1, 2), (3, 4), (5, 6)]);
	let buf = encode!(buf => encode_map(&mut buf, &map, false, |buf, val| encode_u8(buf, *val), |buf, val| encode_u8(buf, *val)));
	assert!(buf.starts_with(&[3]));
	assert!(matches!(buf[1..3], [1, 2] | [3, 4] | [5, 6]));
	assert!(matches!(buf[3..5], [1, 2] | [3, 4] | [5, 6]));
	assert!(matches!(buf[5..7], [1, 2] | [3, 4] | [5, 6]));

	let dec = &decode_map(&[3, 1, 2, 3, 4, 5, 6], &mut 0, false, decode_u8, decode_u8).unwrap();
	assert_eq!(dec, &map);

	let map = HashMap::from([(1, 2), (3, 4), (5, 6)]);
	let buf = encode!(buf => encode_map(&mut buf, &map, false, |buf, val| encode_vuint(buf, *val), |buf, val| encode_u16(buf, *val)));
	assert!(buf.starts_with(&[3]));
	assert!(matches!(buf[1..4], [1, 2, 0] | [3, 4, 0] | [5, 6, 0]));
	assert!(matches!(buf[4..7], [1, 2, 0] | [3, 4, 0] | [5, 6, 0]));
	assert!(matches!(buf[7..10], [1, 2, 0] | [3, 4, 0] | [5, 6, 0]));

	let dec = &decode_map(&[3, 1, 2, 0, 3, 4, 0, 5, 6, 0], &mut 0, false, decode_vuint, decode_u16)
		.unwrap();
	assert_eq!(dec, &map);

	let buf = encode!(buf => encode_map(&mut buf, &map, true, |buf, val| encode_vuint(buf, *val), |buf, val| encode_u16(buf, *val)));
	assert!(matches!(buf[0..3], [1, 2, 0] | [3, 4, 0] | [5, 6, 0]));
	assert!(matches!(buf[3..6], [1, 2, 0] | [3, 4, 0] | [5, 6, 0]));
	assert!(matches!(buf[6..9], [1, 2, 0] | [3, 4, 0] | [5, 6, 0]));

	let dec = &decode_map(&[9, 1, 2, 0, 3, 4, 0, 5, 6, 0], &mut 0, true, decode_vuint, decode_u16)
		.unwrap();
	assert_eq!(dec, &map);
}

#[test]
fn any() {
	assert_eq!(encode(&Value::from(1u64)), &[0, VUINT_TYPEID, 1]);
	assert_eq!(encode(&Value::from(1i64)), &[0, VINT_TYPEID, 1]);
	assert_eq!(encode(&Value::from(true)), &[0, BOOL_TYPEID, 1]);
	assert_eq!(encode(&Value::from(1.5)), &[0, F64_TYPEID, 0, 0, 0, 0, 0, 0, 248, 63]);
	assert_eq!(encode(&Value::from("abc")), &[0, STR_TYPEID, 3, b'a', b'b', b'c']);
	assert_eq!(encode(&Value::from(BigInt::from(256))), &[0, BINT_TYPEID, 2, 0, 1]);
	assert_eq!(
		encode(&Value::from(DateTime::from_timestamp(1, 2).unwrap())),
		&[0, INSTN_TYPEID, 232, 3, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0]
	);
	assert_eq!(
		encode(&Value::from(TimeDelta::nanoseconds(123))),
		&[0, DUR_TYPEID, 123, 0, 0, 0, 0, 0, 0, 0]
	);
	assert_eq!(
		encode(&Value::from([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16])),
		&[0, UUID_TYPEID, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]
	);

	assert_eq!(decode(&[0, VUINT_TYPEID, 1]), Some(Value::from(1)));
	assert_eq!(decode(&[0, VINT_TYPEID, 1]), Some(Value::from(1)));
	assert_eq!(decode(&[0, BOOL_TYPEID, 1]), Some(Value::from(true)));
	assert_eq!(decode(&[0, F64_TYPEID, 0, 0, 0, 0, 0, 0, 248, 63]), Some(Value::from(1.5)));
	assert_eq!(decode(&[0, STR_TYPEID, 3, b'a', b'b', b'c']), Some(Value::from("abc")));
	assert_eq!(decode(&[0, BINT_TYPEID, 2, 0, 1]), Some(Value::from(BigInt::from(256))));
	assert_eq!(
		decode(&[0, INSTN_TYPEID, 232, 3, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0]),
		Some(Value::from(DateTime::from_timestamp(1, 2).unwrap()))
	);
	assert_eq!(
		decode(&[0, DUR_TYPEID, 123, 0, 0, 0, 0, 0, 0, 0]),
		Some(Value::from(TimeDelta::nanoseconds(123)))
	);
	assert_eq!(
		decode(&[0, UUID_TYPEID, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]),
		Some(Value::from([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]))
	);
}
#[test]
fn any_cont() {
	assert_eq!(encode(&Value::from(vec![1, 2, 3])), &[0, ARR_TYPEID, VINT_TYPEID, 3, 1, 2, 3]);
	#[rustfmt::skip]
	let arr = &[
		0, ARR_TYPEID, ANY_TYPEID, 3, VINT_TYPEID, 1, BOOL_TYPEID, 1, STR_TYPEID, 3, b'a',
		b'b', b'c'
	];
	assert_eq!(encode(&Value::Arr(vec![1.into(), true.into(), "abc".into()])), arr);

	let buf = encode(&Value::map_from([(1, 2), (3, 4)]));
	assert!(buf.starts_with(&[0, MAP_TYPEID, VINT_TYPEID, VINT_TYPEID, 2]));
	assert!(matches!(buf[5..7], [1, 2] | [3, 4]));
	assert!(matches!(buf[7..9], [1, 2] | [3, 4]));

	let buf = encode(&Value::map_from([
		(Key::from(1), Value::from(2)),
		(Key::from(true), Value::from(2)),
	]));
	assert!(buf.starts_with(&[0, MAP_TYPEID, ANY_TYPEID, VINT_TYPEID, 2]));
	assert!(matches!(buf[5..8], [VINT_TYPEID, 1, 2] | [BOOL_TYPEID, 1, 2]));
	assert!(matches!(buf[8..11], [VINT_TYPEID, 1, 2] | [BOOL_TYPEID, 1, 2]));

	assert_eq!(decode(&[0, ARR_TYPEID, VINT_TYPEID, 3, 1, 2, 3]), Some(Value::from(vec![1, 2, 3])));
	assert_eq!(decode(arr), Some(Value::Arr(vec![1.into(), true.into(), "abc".into()])));

	let buf = decode(&[0, MAP_TYPEID, VINT_TYPEID, VINT_TYPEID, 2, 1, 2, 3, 4]).unwrap();
	assert_eq!(buf, Value::map_from([(1, 2), (3, 4)]));

	let buf =
		decode(&[0, MAP_TYPEID, ANY_TYPEID, VINT_TYPEID, 2, VINT_TYPEID, 1, 2, BOOL_TYPEID, 1, 2]);
	let map = Value::map_from([(Key::from(1), Value::from(2)), (Key::from(true), Value::from(2))]);
	assert_eq!(buf, Some(map));
}

#[test]
fn item() {
	let file = "struct A { a: vuint, b: map<str, any> } enum B { A, B { v?: A }, C }";
	let file =
		parse_declaration_file(file, "test".to_string(), &Default::default(), &VoidProvider {});
	let provider = FixedSetProvider::new(vec![file.unwrap()]);

	#[rustfmt::skip]
	let buf = [
		4, b't', b'e', b's', b't', 1, 1, 1, 0b101, 17, 2, 0b100, 1, 0b1101, 12, 1, b'a', 
		VUINT_TYPEID, 1, 1, b'b', ARR_TYPEID, VUINT_TYPEID, 3, 1, 2, 3
	];
	let value_v_b = Value::map_from([
		(Key::from("a"), Value::from(1)),
		(Key::from("b"), Value::from(vec![1, 2, 3])),
	]);
	let value = Value::map_from([
		(Key::enum_variant_key().clone(), Value::from("B")),
		(
			Key::from("v"),
			Value::map_from([(Key::from("a"), Value::from(1)), (Key::from("b"), value_v_b)]),
		),
	]);
	assert_eq!(encoding::decode(&buf, &provider).unwrap(), value);

	#[rustfmt::skip]
	let buf = [
		4, b't', b'e', b's', b't', 1, 1, 6, 0b1000, 1, 0b1001, 1, 2, 0b1010, 1, 2, 3, 4,
		0b1011, 1, 2, 3, 4, 5, 6, 7, 8, 0b1100, 1, 0b1101, 3, 1, 2, 3
	];
	assert_eq!(
		encoding::decode(&buf, &provider).unwrap(),
		Value::map_from([(Key::enum_variant_key().clone(), "B")])
	);
}
