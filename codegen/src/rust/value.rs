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

fn to_struct(source: &mut String, item: &DeclItem) {
	let DeclItem::Struct { name, def, .. } = item else { unreachable!() };

	write!(source, "impl TryFrom<Value> for {name} {{\n").unwrap();
	source.push_str("\ttype Error = ();\n");
	write!(source, "\tfn try_from(value: Value) -> Result<{name}, ()> {{\n").unwrap();

	source.push_str("\t\tlet Value::Map(mut map) = value else { return Err(()); };\n");

	for Field { name, is_optional, typeid, .. } in def.fields.values() {
		if typeid.is_any() {
			if *is_optional {
				write!(source, "\t\tlet f_{name} = map.remove(&\"{name}\".into());\n").unwrap();
			} else {
				write!(source, "\t\tlet f_{name} = map.remove(&\"{name}\".into()).ok_or(())?;\n")
					.unwrap();
			}
		} else if *is_optional {
			write!(source, "\t\tlet f_{name} = map.remove(&\"{name}\".into())").unwrap();
			source.push_str(".map(|v| v.try_into()).transpose()?;\n");
		} else {
			write!(source, "\t\tlet f_{name} = map.remove(&\"{name}\".into())").unwrap();
			source.push_str(".ok_or(())?.try_into()?;\n");
		}
	}

	source.push_str("\t\tif !map.is_empty() { return Err(()); }\n");

	write!(source, "\t\tOk({name} {{\n").unwrap();
	for Field { name, .. } in def.fields.values() {
		write!(source, "\t\t\t{name}: f_{name}, \n").unwrap();
	}

	source.push_str("\t\t})\n\t}\n}\n");
}

fn from_struct(source: &mut String, item: &DeclItem) {
	let DeclItem::Struct { name, def, .. } = item else { unreachable!() };

	write!(source, "impl Into<Value> for {name} {{\n").unwrap();
	source.push_str("\tfn into(self) -> Value {\n");
	source.push_str("\t\tlet mut map = HashMap::new();\n");

	// insert fields
	for Field { name, is_optional, .. } in def.fields.values() {
		if *is_optional {
			write!(source, "\t\tif let Some(value) = self.{name} {{\n").unwrap();
			write!(source, "\t\t\tmap.insert(Key::from(\"{name}\"), value.into());\n\t\t}}\n")
				.unwrap();
		} else {
			write!(source, "\t\tmap.insert(Key::from(\"{name}\"), self.{name}.into());\n").unwrap();
		}
	}

	source.push_str("\t\tValue::Map(Box::new(map))\n\t}\n}\n");
}

