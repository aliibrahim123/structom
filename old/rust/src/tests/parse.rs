use std::collections::HashMap;

use chrono::{DateTime, NaiveDate, TimeDelta};
use num_bigint::BigInt;

use crate::{
	DeclFile, DeclProvider, FixedSetProviderRef, Key, ParseError, Value, VoidProvider, parser,
};

fn parse(source: &str) -> Result<Value, ParseError> {
	parser::parse(source, &Default::default(), &VoidProvider {})
}
fn parse_with(source: &str, provider: &dyn DeclProvider) -> Result<Value, ParseError> {
	parser::parse(source, &Default::default(), provider)
}
fn parse_decl(source: &str, name: &str) -> Result<DeclFile, ParseError> {
	parser::parse_declaration_file(source, name.to_string(), &Default::default(), &VoidProvider {})
}

fn parse_typed(ty: &str, source: &str) -> Result<Value, ParseError> {
	parse(&format!("struct A {{ v: {ty} }} A {{ v: {source} }}"))
		.map(|v| v[&Key::from("v")].clone())
}

#[test]
fn core() {
	assert!(parse("").is_err());
	assert_eq!(parse("true"), Ok(Value::Bool(true)));
	assert_eq!(parse("false"), Ok(Value::Bool(false)));
	assert_eq!(parse("\"abc\""), Ok(Value::Str("abc".to_string())));
}

#[test]
fn int() {
	assert_eq!(parse("1"), Ok(Value::Uint(1)));
	assert_eq!(parse("-1"), Ok(Value::Int(-1)));

	assert_eq!(parse_typed("vuint", "1"), Ok(Value::Uint(1)));
	assert_eq!(parse_typed("vuint", "1",), Ok(Value::Uint(1)));
	assert_eq!(parse_typed("vint", "1",), Ok(Value::Uint(1)));
	assert_eq!(parse_typed("u8", "1",), Ok(Value::Uint(1)));
	assert_eq!(parse_typed("u16", "1",), Ok(Value::Uint(1)));
	assert_eq!(parse_typed("u32", "1",), Ok(Value::Uint(1)));
	assert_eq!(parse_typed("u64", "1",), Ok(Value::Uint(1)));
	assert_eq!(parse_typed("i64", "1",), Ok(Value::Uint(1)));
	assert_eq!(parse_typed("i32", "1",), Ok(Value::Uint(1)));
	assert_eq!(parse_typed("i16", "1",), Ok(Value::Uint(1)));
	assert_eq!(parse_typed("i8", "1",), Ok(Value::Uint(1)));

	assert_eq!(parse_typed("vuint", "+1"), Ok(Value::Uint(1)));
	assert!(parse_typed("vuint", "-1").is_err());
	assert!(parse_typed("u8", "256").is_err());

	assert_eq!(parse("123456789bint"), Ok(Value::BigInt(BigInt::from(123456789))));
	assert_eq!(parse_typed("bint", "123456789bint"), Ok(Value::BigInt(BigInt::from(123456789))));
	assert!(parse_typed("bint", "123456789").is_err());
	assert!(parse_typed("u8", "123456789bint").is_err());
}

#[test]
fn float() {
	assert_eq!(parse("1.0"), Ok(Value::Float(1.0)));
	assert_eq!(parse_typed("f32", "1.0"), Ok(Value::Float(1.0)));
	assert_eq!(parse_typed("f64", "1"), Ok(Value::Float(1.0)));

	assert!(parse("nan").unwrap().as_float().unwrap().is_nan());
	assert_eq!(parse("inf"), Ok(Value::Float(f64::INFINITY)));
	assert_eq!(parse("-inf"), Ok(Value::Float(f64::NEG_INFINITY)));

	assert!(parse_typed("u8", "1.0").is_err());
}

#[test]
fn uuid() {
	let uuid = [
		0x12, 0x34, 0x56, 0x78, 0x12, 0x34, 0x12, 0x34, 0x12, 0x34, 0x12, 0x34, 0x56, 0x78, 0x9a,
		0xbc,
	];
	assert_eq!(parse("uuid \"12345678-1234-1234-1234-123456789abc\""), Ok(Value::UUID(uuid)));
	let value = parse_typed("uuid", "uuid \"12345678-1234-1234-1234-123456789abc\"");
	assert_eq!(value, Ok(Value::UUID(uuid)));
	assert!(parse("uuid \"12345678-GGGG-1234-1234-123456789abc\"").is_err());
	assert!(parse_typed("uuid", "true").is_err());
}

