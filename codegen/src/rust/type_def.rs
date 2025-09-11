use std::fmt::Write;

use structom::{DeclProvider, internal::*};

use crate::{
	rust::Ctx,
	utils::{add_ident, vuint_to_bytes},
};

/// generate type definition for a decleration file
pub fn gen_type_def(source: &mut String, rel_path: &str, ctx: &Ctx) {
	let Ctx { file, .. } = ctx;

	for (_, item) in &file.items {
		match item {
			DeclItem::Struct { name, def, .. } => {
				// write derived traits
				source.push_str("#[derive(Default, Clone, PartialEq, Debug)]\n");

				write!(source, "pub struct {name} ").unwrap();
				write_struct(source, def, 1, false, ctx);
				source.push('\n');
			}
			DeclItem::Enum { .. } => write_enum(source, item, ctx),
		}
	}
	// write serialized traits
	for (_, item) in &file.items {
		write_serialized_trait(source, item, rel_path);
	}

	source.push('\n');
}

fn write_enum(source: &mut String, item: &DeclItem, ctx: &Ctx) {
	let DeclItem::Enum { name, variants, .. } = item else { unreachable!() };
	// write derived traits
	source.push_str("#[derive(Clone, PartialEq, Debug)]\n");

	// write discriminator type based on the largest tag
	match variants.iter().last().and_then(|v| v.as_ref()).unwrap().tag {
		0..256 => source.push_str("#[repr(u8)]\n"),
		256..65535 => source.push_str("repr(u16)]\n"),
		_ => source.push_str("repr(u32)]\n"),
	}

	write!(source, "pub enum {name} {{\n").unwrap();

	// write variants
	let mut last_tag = 0;
	for variant in variants.iter().flat_map(|v| v.as_ref()) {
		write!(source, "\t{}", variant.name).unwrap();
		// write field definition if exists
		if let Some(ref def) = variant.def {
			source.push(' ');
			write_struct(source, def, 2, true, ctx);
		}
		// write explicit tag if needed
		if variant.tag != last_tag + 1 && last_tag != 0 {
			write!(source, " = {}", variant.tag).unwrap();
		};
		last_tag = variant.tag;

		source.push_str(",\n");
	}
	source.push_str("}\n");

	// write default trait impl
	write!(source, "impl Default for {} {{\n", item.name()).unwrap();
	source.push_str("\tfn default () -> Self {\n\t\t");
	// get first variant
	let variant = variants.iter().find_map(|v| v.as_ref()).unwrap();

	write!(source, "Self::{}", variant.name).unwrap();
	// case has fields
	if let Some(def) = &variant.def {
		source.push_str(" {");
		for Field { name, .. } in def.fields.iter().flat_map(|f| f.as_ref()) {
			write!(source, "\n\t\t\t{name}: Default::default(),").unwrap();
		}
		source.push_str("\n\t\t}");
	}
	source.push_str("\n\t}\n}\n");
}

/// write struct definition
fn write_struct(source: &mut String, def: &StructDef, ident: usize, is_enum: bool, ctx: &Ctx) {
	source.push_str("{\n");

	// write every fields
	for field in def.fields.iter().flat_map(|f| f.as_ref()) {
		add_ident(source, ident);
		write!(source, "{}{}: ", if is_enum { "" } else { "pub " }, field.name).unwrap();
		if field.is_optional {
			source.push_str("Option<");
			write_type(source, &field.typeid, ctx);
			source.push('>');
		} else {
			write_type(source, &field.typeid, ctx);
		}
		source.push_str(",\n");
	}

	add_ident(source, ident - 1);
	source.push('}')
}

