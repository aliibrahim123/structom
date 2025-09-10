use crate::rust::Ctx;
use std::fmt::Write;

use structom::internal::{DeclItem, EnumVariant, Field};

/// generate value conversion functions
pub fn gen_value_conv(source: &mut String, ctx: &Ctx) {
	let Ctx { file, .. } = ctx;

	for (_, item) in &file.items {
		match item {
			DeclItem::Struct { .. } => {
				from_struct(source, item);

				to_struct(source, item);
			}
			DeclItem::Enum { .. } => {
				from_enum(source, item);

				to_enum(source, item);
			}
		}
	}
}

/// generate code for converting value to struct
fn to_struct(source: &mut String, item: &DeclItem) {
	let DeclItem::Struct { name, def, .. } = item else { unreachable!() };

	// header
	write!(source, "impl TryFrom<Value> for {name} {{\n").unwrap();
	source.push_str("\ttype Error = ();\n");
	write!(source, "\tfn try_from(value: Value) -> Result<{name}, ()> {{\n").unwrap();

	// extract inner map
	source.push_str("\t\tlet Value::Map(mut map) = value else { return Err(()); };\n");

	// extract fields
	let fields = def.fields.iter().filter_map(|f| f.as_ref()).collect::<Vec<_>>();
	for Field { name, is_optional, typeid, .. } in &fields {
		if typeid.is_any() {
			if *is_optional {
				write!(source, "\t\tlet f_{name} = map.remove(&\"{name}\".into());\n").unwrap();
			} else {
				write!(source, "\t\tlet f_{name} = map.remove(&\"{name}\".into()).ok_or(())?;\n")
					.unwrap();
			}
		} else if *is_optional {
			// extract field if found, fail if couldnt convert
			write!(source, "\t\tlet f_{name} = map.remove(&\"{name}\".into())").unwrap();
			source.push_str(".map(|v| v.try_into()).transpose()?;\n");
		} else {
			// extract field, fail if not found or couldnt convert
			write!(source, "\t\tlet f_{name} = map.remove(&\"{name}\".into())").unwrap();
			source.push_str(".ok_or(())?.try_into()?;\n");
		}
	}

	// if remains keys, fail
	source.push_str("\t\tif !map.is_empty() { return Err(()); }\n");

	// build struct
	write!(source, "\t\tOk({name} {{\n").unwrap();
	for chunk in fields.chunks(4) {
		source.push_str("\t\t\t");
		for Field { name, .. } in chunk {
			write!(source, "{name}: f_{name}, ").unwrap();
		}
		source.push('\n');
	}

	source.push_str("\t\t})\n\t}\n}\n");
}

/// generate code for converting struct to value
fn from_struct(source: &mut String, item: &DeclItem) {
	let DeclItem::Struct { name, def, .. } = item else { unreachable!() };

	// header
	write!(source, "impl Into<Value> for {name} {{\n").unwrap();
	source.push_str("\tfn into(self) -> Value {\n");
	source.push_str("\t\tlet mut map = HashMap::new();\n");

	// insert fields
	for field in def.fields.iter().filter_map(|f| f.as_ref()) {
		let Field { name, is_optional, .. } = field;
		if *is_optional {
			write!(source, "\t\tif let Some(value) = self.{name} {{\n").unwrap();
			write!(source, "\t\t\tmap.insert(Key::from(\"{name}\"), value.into());\n\t\t}}\n")
				.unwrap();
		} else {
			write!(source, "\t\tmap.insert(Key::from(\"{name}\"), self.{name}.into());\n").unwrap();
		}
	}

	source.push_str("\t\tValue::Map(map)\n\t}\n}\n");
}

