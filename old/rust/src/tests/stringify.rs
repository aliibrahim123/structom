use chrono::{DateTime, TimeDelta};
use num_bigint::BigInt;

use crate::{Key, StringifyOptions, Value};

fn stringify(value: impl Into<Value>) -> String {
	crate::stringify(&value.into(), &Default::default())
}
fn stringify_pretty(value: impl Into<Value>) -> String {
	crate::stringify(&value.into(), &StringifyOptions { ident: "  ", ..Default::default() })
}
fn stringify_compact(value: impl Into<Value>) -> String {
	let options = StringifyOptions { ident: "  ", compat_items: true, ..Default::default() };
	crate::stringify(&value.into(), &options)
}
#[test]
fn core() {
	assert_eq!(stringify(true), "true");
	assert_eq!(stringify(false), "false");
	assert_eq!(stringify(123), "123");
	assert_eq!(stringify(-123), "-123");
	assert_eq!(stringify(1.5), "1.5");
	assert_eq!(stringify(f64::NAN), "nan");
	assert_eq!(stringify(f64::INFINITY), "inf");
	assert_eq!(stringify(f64::NEG_INFINITY), "-inf");
	assert_eq!(stringify(BigInt::from(12345678901234567890u64)), "12345678901234567890bint");
	assert_eq!(stringify("abc"), "\"abc\"");
	assert_eq!(stringify("abc \n\t\r\" 123"), "\"abc \n\t\r\\\" 123\"");
}

#[test]
fn rich() {
	let inst = DateTime::parse_from_rfc3339("2026-02-02T20:04:02Z").unwrap().to_utc();
	assert_eq!(stringify(inst), "inst \"2026-02-02T20:04:02Z\"");
	let inst = DateTime::parse_from_rfc3339("2026-02-02T20:04:02.123456789Z").unwrap().to_utc();
	assert_eq!(stringify(inst), "instN \"2026-02-02T20:04:02.123456789Z\"");
	let inst = DateTime::parse_from_str("+100000-01-01 02:00:00+02:00", "%Y-%m-%d %H:%M:%S%:z");
	assert_eq!(stringify(inst.unwrap().to_utc()), "inst \"+100000-01-01T00:00:00Z\"");

	let uuid = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];
	assert_eq!(stringify(uuid), "uuid \"00010203-0405-0607-0809-0a0b0c0d0e0f\"");

	let dur = TimeDelta::nanoseconds(12345678912);
	assert_eq!(stringify(dur), "dur \"12s 345ms 678us 912ns\"");
}

#[test]
fn arr() {
	assert_eq!(stringify(vec![1, 2, 3]), "[1,2,3]");
	assert_eq!(stringify(Value::Arr(vec![1.into(), true.into(), "a".into()])), "[1,true,\"a\"]");
	assert_eq!(stringify_pretty(vec![1, 2, 3]), "[\n  1,\n  2,\n  3,\n]");
	assert_eq!(stringify_compact(vec![1, 2, 3]), "[1, 2, 3]");
	let value = stringify_compact(Value::Arr(vec![1.into(), TimeDelta::nanoseconds(123).into()]));
	assert_eq!(value, "[\n  1,\n  dur \"123ns\",\n]");
	assert_eq!(stringify_compact(vec![TimeDelta::nanoseconds(123)]), "[dur \"123ns\"]");
}

fn i(key: impl Into<Key>, value: impl Into<Value>) -> (Key, Value) {
	(key.into(), value.into())
}
#[test]
fn obj() {
	assert_eq!(stringify(Value::map_from([("a", 1)])), "{a:1}");
	let value = stringify(Value::map_from([i("a", 1), i("b", true)]));
	assert!(value.contains("a:1"));
	assert!(value.contains("b:true"));
	assert_eq!(stringify(Value::map_from([("-123", 1)])), "{\"-123\":1}");
	assert_eq!(stringify(Value::map_from([(1, 1)])), "{[1]:1}");
	let value = stringify_pretty(Value::map_from([i("a", 1), i("b", true)]));
	assert!(value.contains("  a: 1,\n"));
	assert!(value.contains("  b: true,\n"));
	let value = stringify_compact(Value::map_from([i("a", 1), i("b", true)]));
	assert!(value.contains("a: 1"));
	assert!(value.contains("b: true"));
	let value =
		stringify_compact(Value::map_from([i("a", 1), i("b", TimeDelta::nanoseconds(123))]));
	assert!(value.contains("  a: 1,\n"));
	assert!(value.contains("  b: dur \"123ns\",\n"));
	let value =
		stringify_compact(Value::map_from([i("a", 1), i(TimeDelta::nanoseconds(123), "b")]));
	assert!(value.contains("  a: 1,\n"));
	assert!(value.contains("  [dur \"123ns\"]: \"b\",\n"));
	let value = stringify_compact(Value::map_from([i("b", TimeDelta::nanoseconds(123))]));
	assert_eq!(value, "{b: dur \"123ns\"}");
}

#[test]
fn enums() {
	assert_eq!(stringify(Value::UnitVar("A".to_string())), "A");
	let value = Value::map_from([i(Key::enum_variant_key().clone(), "A"), i("a", 1)]);
	assert_eq!(stringify(value.clone()), "A{a:1}");
	assert_eq!(stringify_pretty(value), "A{\n  a: 1,\n}");
}
#[test]
fn metadata() {
	let value = crate::stringify(
		&Value::map_from([
			i(Key::has_meta_key().clone(), true),
			i(Key::inner_key().clone(), 123),
			i("base", "16"),
			i("type", "int"),
		]),
		&StringifyOptions { metadata: true, ..Default::default() },
	);
	assert!(
		value == r#"@base("16") @type("int") 123"# || value == r#"@type("int") @base("16") 123"#
	);
}