#[test]
fn inst() {
	let inst = DateTime::parse_from_rfc3339("2026-02-02T20:04:02Z")
		.unwrap()
		.with_timezone(&chrono::Utc);
	let instn = DateTime::parse_from_rfc3339("2026-02-02T20:04:02.123456789Z")
		.unwrap()
		.with_timezone(&chrono::Utc);
	assert_eq!(parse("inst \"2026-02-02T20:04:02Z\""), Ok(Value::Inst(inst)));
	assert_eq!(parse("instN \"2026-02-02T20:04:02.123456789Z\""), Ok(Value::Inst(instn)));
	assert_eq!(parse_typed("instN", "inst \"2026-02-02T20:04:02Z\""), Ok(Value::Inst(inst)));
	assert!(parse("inst \"2026-02-02T20:04:02.1234Z\"").is_err());
	let inst = DateTime::parse_from_str("+10000-01-01 00:00:00+02:00", "%Y-%m-%d %H:%M:%S%:z");
	let value = parse("inst \"+10000-01-01 00:00:00+02:00\"");
	assert_eq!(value, Ok(Value::Inst(inst.unwrap().with_timezone(&chrono::Utc))));
	let inst = NaiveDate::parse_from_str("2026-02-02", "%Y-%m-%d")
		.unwrap()
		.and_hms_opt(0, 0, 0);
	assert_eq!(parse("inst \"2026-02-02\""), Ok(Value::Inst(inst.unwrap().and_utc())));
	assert!(parse_typed("inst", "true").is_err());
}

#[test]
fn dur() {
	assert_eq!(parse("dur \"0s\""), Ok(Value::Dur(TimeDelta::seconds(0))));
	assert_eq!(parse("dur \"10s 10ms\""), Ok(Value::Dur(TimeDelta::milliseconds(10010))));
	assert_eq!(parse("dur \"-10s 10ms\""), Ok(Value::Dur(-TimeDelta::milliseconds(10010))));
	let value = parse("dur \"1y 1mn 1d 1h 1m 1s 1ms 1us 1ns\"");
	assert_eq!(value, Ok(Value::Dur(TimeDelta::nanoseconds(34218061001001001))));
	assert!(parse("dur \"1000y\"").is_err());
	assert_eq!(parse("dur \"1000s 10ms\""), Ok(Value::Dur(TimeDelta::milliseconds(1000010))));
	assert!(parse("dur \"1y 1000s\"").is_err());
	assert!(parse("dur \"1mn 31d\"").is_err());
	assert_eq!(parse("dur \"1y 31d\""), Ok(Value::Dur(TimeDelta::days(365 + 31))));
	assert!(parse_typed("dur", "true").is_err());
	assert_eq!(parse("dur \"  \n  1s \t\n\n   \""), Ok(Value::Dur(TimeDelta::seconds(1))));
	assert!(parse("dur \"1\"").is_err());
	assert!(parse("dur \"1q\"").is_err());
}

#[test]
fn arr() {
	assert_eq!(parse("[]"), Ok(Value::Arr(vec![])));
	assert_eq!(parse("[1, 2, 3]"), Ok(Value::from(vec![1, 2, 3])));
	assert_eq!(parse("[1, true, \"a\"]"), Ok(Value::Arr(vec![1.into(), true.into(), "a".into()])));
	assert_eq!(parse("arr<u8> [1, 2, 3,]"), Ok(Value::from(vec![1, 2, 3])));
	assert_eq!(parse_typed("arr<u8>", "[1, 2, 3]"), Ok(Value::from(vec![1, 2, 3])));
	assert_eq!(parse("[[1, 2], 3]"), Ok(Value::Arr(vec![vec!(1, 2).into(), 3.into()])));
	let value = parse("arr<arr<u8>> [[1, 2], [3]]");
	assert_eq!(value, Ok(Value::Arr(vec![vec![1, 2].into(), vec![3].into()])));
	assert!(parse_typed("arr<u8>", "[1, 2000, 3]").is_err());
	assert!(parse("arr<u8> [1, 2000, 3]").is_err());
	assert!(parse("[1, 2, 3").is_err());
	assert!(parse_typed("arr<u8>", "true").is_err());
}

