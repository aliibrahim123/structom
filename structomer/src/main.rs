use std::{
	collections::HashMap,
	fs::{self, create_dir_all, write},
	io::{self, Write, stdin, stdout},
	path::absolute,
	str::FromStr,
};

use clap::{Parser, ValueEnum};
use serde::Serialize;
use serde_json::{Map as JsonMap, Serializer, Value as JsonValue, json, ser::PrettyFormatter};
use structom::{
	DeclProvider, FSProvider, Key, StringifyOptions, Value, VoidProvider, decode, encode, parse,
	stringify,
};

#[derive(ValueEnum, Clone, Copy, Debug)]
enum Type {
	Obj,
	Bin,
	JSON,
}

/// generate serialization code for structom declerations
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
	/// input file path, if not provided read from stdin
	#[arg(short, long)]
	input: Option<String>,

	/// output file path, if not provided write to stdout
	#[arg(short, long)]
	output: Option<String>,

	/// input type
	#[arg(short, long, default_value = "obj")]
	from: Type,

	/// output type
	#[arg(short, long, default_value = "obj")]
	to: Type,

	/// declerations directory
	#[arg(short, long)]
	declerations: Option<String>,
}

fn main() -> Result<(), String> {
	let Args { input, output, from, to, declerations } = Args::parse();

	let provider: Box<dyn DeclProvider> = match declerations {
		Some(path) => Box::new(
			FSProvider::new(&path).map_err(|_| format!("unable to read directory {path}"))?,
		),
		None => Box::new(VoidProvider {}),
	};

	let input = match input {
		Some(path) => {
			fs::read_to_string(&path).map_err(|_| format!("unable to read file {path}"))?
		}
		None => io::read_to_string(stdin()).map_err(|_| "unable to read from stdin")?,
	};

	let source = match from {
		Type::Obj => parse(&input, &Default::default(), &*provider).map_err(|v| v.to_string())?,
		Type::Bin => decode(&input.as_bytes(), &*provider).ok_or("invalid binary data")?,
		Type::JSON => from_json(JsonValue::from_str(&input).map_err(|e| e.to_string())?),
	};

	let result = match to {
		Type::Obj => {
			stringify(&source, &StringifyOptions { ident: "\t", ..Default::default() }).into_bytes()
		}
		Type::Bin => encode(&source),
		Type::JSON => {
			let mut buf = Vec::new();
			let mut ser = Serializer::with_formatter(&mut buf, PrettyFormatter::with_indent(b"\t"));
			to_json(&source).serialize(&mut ser).map_err(|e| e.to_string())?;
			buf
		}
	};

	match output {
		Some(path) => {
			let res_path = absolute(&path).map_err(|_| format!("unable to read file {path}"))?;
			create_dir_all(res_path.parent().unwrap())
				.map_err(|_| format!("unable to write file {path}"))?;
			write(&res_path, result).map_err(|_| format!("unable to write file {path}"))?;
		}
		None => stdout().write_all(&result).map_err(|_| "unable to write to stdout")?,
	}

	Ok(())
}

pub fn to_json(value: &Value) -> JsonValue {
	match value {
		Value::Bool(b) => json!(b),
		Value::Int(i) => json!(i),
		Value::Uint(u) => json!(u),
		Value::Float(f) => json!(f),
		Value::Str(s) => json!(s),
		Value::UnitVar(s) => json!(s),
		Value::BigInt(_) => JsonValue::Null,
		Value::Inst(d) => json!(d.to_rfc3339()),
		Value::Dur(_) | Value::UUID(_) => json!(value.to_string()),
		Value::Arr(els) => JsonValue::Array(els.iter().map(to_json).collect()),
		Value::Map(map) => {
			let mut jmap = JsonMap::new();
			for (key, value) in map.iter() {
				let key = match key {
					_ if key == Key::enum_variant_key() => "type".to_string(),
					Key::Str(str) => str.to_string(),
					_ => key.to_string(),
				};
				jmap.insert(key, to_json(value));
			}
			JsonValue::Object(jmap)
		}
	}
}

pub fn from_json(value: JsonValue) -> Value {
	match value {
		JsonValue::Bool(bool) => Value::Bool(bool),
		JsonValue::Number(nb) => {
			if let Some(nb) = nb.as_i64() {
				Value::Int(nb)
			} else if let Some(nb) = nb.as_u64() {
				Value::Uint(nb)
			} else {
				Value::Float(nb.as_f64().unwrap())
			}
		}
		JsonValue::String(s) => Value::Str(s),
		JsonValue::Array(arr) => Value::Arr(arr.into_iter().map(from_json).collect()),
		JsonValue::Object(obj) => {
			let mut map = HashMap::new();
			for (k, v) in obj {
				map.insert(Key::Str(k), from_json(v));
			}
			Value::Map(Box::new(map))
		}
		JsonValue::Null => Value::Str("null".to_string()),
	}
}