fn to_enum(source: &mut String, item: &DeclItem) {
	let DeclItem::Enum { name, variants, .. } = item else { unreachable!() };

	write!(source, "impl TryFrom<Value> for {name} {{\n").unwrap();
	source.push_str("\ttype Error = ();\n");
	write!(source, "\tfn try_from(value: Value) -> Result<{name}, ()> {{\n").unwrap();

	let (unit_vars, fieldfull_vars): (Vec<_>, Vec<_>) =
		variants.iter().partition(|v| v.def.is_none());

	if !unit_vars.is_empty() {
		source.push_str("\t\tif let Value::UnitVar(v) = value {\n");
		source.push_str("\t\t\treturn match v.as_str() {\n");
		for EnumVariant { name: var_name, .. } in unit_vars {
			write!(source, "\t\t\t\t\"{var_name}\" => Ok({name}::{var_name}),\n").unwrap();
		}
		source.push_str("\t\t\t\t_ => Err(()),\n");
		source.push_str("\t\t\t};\n\t\t}\n");
	}

	if !fieldfull_vars.is_empty() {
		source.push_str("\t\tif let Some(var) = value.enum_variant() {\n");

		source.push_str("\t\t\treturn match var {\n");
		for EnumVariant { name: var_name, .. } in &fieldfull_vars {
			write!(source, "\t\t\t\t\"{var_name}\" => {name}_{var_name}_from_value(value),\n")
				.unwrap();
		}
		source.push_str("\t\t\t\t_ => Err(()),\n");
		source.push_str("\t\t\t};\n\t\t}\n");
	}

	source.push_str("\t\tErr(())\n\t}\n}\n");

	// generate conv fns for fieldfull variants
	for variant in fieldfull_vars {
		let EnumVariant { name: var_name, def: Some(def), .. } = variant else { unreachable!() };

		write!(source, "fn {name}_{var_name}_from_value(value: Value) -> Result<{name}, ()> {{\n")
			.unwrap();

		source.push_str("\tlet Value::Map(mut map) = value else { return Err(()); };\n");
		source.push_str("\tmap.remove(&\"$enum_variant\".into()).ok_or(())?;\n");

		for Field { name, is_optional, typeid, .. } in def.fields.values() {
			if typeid.is_any() {
				if *is_optional {
					write!(source, "\tlet f_{name} = map.remove(&\"{name}\".into());\n").unwrap();
				} else {
					write!(source, "\tlet f_{name} = map.remove(&\"{name}\".into()).ok_or(())?;\n")
						.unwrap();
				}
			} else if *is_optional {
				write!(source, "\tlet f_{name} = map.remove(&\"{name}\".into())").unwrap();
				source.push_str(".map(|v| v.try_into()).transpose()?;\n");
			} else {
				write!(source, "\tlet f_{name} = map.remove(&\"{name}\".into())").unwrap();
				source.push_str(".ok_or(())?.try_into()?;\n");
			}
		}

		source.push_str("\tif !map.is_empty() { return Err(()); }\n");

		write!(source, "\tOk({name}::{var_name} {{\n").unwrap();
		for Field { name, .. } in def.fields.values() {
			write!(source, "\t\t{name}: f_{name}, \n").unwrap();
		}

		source.push_str("\t})\n}\n");
	}
}

fn from_enum(source: &mut String, item: &DeclItem) {
	let DeclItem::Enum { name, variants, .. } = item else { unreachable!() };

	write!(source, "impl Into<Value> for {name} {{\n").unwrap();
	source.push_str("\tfn into(self) -> Value {\n");

	source.push_str("\t\tmatch self {\n");
	for EnumVariant { name: var_name, def, .. } in variants {
		// fieldfull variants
		if def.is_some() {
			write!(source, "\t\t\t{name}::{var_name} {{ .. }} => ").unwrap();
			write!(source, "{name}_{var_name}_to_value(self),\n").unwrap();
		// unit variants
		} else {
			write!(source, "\t\t\t{name}::{var_name} => Value::UnitVar(\"{var_name}\"").unwrap();
			source.push_str(".to_string()),\n");
		}
	}

	source.push_str("\t\t}\n\t}\n}\n");

	// generate conv fns for fieldfull variants
	for variant in variants.iter().filter(|v| v.def.is_some()) {
		let EnumVariant { name: var_name, def: Some(def), .. } = variant else { unreachable!() };
		write!(source, "fn {name}_{var_name}_to_value(value: {name}) -> Value {{\n").unwrap();

		write!(source, "\tlet {name}::{var_name} {{\n").unwrap();
		for Field { name, .. } in def.fields.values() {
			write!(source, "\t\t{name}: f_{name}, \n").unwrap();
		}
		source.push_str("\t} = value else { unreachable!() };\n");

		write!(source, "\tlet mut map = HashMap::new();\n").unwrap();
		write!(source, "\tmap.insert(\"$enum_variant\".into(), \"{var_name}\".into());\n").unwrap();

		for Field { name, is_optional, .. } in def.fields.values() {
			if *is_optional {
				write!(source, "\tif let Some(value) = f_{name} {{\n").unwrap();
				write!(source, "\t\tmap.insert(Key::from(\"{name}\"), value.into());\n\t}}\n")
					.unwrap();
			} else {
				write!(source, "\tmap.insert(Key::from(\"{name}\"), f_{name}.into());\n").unwrap();
			}
		}

		source.push_str("\tValue::Map(Box::new(map))\n}\n");
	}
}