fn i(k: impl Into<Key>, v: impl Into<Value>) -> (Key, Value) {
	(k.into(), v.into())
}
#[test]
fn map() {
	assert_eq!(parse("{}"), Ok(Value::Map(Box::new(HashMap::new()))));
	assert_eq!(parse("{ a: 1 }"), Ok(Value::map_from([("a", 1)])));
	assert_eq!(parse("{ a: 1, b: 2, }"), Ok(Value::map_from([("a", 1), ("b", 2)])));
	assert_eq!(parse("{ a: 1, b: true, }"), Ok(Value::map_from([i("a", 1), i("b", true)])));
	assert_eq!(parse("{ \"a\": 1 }"), Ok(Value::map_from([i("a", 1)])));
	assert_eq!(parse("{ [1]: 1 }"), Ok(Value::map_from([i(1u8, 1)])));
	assert_eq!(parse("map<str, u8> { a: 1 }"), Ok(Value::map_from([("a", 1)])));
	assert_eq!(parse_typed("map<str, u8>", "{ a: 1 }"), Ok(Value::map_from([("a", 1)])));
	let value = parse("map<str, map<str, u8>> { a: { a: 1 } }");
	assert_eq!(value, Ok(Value::map_from([("a", Value::map_from([("a", 1)]))])));
	assert_eq!(parse("{ a: { a: 1 } }"), Ok(Value::map_from([("a", Value::map_from([("a", 1)]))])));
	assert!(parse_typed("map<u8, u8>", "{ a: 1 }").is_err());
	assert!(parse("map<u8, u8> { a: 1 }").is_err());
	assert!(parse("map<u8, u8> { [1]: \"a\" }").is_err());
	assert!(parse("{ a: 1, a: 2 }").is_err());
	assert!(parse("{ [[1]]: 1 }").is_err());
	assert!(parse("{ a: 1 ").is_err());
	assert!(parse("{ a 1 }").is_err());
	assert!(parse("{ a: 1 b: 2 }").is_err());
	assert!(parse("{ a: 1,, }").is_err());
	assert!(parse_typed("map<str, u8>", "true").is_err());
}

#[test]
fn val_struct() {
	assert_eq!(parse("struct A { a: u8 } A { a: 1 }"), Ok(Value::map_from([("a", 1)])));
	assert_eq!(
		parse("struct A { a: u8, b: bool } A { b: true, a: 1, }"),
		Ok(Value::map_from([i("a", 1), i("b", true)]))
	);
	assert_eq!(parse("struct A { a: u8 } A { \"a\": 1 }"), Ok(Value::map_from([("a", 1)])));
	assert_eq!(
		parse("struct A { a: u8 } [A { a: 1 }]"),
		Ok(Value::Arr(vec![Value::map_from([("a", 1)])]))
	);
	assert_eq!(parse("struct A { a?: u8 } A { a: 1 }"), Ok(Value::map_from([("a", 1)])));
	assert_eq!(parse("struct A { a?: u8 } A {}"), Ok(Value::Map(Box::default())));
	assert_eq!(
		parse("struct A { a: u8 } struct B { v: A } B { v: A { a: 1 } }"),
		Ok(Value::map_from([("v", Value::map_from([("a", 1)]))]))
	);
	assert_eq!(
		parse("struct A { a: u8 } struct B { v: A } B { v: { a: 1 } }"),
		Ok(Value::map_from([("v", Value::map_from([("a", 1)]))]))
	);
	assert_eq!(
		parse("struct A { a: u8 } arr<A> [{ a: 1 }]"),
		Ok(Value::Arr(vec![Value::map_from([("a", 1)])]))
	);
	assert_eq!(
		parse("struct A { a: arr<u8>, b: map<str, u8> } A { a: [1], b: { a: 1 } }"),
		Ok(Value::map_from([i("a", vec![1]), i("b", Value::map_from([("a", 1)]))]))
	);
	assert_eq!(
		parse("struct A { a: struct { v: u8 } } A { a: { v: 1 } }"),
		Ok(Value::map_from([("a", Value::map_from([("v", 1)]))]))
	);

	let file = parse_decl("struct A { a: u8 }", "file1").unwrap();
	let provider = FixedSetProviderRef::new(&[&file]);
	assert_eq!(
		parse_with("import \"file1\" A { a: 1 }", &provider),
		Ok(Value::map_from([("a", 1)]))
	);
	assert_eq!(
		parse_with("import \"file1\" as file1 file1.A { a: 1 }", &provider),
		Ok(Value::map_from([("a", 1)]))
	);

	assert!(parse("struct A { a: u8 } A { a: 1").is_err());
	assert!(parse("struct A { a: u8 } A { a: 1, a: 2 }").is_err());
	assert!(parse("struct A { a: u8 } A { a: 1, b: 2 }").is_err());
	assert!(parse("struct A { a: u8 } A {}").is_err());
	assert!(parse("struct A { a: u8 } A { a 1 }").is_err());
	assert!(parse("struct A { a: u8 } struct B { a: u8 } B { a: A { a: 1 } }").is_err());
	assert!(parse("struct A { a: u8 } struct B { a: A } B { a: 1 }").is_err());
}

