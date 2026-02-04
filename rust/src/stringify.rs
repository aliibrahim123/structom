use std::{collections::HashMap, fmt::Write};

use chrono::{DateTime, TimeDelta, Timelike, Utc};

use crate::{
	Key, Value,
	builtins::{D_AS_NS, H_AS_NS, M_AS_NS, MS_AS_NS, S_AS_NS, US_AS_NS, Y_AS_NS},
	parser::utils::StrExt,
};

/// options for [`stringify()`]
#[derive(Debug, Clone)]
pub struct StringifyOptions<'a> {
	/// whether to stringify metadata, default: `false`.
	pub metadata: bool,
	/// sequence of characters used as 1 depth of indentation, default: `""`.
	pub ident: &'a str,
	/// write maps and arr on single line if possible, default `false`
	pub compat_items: bool,
}

impl Default for StringifyOptions<'static> {
	fn default() -> Self {
		Self { metadata: false, ident: "", compat_items: false }
	}
}

/// stringify a [`Value`] into object notation.
///
/// for the other way see [`parse`](`crate::parse`)
///
/// ## example
/// ```
/// let value = Value::from(vec![1, 2, 3]);
///  assert_eq!(stringify(value, &StringifyOptions::default()), "[1, 2, 3]");
/// ```
pub fn stringify(value: &Value, options: &StringifyOptions) -> String {
	let mut result = "".to_string();
	str_value(value, &mut result, 0, options);
	return result;
}

/// for commons between keys and values
macro_rules! str_commons {
	($ty:ident, $value:ident, $result:expr) => {
		match $value {
			$ty::Bool(v) => match v {
				true => $result.push_str("true"),
				false => $result.push_str("false"),
			},
			$ty::Uint(nb) => $result.push_str(&nb.to_string()),
			$ty::Int(nb) => $result.push_str(&nb.to_string()),
			$ty::BigInt(nb) => write!($result, "{nb}bint").unwrap(),
			$ty::Str(str) => str_str(str, $result),
			$ty::Inst(inst) => str_inst(inst, $result),
			$ty::Dur(dur) => str_dur(dur, $result),
			$ty::UUID(uuid) => str_uuid(uuid, $result),
			#[allow(unreachable_patterns)]
			_ => (),
		}
	};
}
pub fn str_key(value: &Key) -> String {
	let mut result = String::new();
	str_commons!(Key, value, &mut result);
	return result;
}
pub fn str_value(value: &Value, result: &mut String, depth: usize, options: &StringifyOptions) {
	str_commons!(Value, value, result);

	match value {
		Value::Float(nb) => {
			// rust inf is similar to structom one
			if nb.is_nan() { result.push_str("nan") } else { result.push_str(&nb.to_string()) }
		}
		Value::Arr(arr) => str_arr(arr, result, depth, options),
		Value::Map(map) => str_map(map, result, depth, options),
		Value::UnitVar(var) => result.push_str(var),
		_ => unreachable!(),
	}
}

fn str_str(str: &str, result: &mut String) {
	result.push('\"');
	let mut last_ind = 0;
	while let Some(ind) = str.find_after('"', last_ind) {
		result.push_str(&str[last_ind..ind]);
		result.push_str("\\\"");
		last_ind = ind + 1;
	}
	result.push_str(&str[last_ind..]);
	result.push('\"');
}

/// add new line and ident till depth
fn add_indent(result: &mut String, depth: usize, options: &StringifyOptions) {
	if options.ident.len() > 0 {
		result.push('\n');
		for _ in 0..depth {
			result.push_str(options.ident);
		}
	}
}

fn do_ident(options: &StringifyOptions) -> bool {
	!options.ident.is_empty()
}

/// can be compacted
fn is_simple(value: &Value) -> bool {
	match value {
		Value::Bool(_) | Value::Uint(_) | Value::Int(_) | Value::Float(_) | Value::UnitVar(_) => {
			true
		}
		Value::Str(str) => str.len() <= 10,
		_ => false,
	}
}
/// can be compact
fn is_simple_key(key: &Key) -> bool {
	match key {
		Key::Bool(_) | Key::Uint(_) | Key::Int(_) => true,
		Key::Str(str) => str.len() <= 10,
		_ => false,
	}
}