/// convert built-in typeid to a rust type
fn resolve_built_in_type(typeid: u8, is_key: bool) -> &'static str {
	match typeid as u8 {
		ANY_TYPEID if !is_key => "Value",
		ANY_TYPEID if is_key => "Key",
		BOOL_TYPEID => "bool",
		STR_TYPEID => "String",

		U8_TYPEID => "u8",
		U16_TYPEID => "u16",
		U32_TYPEID => "u32",
		U64_TYPEID => "u64",

		I8_TYPEID => "i8",
		I16_TYPEID => "i16",
		I32_TYPEID => "i32",
		I64_TYPEID => "i64",

		F32_TYPEID => "f32",
		F64_TYPEID => "f64",

		VINT_TYPEID => "i64",
		VUINT_TYPEID => "u64",
		BINT_TYPEID => "Vec<u8>",

		UUID_TYPEID => "[u8; 16]",
		DUR_TYPEID => "chrono::TimeDelta",
		INST_TYPEID | INSTN_TYPEID => "chrono::DateTime<chrono::Utc>",

		_ => unreachable!(),
	}
}
/// convert a typeid to a rust type
fn write_type(source: &mut String, typeid: &TypeId, ctx: &Ctx) {
	// built-ins
	if typeid.ns == 0 {
		match typeid.id as u8 {
			ARR_TYPEID => {
				source.push_str("Vec<");
				// item type
				write_type(source, typeid.item.as_ref().unwrap(), ctx);
				source.push('>');
			}
			MAP_TYPEID => {
				source.push_str("HashMap<");
				// key type
				source.push_str(resolve_built_in_type(typeid.variant as u8, true));
				source.push_str(", ");
				// value type
				write_type(source, typeid.item.as_ref().unwrap(), ctx);
				source.push('>');
			}
			id => source.push_str(resolve_built_in_type(id, false)),
		}
	// user-defined type
	} else {
		let Ctx { file, provider, path_map } = ctx;
		// same file
		if typeid.ns == file.id {
			source.push_str(file.get_by_id(typeid.id).unwrap().name());
		// other file
		} else {
			// write mod_path::type_name
			let file = provider.get_by_id(typeid.ns);
			source.push_str(path_map.get(&file.id).unwrap());
			source.push_str("::");
			source.push_str(file.get_by_id(typeid.id).unwrap().name());
		}
	}
}

/// generate Serialized trait impl
fn write_serialized_trait(source: &mut String, item: &DeclItem, file: &str) {
	let name = item.name();
	write!(source, "impl Serialized for {name} {{\n").unwrap();

	// encode
	source.push_str("\tfn encode(&self) -> Vec<u8> {\n");
	source.push_str("\t\tlet mut data = Vec::new();\n");
	// header
	source.push_str("\t\tdata.extend_from_slice(&[\n\t\t\t");
	for byte in vuint_to_bytes(file.len() as u64) {
		write!(source, "0x{byte:02x}, ").unwrap();
	}
	for byte in file.as_bytes() {
		write!(source, "0x{byte:02x}, ").unwrap();
	}
	for byte in vuint_to_bytes(item.typeid() as u64) {
		write!(source, "0x{byte:02x}, ").unwrap();
	}
	source.push_str("\n\t\t]);\n");
	// encode item
	write!(source, "\t\tencode_{name}(&mut data, self);\n").unwrap();
	source.push_str("\t\tdata\n");
	source.push_str("\t}\n");

	// encode_inline
	source.push_str("\tfn encode_inline(&self, data: &mut Vec<u8>) {\n");
	write!(source, "\t\tencode_{name}(data, self);\n").unwrap();
	source.push_str("\t}\n");

	// decode
	write!(source, "\tfn decode(data: &[u8]) -> Option<{name}> {{\n").unwrap();
	source.push_str("\t\tlet mut ind = 0;\n");
	// check decl_path
	write!(source, "\t\tif decode_str(data, &mut ind)? != {file:?} {{\n").unwrap();
	source.push_str("\t\t\treturn None;\n\t\t}\n");
	// check typeid
	write!(source, "\t\tif decode_vuint(data, &mut ind)? != {} {{\n", item.typeid()).unwrap();
	source.push_str("\t\t\treturn None;\n\t\t}\n");
	// decode item
	write!(source, "\t\tlet value = decode_{name}(data, &mut ind)?;\n").unwrap();
	// check no remaining data
	source.push_str("\t\tif ind != data.len() { None } else { Some(value) }\n");
	source.push_str("\t}\n");

	// decode_headless
	write!(source, "\tfn decode_headless(data: &[u8]) -> Option<{name}> {{\n").unwrap();
	source.push_str("\t\tlet mut ind = 0;\n");
	write!(source, "\t\tlet value = decode_{name}(data, &mut ind)?;\n").unwrap();
	// check no remaining data
	source.push_str("\t\tif ind != data.len() { None } else { Some(value) }\n");
	source.push_str("\t}\n");

	// decode_inline
	write!(source, "\tfn decode_inline(data: &[u8], ind: &mut usize) -> Option<{name}> {{\n")
		.unwrap();
	write!(source, "\t\tdecode_{name}(data, ind)\n").unwrap();
	source.push_str("\t}\n");

	source.push_str("}\n");
}