#[test]
fn val_enum() {
	assert_eq!(parse("enum A { A, B, C } A.A"), Ok(Value::UnitVar("A".into())));
	assert_eq!(parse("enum A { A, B, C } A.B"), Ok(Value::UnitVar("B".into())));
	assert_eq!(
		parse("enum A { A { v: u8 } } A.A { v: 1 }"),
		Ok(Value::map_from([i(Key::enum_variant_key().clone(), "A"), i("v", 1)]))
	);
	assert_eq!(
		parse("enum A { B, C } struct B { v: A } B { v: A.B }"),
		Ok(Value::map_from([("v", Value::UnitVar("B".into()))]))
	);
	assert_eq!(
		parse("enum A { A, B, C } struct B { v: A } B { v: B }"),
		Ok(Value::map_from([("v", Value::UnitVar("B".into()))]))
	);
	assert_eq!(
		parse("enum A { A { v: u8 } } struct B { v: A } B { v: A { v: 1 } }"),
		Ok(Value::map_from([(
			"v",
			Value::map_from([i(Key::enum_variant_key().clone(), "A"), i("v", 1)])
		)]))
	);
	assert_eq!(
		parse("enum A { A, B, C } arr<A> [A, B, C]"),
		Ok(Value::from(vec![
			Value::UnitVar("A".into()),
			Value::UnitVar("B".into()),
			Value::UnitVar("C".into())
		]))
	);
	assert_eq!(
		parse("struct A { a: enum { A, B, C } } A { a: A }"),
		Ok(Value::map_from([("a", Value::UnitVar("A".into()))]))
	);

	let file = parse_decl("enum A { B, C } struct B { v: A }", "file1").unwrap();
	let provider = FixedSetProviderRef::new(&[&file]);
	assert_eq!(parse_with("import \"file1\" A.B", &provider), Ok(Value::UnitVar("B".into())));
	assert_eq!(
		parse_with("import \"file1\" as file1 file1.A.C", &provider),
		Ok(Value::UnitVar("C".into()))
	);
	assert_eq!(
		parse_with("import \"file1\" as file1 file1.B { v: B }", &provider),
		Ok(Value::map_from([("v", Value::UnitVar("B".into()))]))
	);
	assert_eq!(
		parse_with("import \"file1\" as file1 file1.B { v: file1.A.B }", &provider),
		Ok(Value::map_from([("v", Value::UnitVar("B".into()))]))
	);

	assert!(parse("enum A { A, B, C } A.D").is_err());
	assert!(parse("enum A { B { v: u8 } } A.B {}").is_err());
	assert!(parse("enum A { B, C } struct B { v: A } B { v: 1 }").is_err());
	assert!(parse("enum A { B, C } struct B { v: u8 } B { v: A.B }").is_err());
}
