use std::fmt::Write;

use structom::{DeclProvider, internal::*};

use crate::{
	rust::Ctx,
	utils::{add_ident, encode_header},
};

pub fn gen_type_def(source: &mut String, rel_path: &str, ctx: &Ctx) {
	let Ctx { file, .. } = ctx;

	for (_, item) in &file.items {
		match item {
			DeclItem::Struct { name, def, .. } => {
				source.push_str("#[derive(Default, Clone, PartialEq, Debug)]\n");

				write!(source, "pub struct {name} ").unwrap();
				write_fields(source, def, 1, false, ctx);
				source.push('\n');
			}
			DeclItem::Enum { .. } => write_enum(source, item, ctx),
		}
	}
	for (_, item) in &file.items {
		write_serialized_trait(source, item, rel_path);
	}

	source.push('\n');
}

fn write_enum(source: &mut String, item: &DeclItem, ctx: &Ctx) {
	let DeclItem::Enum { name, variants, .. } = item else { unreachable!() };
	source.push_str("#[derive(Clone, PartialEq, Debug)]\n");

	// write discriminator type based on the largest tag
	match variants.last().unwrap().tag {
		0..256 => source.push_str("#[repr(u8)]\n"),
		256..65535 => source.push_str("repr(u16)]\n"),
		_ => source.push_str("repr(u32)]\n"),
	}

	write!(source, "pub enum {name} {{\n").unwrap();

	let mut last_tag = 0;
	for variant in variants {
		write!(source, "\t{}", variant.name).unwrap();
		// write variant fields
		if let Some(def) = &variant.def {
			source.push(' ');
			write_fields(source, def, 2, true, ctx);
		}
		// write explicit tag if needed
		if variant.tag != last_tag + 1 && last_tag != 0 {
			write!(source, " = {}", variant.tag).unwrap();
		};
		last_tag = variant.tag;

		source.push_str(",\n");
	}
	source.push_str("}\n");

	write!(source, "impl Default for {} {{\n", item.name()).unwrap();
	source.push_str("\tfn default () -> Self {\n\t\t");
	let variant = &variants[0];

	write!(source, "Self::{}", variant.name).unwrap();
	if let Some(def) = &variant.def {
		source.push_str(" {");
		for Field { name, .. } in def.fields.values() {
			write!(source, "\n\t\t\t{name}: Default::default(),").unwrap();
		}
		source.push_str("\n\t\t}");
	}
	source.push_str("\n\t}\n}\n");
}

fn write_fields(source: &mut String, def: &StructDef, ident: usize, is_enum: bool, ctx: &Ctx) {
	source.push_str("{\n");

	for field in def.fields.values() {
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
fn resolve_built_in_type(typeid: u16, is_key: bool) -> &'static str {
	match typeid {
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
		BINT_TYPEID => "num_bigint::BigInt",

		UUID_TYPEID => "[u8; 16]",
		DUR_TYPEID => "chrono::TimeDelta",
		INST_TYPEID | INSTN_TYPEID => "chrono::DateTime<chrono::Utc>",

		_ => unreachable!(),
	}
}
/// convert a typeid to a rust type
fn write_type(source: &mut String, typeid: &TypeId, ctx: &Ctx) -> Option<()> {
	if typeid.is_builtin() {
		match typeid.id {
			ARR_TYPEID => {
				source.push_str("Vec<");
				write_type(source, typeid.item(), ctx);
				source.push('>');
			}
			MAP_TYPEID => {
				source.push_str("HashMap<");
				source.push_str(resolve_built_in_type(typeid.variant, true));
				source.push_str(", ");
				write_type(source, typeid.item(), ctx);
				source.push('>');
			}
			id => source.push_str(resolve_built_in_type(id, false)),
		}
	// user-defined type
	} else {
		let Ctx { file, provider, path_map } = ctx;
		// same file
		if typeid.ns == file.id {
			source.push_str(file.get_by_id(typeid.id)?.name());
		} else {
			// write mod_path::type_name
			let file = provider.get(typeid.ns);
			source.push_str(path_map.get(&file.id)?);
			source.push_str("::");
			source.push_str(file.get_by_id(typeid.id)?.name());
		}
	}
	Some(())
}

/// generate Serialized trait impl
fn write_serialized_trait(source: &mut String, item: &DeclItem, file: &str) {
	let name = item.name();
	write!(source, "impl Serialized for {name} {{\n").unwrap();

	// encode
	source.push_str("\tfn encode(&self) -> Vec<u8> {\n");
	source.push_str("\t\tlet mut data = Vec::new();\n");

	source.push_str("\t\tdata.extend_from_slice(&[\n\t\t\t");
	encode_header(source, file, item.typeid() as u64);
	source.push_str("\n\t\t]);\n");

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
	// check header
	write!(source, "\t\tif decode_str(data, &mut ind)? != {file:?} {{\n").unwrap();
	source.push_str("\t\t\treturn None;\n\t\t}\n");
	// check typeid
	write!(source, "\t\tif decode_vuint(data, &mut ind)? != {} {{\n", item.typeid()).unwrap();
	source.push_str("\t\t\treturn None;\n\t\t}\n");

	write!(source, "\t\tlet value = decode_{name}(data, &mut ind)?;\n").unwrap();

	source.push_str("\t\tif ind != data.len() { None } else { Some(value) }\n");
	source.push_str("\t}\n");

	// decode_headless
	write!(source, "\tfn decode_headless(data: &[u8]) -> Option<{name}> {{\n").unwrap();
	source.push_str("\t\tlet mut ind = 0;\n");
	write!(source, "\t\tlet value = decode_{name}(data, &mut ind)?;\n").unwrap();

	source.push_str("\t\tif ind != data.len() { None } else { Some(value) }\n");
	source.push_str("\t}\n");

	// decode_inline
	write!(source, "\tfn decode_inline(data: &[u8], ind: &mut usize) -> Option<{name}> {{\n")
		.unwrap();
	write!(source, "\t\tdecode_{name}(data, ind)\n").unwrap();
	source.push_str("\t}\n");

	source.push_str("}\n");
}