/// generate code for converting value to enum
fn to_enum(source: &mut String, item: &DeclItem) {
	let DeclItem::Enum { name, variants, .. } = item else { unreachable!() };

	// header
	write!(source, "impl TryFrom<Value> for {name} {{\n").unwrap();
	source.push_str("\ttype Error = ();\n");
	write!(source, "\tfn try_from(value: Value) -> Result<{name}, ()> {{\n").unwrap();

	// split variants
	let (fieldless_vars, fieldfull_vars): (Vec<_>, Vec<_>) =
		variants.iter().filter_map(|v| v.as_ref()).partition(|v| v.def.is_none());

	// fieldless variants, written as strings
	if fieldless_vars.len() > 0 {
		source.push_str("\t\tif let Value::Str(v) = value {\n");
		// match based on variant
		source.push_str("\t\t\treturn match v.as_str() {\n");
		for EnumVariant { name: var_name, .. } in fieldless_vars {
			write!(source, "\t\t\t\t\"{var_name}\" => Ok({name}::{var_name}),\n").unwrap();
		}
		source.push_str("\t\t\t\t_ => Err(()),\n");
		source.push_str("\t\t\t};\n\t\t}\n");
	}

	// fieldfull variants
	if fieldfull_vars.len() > 0 {
		source.push_str("\t\tif let Some(var) = value.enum_variant() {\n");
		// match based on variant
		source.push_str("\t\t\treturn match var {\n");
		for EnumVariant { name: var_name, .. } in &fieldfull_vars {
			write!(source, "\t\t\t\t\"{var_name}\" => {name}_{var_name}_from_value(value),\n")
				.unwrap();
		}
		source.push_str("\t\t\t\t_ => Err(()),\n");
		source.push_str("\t\t\t};\n\t\t}\n");
	}

	// fail if other type
	source.push_str("\t\tErr(())\n\t}\n}\n");

	// generate conv fns for fieldfull variants
	for variant in fieldfull_vars {
		let EnumVariant { name: var_name, def: Some(def), .. } = variant else { unreachable!() };
		// header
		write!(source, "fn {name}_{var_name}_from_value(value: Value) -> Result<{name}, ()> {{\n")
			.unwrap();
		// extract inner map
		source.push_str("\tlet Value::Map(mut map) = value else { return Err(()); };\n");
		source.push_str("\tmap.remove(&\"$enum_variant\".into()).ok_or(())?;\n");

		// extract fields
		let fields = def.fields.iter().filter_map(|f| f.as_ref()).collect::<Vec<_>>();
		for Field { name, is_optional, typeid, .. } in &fields {
			if typeid.is_any() {
				if *is_optional {
					write!(source, "\tlet f_{name} = map.remove(&\"{name}\".into());\n").unwrap();
				} else {
					write!(source, "\tlet f_{name} = map.remove(&\"{name}\".into()).ok_or(())?;\n")
						.unwrap();
				}
			} else if *is_optional {
				// extract field if found, fail if couldnt convert
				write!(source, "\tlet f_{name} = map.remove(&\"{name}\".into())").unwrap();
				source.push_str(".map(|v| v.try_into()).transpose()?;\n");
			} else {
				// extract field, fail if not found or couldnt convert
				write!(source, "\tlet f_{name} = map.remove(&\"{name}\".into())").unwrap();
				source.push_str(".ok_or(())?.try_into()?;\n");
			}
		}

		// if remains keys, fail
		source.push_str("\tif !map.is_empty() { return Err(()); }\n");

		// build enum
		write!(source, "\tOk({name}::{var_name} {{\n").unwrap();
		for chunk in fields.chunks(4) {
			source.push_str("\t\t");
			for Field { name, .. } in chunk {
				write!(source, "{name}: f_{name}, ").unwrap();
			}
			source.push('\n');
		}

		source.push_str("\t})\n}\n");
	}
}

/// generate code for converting value to enum
fn from_enum(source: &mut String, item: &DeclItem) {
	let DeclItem::Enum { name, variants, .. } = item else { unreachable!() };

	// header
	write!(source, "impl Into<Value> for {name} {{\n").unwrap();
	source.push_str("\tfn into(self) -> Value {\n");

	// match based on variant
	source.push_str("\t\tmatch self {\n");
	for variant in variants.iter().filter_map(|v| v.as_ref()) {
		let EnumVariant { name: var_name, def, .. } = variant;
		// fieldfull variants
		if def.is_some() {
			write!(source, "\t\t\t{name}::{var_name} {{ .. }} => ").unwrap();
			write!(source, "{name}_{var_name}_to_value(self),\n").unwrap();
		// fieldless variants
		} else {
			write!(source, "\t\t\t{name}::{var_name} => Value::from(\"{var_name}\"),\n").unwrap();
		}
	}

	source.push_str("\t\t}\n\t}\n}\n");

	// generate conv fns for fieldfull variants
	for variant in variants.iter().filter_map(|v| v.as_ref().filter(|v| v.def.is_some())) {
		let EnumVariant { name: var_name, def: Some(def), .. } = variant else { unreachable!() };
		// header
		write!(source, "fn {name}_{var_name}_to_value(value: {name}) -> Value {{\n").unwrap();

		// extract variant fields
		write!(source, "\tlet {name}::{var_name} {{\n").unwrap();
		let fields = def.fields.iter().filter_map(|f| f.as_ref()).collect::<Vec<_>>();
		for chunk in fields.chunks(4) {
			source.push_str("\t\t");
			for Field { name, .. } in chunk {
				write!(source, "{name}: f_{name}, ").unwrap();
			}
			source.push('\n');
		}
		source.push_str("\t} = value else { unreachable!() };\n");

		// build map, insert variant name
		write!(source, "\tlet mut map = HashMap::new();\n").unwrap();
		write!(source, "\tmap.insert(\"$enum_variant\".into(), \"{var_name}\".into());\n").unwrap();

		for Field { name, is_optional, .. } in fields {
			if *is_optional {
				write!(source, "\tif let Some(value) = f_{name} {{\n").unwrap();
				write!(source, "\t\tmap.insert(Key::from(\"{name}\"), value.into());\n\t}}\n")
					.unwrap();
			} else {
				write!(source, "\tmap.insert(Key::from(\"{name}\"), f_{name}.into());\n").unwrap();
			}
		}

		source.push_str("\tValue::Map(map)\n}\n");
	}
}