fn str_map(
	map: &HashMap<Key, Value>, result: &mut String, depth: usize, options: &StringifyOptions,
) {
	// case metadata
	if options.metadata && map.contains_key(Key::has_meta_key()) {
		for (key, value) in map.iter() {
			if !matches!(key.as_str(), Some("$has_meta" | "value")) {
				result.push('@');
				result.push_str(key.as_str().unwrap());
				result.push('(');
				str_value(value, result, depth, options);
				result.push_str(") ");
			}
		}
		str_value(&map[Key::inner_key()], result, depth, options);
		return;
	}

	let is_enum = map.contains_key(Key::enum_variant_key());
	if is_enum {
		result.push_str(map[Key::enum_variant_key()].as_str().unwrap());
	}

	if map.len() == 0 {
		result.push_str("{}");
		return;
	}
	result.push_str("{");

	// compact: 4 simple fields or 1 field
	let is_one_key = map.len() == 1 || (is_enum && map.len() == 2);
	let compact = options.compat_items
		&& map.keys().all(is_simple_key)
		&& (is_one_key || (map.len() <= 4 && map.values().all(is_simple)));

	for (ind, (key, value)) in map.iter().enumerate() {
		if key.as_str() == Some("$enum_variant") {
			continue;
		}
		if ind != 0 {
			result.push_str(if do_ident(options) { ", " } else { "," });
		}
		if !compact {
			add_indent(result, depth + 1, options);
		}

		if let Key::Str(key) = key {
			// as identifier
			if key.chars().all(|c| matches!(c, 'a'..='z' | 'A'..='Z' | '0'..='9' | '_')) {
				result.push_str(key);
			// as str
			} else {
				str_str(key, result);
			}
		// as [key]
		} else {
			result.push('[');
			str_commons!(Key, key, result);
			result.push(']');
		}

		result.push_str(":");
		if do_ident(options) {
			result.push(' ');
		}
		str_value(value, result, depth + if compact { 0 } else { 1 }, options);
	}

	add_indent(result, depth, options);
	result.push_str("}");
}

fn str_arr(arr: &Vec<Value>, result: &mut String, depth: usize, options: &StringifyOptions) {
	if arr.len() == 0 {
		result.push_str("[]");
		return;
	}
	result.push_str("[");

	// compact: 8 simple item or 1 item
	let compact =
		options.compat_items && (arr.len() == 1 || (arr.len() <= 8 && arr.iter().all(is_simple)));

	for (ind, value) in arr.iter().enumerate() {
		if ind != 0 {
			result.push_str(if do_ident(options) { ", " } else { "," });
		}
		if compact {
			str_value(value, result, depth, options);
		} else {
			add_indent(result, depth + 1, options);
			str_value(value, result, depth + 1, options);
		}
	}

	add_indent(result, depth, options);
	result.push_str("]");
}

fn str_uuid(uuid: &[u8; 16], result: &mut String) {
	result.push_str("uuid \"");
	#[rustfmt::skip]
	write!(result,
		"{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
		uuid[0], uuid[1], uuid[2], uuid[3], uuid[4], uuid[5], uuid[6], uuid[7], uuid[8],
		uuid[9], uuid[10], uuid[11], uuid[12], uuid[13], uuid[14], uuid[15]
	).unwrap();
	result.push('"');
}

fn str_inst(inst: &DateTime<Utc>, result: &mut String) {
	result.push_str(if inst.nanosecond() % 1000000 == 0 { "inst \"" } else { "instN \"" });
	result.push_str(&inst.to_rfc3339());
	result.push('"');
}

fn str_dur_part(value: i64, result: &mut String, unit: &str, range: i64, mutl: i64) {
	let part = (value / mutl) % range;
	if part == 0 {
		return;
	}
	result.push_str(&part.to_string());
	result.push_str(unit);
}
fn str_dur(dur: &TimeDelta, result: &mut String) {
	result.push_str("dur \"");
	let value = dur.abs().num_nanoseconds().unwrap();

	if value == 0 {
		result.push_str("0s\"");
		return;
	}
	if dur.num_nanoseconds().unwrap().is_negative() {
		result.push('-');
	}

	str_dur_part(value, result, "y ", 290, Y_AS_NS as i64);
	str_dur_part(value, result, "d ", 365, D_AS_NS as i64);
	str_dur_part(value, result, "h ", 24, H_AS_NS as i64);
	str_dur_part(value, result, "m ", 60, M_AS_NS as i64);
	str_dur_part(value, result, "s ", 60, S_AS_NS as i64);
	str_dur_part(value, result, "ms ", 1000, MS_AS_NS as i64);
	str_dur_part(value, result, "us ", 1000, US_AS_NS as i64);
	str_dur_part(value, result, "ns", 1000, 1);
	result.push('"');
}
