use std::{collections::HashMap, fmt::Write, sync::LazyLock};

use chrono::{DateTime, TimeDelta, Timelike, Utc};

use crate::{Key, Value};

#[derive(Debug, Default)]
pub struct StringifyOptions<'a> {
	pub metadata: bool,
	pub ident: &'a str,
	pub enums: bool,
}

pub fn stringify(value: &Value, options: &StringifyOptions) -> String {
	let mut result = "".to_string();

	str_value(value, &mut result, 0, options);

	return result;
}

// for commons between keys and values
macro_rules! str_commons {
	($ty:ident, $value:ident, $result:ident) => {
		match $value {
			$ty::Bool(v) => match v {
				true => $result.push_str("true"),
				false => $result.push_str("false"),
			},
			$ty::Uint(nb) => $result.push_str(nb.to_string().as_str()),
			$ty::Int(nb) => $result.push_str(nb.to_string().as_str()),
			$ty::BigInt(_) => $result.push_str("0bint"),
			$ty::Str(str) => {
				$result.push('\"');
				$result.push_str(str.replace('"', "\\\"").as_str());
				$result.push('\"');
			}
			$ty::Inst(inst) => str_inst(inst, $result),
			$ty::Dur(dur) => str_dur(dur, $result),
			$ty::UUID(uuid) => str_uuid(uuid, $result),
			_ => (),
		}
	};
}

pub fn str_value(value: &Value, result: &mut String, depth: usize, options: &StringifyOptions) {
	str_commons!(Value, value, result);

	match value {
		Value::Float(nb) => {
			// rust inf is similar to structom one
			if nb.is_nan() {
				result.push_str("nan")
			} else {
				result.push_str(nb.to_string().as_str())
			}
		}
		Value::Array(arr) => str_arr(arr, result, depth, options),
		Value::Map(map) => str_map(map, result, depth, options),
		_ => (),
	}
}

fn add_indent(result: &mut String, depth: usize, options: &StringifyOptions) {
	if options.ident.len() > 0 {
		result.push('\n');
		for _ in 0..depth {
			result.push_str(options.ident);
		}
	}
}

static META_KEY: LazyLock<Key> = LazyLock::new(|| Key::Str("$has_meta".to_string()));
static VALUE_KEY: LazyLock<Key> = LazyLock::new(|| Key::Str("value".to_string()));
static ENUM_VARIANT_KEY: LazyLock<Key> = LazyLock::new(|| Key::Str("$enum_variant".to_string()));
fn str_map(
	map: &HashMap<Key, Value>, result: &mut String, depth: usize, options: &StringifyOptions,
) {
	// case metadata
	if options.metadata && map.contains_key(&META_KEY) {
		// stringify metadata
		for (key, value) in map.iter() {
			if !matches!(key.as_str(), Some("$has_meta" | "value")) {
				result.push('@');
				result.push_str(key.as_str().unwrap());
				result.push('(');
				str_value(value, result, depth, options);
				result.push_str(") ");
			}
		}
		// stringify value
		str_value(&map[&VALUE_KEY], result, depth, options);
		return;
	}

	// case enums
	if options.enums && map.contains_key(&ENUM_VARIANT_KEY) {
		result.push_str(map.get(&ENUM_VARIANT_KEY).unwrap().as_str().unwrap());
	}

	result.push_str("{");

	// loop through map
	for (ind, (key, value)) in map.iter().enumerate() {
		// skip $enum_variant
		if key.as_str() == Some("$enum_variant") {
			continue;
		}

		// comma
		if ind != 0 {
			result.push_str(",");
		}

		// key
		add_indent(result, depth + 1, options);
		if let Key::Str(key) = key {
			result.push_str(key);
		} else {
			result.push('[');
			str_commons!(Key, key, result);
			result.push(']');
		}

		// colon
		result.push_str(":");
		if !options.ident.is_empty() {
			result.push(' ');
		}

		// value
		str_value(value, result, depth + 1, options);
	}

	add_indent(result, depth, options);
	result.push_str("}");
}

fn str_arr(arr: &Vec<Value>, result: &mut String, depth: usize, options: &StringifyOptions) {
	result.push_str("[");

	// loop through array
	for (ind, value) in arr.iter().enumerate() {
		// comma
		if ind != 0 {
			result.push_str(",");
		}

		add_indent(result, depth + 1, options);
		str_value(value, result, depth + 1, options);
	}

	add_indent(result, depth, options);
	result.push_str("]");
}

fn str_uuid(uuid: &[u8; 16], result: &mut String) {
	result.push_str("uuid \"");

	result.write_fmt(format_args!(
		"{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
		uuid[0], uuid[1], uuid[2], uuid[3], uuid[4], uuid[5], uuid[6], uuid[7], uuid[8],
		uuid[9], uuid[10], uuid[11], uuid[12], uuid[13], uuid[14], uuid[15])
	).unwrap();

	result.push('"');
}

fn str_inst(inst: &DateTime<Utc>, result: &mut String) {
	// type
	result.push_str(if inst.nanosecond() % 1000000 == 0 { "inst \"" } else { "instN \"" });
	// value source
	result.push_str(inst.to_rfc3339().as_str());
	result.push('"');
}

fn str_dur_part(value: i64, result: &mut String, unit: &str, range: i64, mutl: i64) {
	// get part
	let part = (value / mutl) % range;
	// skip if part is none
	if part == 0 {
		return;
	}

	result.push_str(part.to_string().as_str());
	result.push_str(unit);
}
fn str_dur(value: &TimeDelta, result: &mut String) {
	let mut value = value.num_nanoseconds().unwrap();
	result.push_str("dur \"");

	if value == 0 {
		result.push_str("0s\"");
		return;
	}
	// neg
	if value < 0 {
		result.push('-');

		// dont ask about this hack
		// in negative dur, the nano part is inverted, and the seconds are less than by 1
		value = value.abs();
		let nano_rem = value % 1000000000;
		//      remove nano          inc sec by 1   invert nano
		value = (value - nano_rem) + (1000000000) + (1000000000 - nano_rem);
	}

	// parts
	str_dur_part(value, result, "y ", 300, 31536000000000000);
	str_dur_part(value, result, "d ", 365, 86400000000000);
	str_dur_part(value, result, "h ", 24, 3600000000000);
	str_dur_part(value, result, "m ", 60, 60000000000);
	str_dur_part(value, result, "s ", 60, 1000000000);
	str_dur_part(value, result, "ms ", 1000, 1000000);
	str_dur_part(value, result, "us ", 1000, 1000);
	str_dur_part(value, result, "ns", 1000, 1);

	result.push('"');
}
