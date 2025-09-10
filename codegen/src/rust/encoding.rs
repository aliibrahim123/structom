use crate::{rust::Ctx, utils::add_ident};
use std::fmt::Write;

use structom::{DeclProvider, internal::*};

/// generate encoding functions
pub fn gen_encoding(source: &mut String, ctx: &Ctx) {
	let Ctx { file, .. } = ctx;

	for (_, item) in &file.items {
		match item {
			DeclItem::Struct { name, def, .. } => {
				write!(source, "pub fn encode_{name}(data: &mut Vec<u8>, value: &{name}) {{\n",)
					.unwrap();
				encode_struct(source, def, name, ctx);
				source.push_str("}\n");

				write!(source, "pub fn decode_{name}(data: &[u8], ind: &mut usize)").unwrap();
				write!(source, " -> Option<{name}> {{\n").unwrap();
				decode_struct(source, def, name, ctx);
				source.push_str("}\n\n");
			}
			DeclItem::Enum { .. } => {
				encode_enum(source, item, ctx);

				decode_enum(source, item, ctx);
			}
		}
	}
}

/// generate encoding function for enum
fn encode_enum(source: &mut String, item: &DeclItem, ctx: &Ctx) {
	let DeclItem::Enum { name, variants, .. } = item else { unreachable!() };

	// fn decleration
	write!(source, "pub fn encode_{name}(data: &mut Vec<u8>, value: &{name}) {{\n").unwrap();
	// encode based on variant
	source.push_str("\tmatch value {");
	for variant in variants.iter().filter_map(|v| v.as_ref()) {
		let EnumVariant { name: var_name, tag, .. } = variant;
		// has fields
		if variant.def.is_some() {
			write!(source, "\n\t\t{name}::{var_name} {{ .. }} => {{\n").unwrap();
			write!(source, "\t\t\tencode_vuint(data, {tag});\n",).unwrap();
			write!(source, "\t\t\tencode_{name}_{var_name}(data, value);\n",).unwrap();
			source.push_str("\t\t},");
		// case unit enum
		} else {
			write!(source, "\n\t\t{name}::{var_name} => encode_vuint(data, {tag}),",).unwrap();
		}
	}
	source.push_str("\n\t}\n}\n");

	// generate encode functions for variants with fields
	for variant in variants.iter().filter_map(|v| v.as_ref().filter(|v| v.def.is_some())) {
		let EnumVariant { name: var_name, def: Some(def), .. } = variant else { unreachable!() };
		write!(source, "pub fn encode_{name}_{var_name}(data: &mut Vec<u8>, ").unwrap();
		write!(source, "value: &{name}) {{\n").unwrap();
		encode_struct(source, def, &format!("{name}::{var_name}"), ctx);
		source.push_str("}\n");
	}
}
/// generate encoding function for struct
fn encode_struct(source: &mut String, def: &StructDef, type_name: &str, ctx: &Ctx) {
	// extract fields
	write!(source, "\tlet {type_name} {{").unwrap();
	for chunk in def.fields.iter().filter_map(|f| f.as_ref()).collect::<Vec<_>>().chunks(4) {
		source.push_str("\n\t\t");
		for Field { name, .. } in chunk {
			write!(source, "{name}: f_{name}, ").unwrap();
		}
	}
	source.push_str("\n\t} = value else { unreachable!() };\n");

	// split fields into optional and required
	#[rustfmt::skip]
	let (opt_fields, req_fields): (Vec<_>, Vec<_>) = 
		def.fields.iter().filter_map(|f| f.as_ref()).partition(|field| field.is_optional);

	// encode fields count
	// case only required, direct count
	if opt_fields.is_empty() {
		write!(source, "\tencode_vuint(data, {});\n", req_fields.len()).unwrap();
	// case there optionals, count them if have value
	} else {
		write!(source, "\tencode_vuint(data, {}\n", req_fields.len()).unwrap();
		for field in &opt_fields {
			write!(source, "\t\t+ if f_{}.is_some() {{1}} else {{0}}\n", field.name).unwrap();
		}
		source.push_str("\t);\n");
	}

	// encode required fields
	for field in req_fields {
		encode_field(source, &field, ctx);
	}
	// encode optional fields if have value
	for field in opt_fields {
		write!(source, "\tif let Some(f_{0}) = f_{0} {{\n", field.name).unwrap();
		encode_field(source, &field, ctx);
		source.push_str("\t}\n");
	}
}

// encode code for common field value types
// encode header then value
fn encode_simple_value(source: &mut String, ty: &str, name: &str, tag: u32, size: u32) {
	write!(source, "\tencode_vuint(data, {});\n", tag << 3 | size).unwrap();
	write!(source, "\tencode_{ty}(data, &f_{name});\n").unwrap();
}
fn encode_copy_value(source: &mut String, ty: &str, name: &str, tag: u32, size: u32) {
	write!(source, "\tencode_vuint(data, {});\n", tag << 3 | size).unwrap();
	write!(source, "\tencode_{ty}(data, *f_{name});\n").unwrap();
}
fn encode_compound_value(source: &mut String, ty: &str, name: &str, tag: u32, size: u32) {
	// has len field with predefined value
	write!(source, "\tencode_vuint(data, {});\n", tag << 3 | 0b101).unwrap();
	write!(source, "\tencode_vuint(data, {size});\n").unwrap();
	write!(source, "\tencode_{ty}(data, f_{name});\n").unwrap();
}
fn encode_sized_value(source: &mut String, tag: u32, encoder: impl Fn(&mut String)) {
	write!(source, "\tencode_vuint(data, {});\n", tag << 3 | 0b101).unwrap();
	// reserve 2 byte space for len
	source.push_str("\tlet size_ind = data.len();\n");
	source.push_str("\tdata.extend_from_slice(&[0,0]);\n\t");
	// encode value
	encoder(source);
	// encode len, expand it if required
	source.push_str(
		";\n\tencode_vuint_pre_aloc(data, (data.len() - size_ind - 2) as u64, size_ind, 2);\n",
	);
}
// generate fn that encode primitive types
fn write_primitive_encoder(source: &mut String, typeid: u8, is_key: bool) {
	match typeid {
		ANY_TYPEID if is_key => source.push_str("encode_any_key"),
		ANY_TYPEID if !is_key => source.push_str("encode_any"),

		BOOL_TYPEID => source.push_str("|data, value| encode_bool(data, *value)"),
		U8_TYPEID => source.push_str("|data, value| encode_u8(data, *value)"),
		U16_TYPEID => source.push_str("|data, value| encode_u16(data, *value)"),
		U32_TYPEID => source.push_str("|data, value| encode_u32(data, *value)"),
		U64_TYPEID => source.push_str("|data, value| encode_u64(data, *value)"),

		I8_TYPEID => source.push_str("|data, value| encode_i8(data, *value)"),
		I16_TYPEID => source.push_str("|data, value| encode_i16(data, *value)"),
		I32_TYPEID => source.push_str("|data, value| encode_i32(data, *value)"),
		I64_TYPEID => source.push_str("|data, value| encode_i64(data, *value)"),

		VUINT_TYPEID => source.push_str("|data, value| encode_vuint(data, *value)"),
		VINT_TYPEID => source.push_str("|data, value| encode_vint(data, *value)"),
		BINT_TYPEID => source.push_str("encode_u8_arr"),

		F32_TYPEID => source.push_str("|data, value| encode_f32(data, *value)"),
		F64_TYPEID => source.push_str("|data, value| encode_f64(data, *value)"),

		STR_TYPEID => source.push_str("|data, value| encode_str(data, value.as_str())"),

		INST_TYPEID => source.push_str("encode_inst"),
		INSTN_TYPEID => source.push_str("encode_instN"),
		DUR_TYPEID => source.push_str("encode_dur"),
		UUID_TYPEID => source.push_str("encode_uuid"),
		_ => (),
	}
}
/// write fn that encode specific type
fn write_value_encoder(source: &mut String, typeid: &TypeId, ctx: &Ctx) {
	let Ctx { file, path_map, provider } = ctx;
	// builtins
	if typeid.ns == 0 {
		match typeid.id as u8 {
			ARR_TYPEID => {
				source.push_str("|data, value| encode_arr(data, value, false, ");
				// item encoder
				write_value_encoder(source, typeid.item.as_ref().unwrap(), ctx);
				source.push_str(")");
			}
			MAP_TYPEID => {
				source.push_str("|data, value| encode_map(data, value, false, ");
				// key encoder
				write_primitive_encoder(source, typeid.variant as u8, true);
				source.push_str(", ");
				// value encoder
				write_value_encoder(source, typeid.item.as_ref().unwrap(), ctx);
				source.push_str(")");
			}
			id => write_primitive_encoder(source, id, false),
		}
	// user defined
	} else {
		// same file
		if typeid.ns == file.id {
			write!(source, "encode_{}", file.get_by_id(typeid.id).unwrap().name()).unwrap();
		// different file
		} else {
			// write mod_path::encode_type
			let file = provider.get_by_id(typeid.ns);
			source.push_str(path_map.get(&typeid.ns).unwrap());
			source.push_str("::encode_");
			source.push_str(file.get_by_id(typeid.id).unwrap().name());
		}
	}
}
/// generate encode code for a field
fn encode_field(source: &mut String, field: &Field, ctx: &Ctx) {
	let Ctx { file, path_map, provider } = ctx;
	let Field { name, typeid, tag, .. } = field;
	// builtins
	if typeid.ns == 0 {
		match typeid.id as u8 {
			ANY_TYPEID => encode_sized_value(source, *tag, |source| {
				write!(source, "encode_any(data, f_{name})").unwrap()
			}),

			BOOL_TYPEID => encode_copy_value(source, "bool", name, *tag, 0b000),

			U8_TYPEID => encode_copy_value(source, "u8", name, *tag, 0b000),
			U16_TYPEID => encode_copy_value(source, "u16", name, *tag, 0b001),
			U32_TYPEID => encode_copy_value(source, "u32", name, *tag, 0b010),
			U64_TYPEID => encode_copy_value(source, "u64", name, *tag, 0b011),

			I8_TYPEID => encode_copy_value(source, "i8", name, *tag, 0b000),
			I16_TYPEID => encode_copy_value(source, "i16", name, *tag, 0b001),
			I32_TYPEID => encode_copy_value(source, "i32", name, *tag, 0b010),
			I64_TYPEID => encode_copy_value(source, "i64", name, *tag, 0b011),

			F32_TYPEID => encode_copy_value(source, "f32", name, *tag, 0b010),
			F64_TYPEID => encode_copy_value(source, "f64", name, *tag, 0b011),

			VUINT_TYPEID => encode_copy_value(source, "vuint", name, *tag, 0b100),
			VINT_TYPEID => encode_copy_value(source, "vint", name, *tag, 0b100),
			BINT_TYPEID => encode_simple_value(source, "u8_arr", name, *tag, 0b101),

			STR_TYPEID => encode_simple_value(source, "str", name, *tag, 0b101),
			ARR_TYPEID => encode_sized_value(source, *tag, |source| {
				write!(source, "encode_arr(data, f_{name}, true, ").unwrap();
				// item encoder
				write_value_encoder(source, typeid.item.as_ref().unwrap(), ctx);
				source.push(')');
			}),
			MAP_TYPEID => encode_sized_value(source, *tag, |source| {
				write!(source, "encode_map(data, f_{name}, true, ").unwrap();
				// key encoder
				write_primitive_encoder(source, typeid.variant as u8, true);
				source.push_str(", ");
				// value encoder
				write_value_encoder(source, typeid.item.as_ref().unwrap(), ctx);
				source.push(')');
			}),

			INST_TYPEID => encode_simple_value(source, "inst", name, *tag, 0b011),
			INSTN_TYPEID => encode_compound_value(source, "instN", name, *tag, 12),
			DUR_TYPEID => encode_simple_value(source, "dur", name, *tag, 0b011),
			UUID_TYPEID => encode_compound_value(source, "uuid", name, *tag, 16),
			_ => unreachable!(),
		}
	// user defined types
	} else {
		encode_sized_value(source, *tag, |source| {
			// same file
			if typeid.ns == file.id {
				let item_name = file.get_by_id(typeid.id).unwrap().name();
				write!(source, "encode_{item_name}(data, f_{name})",).unwrap();
			// different file
			} else {
				// write mod_path::encode_value
				let file = provider.get_by_id(typeid.ns);
				let item_name = file.get_by_id(typeid.id).unwrap().name();
				source.push_str(path_map.get(&typeid.ns).unwrap());
				write!(source, "::encode_{item_name}(data, f_{name})").unwrap();
			}
		});
	}
}

fn decode_enum(source: &mut String, item: &DeclItem, ctx: &Ctx) {
	let DeclItem::Enum { name, variants, .. } = item else { unreachable!() };

	write!(source, "pub fn decode_{name}(data: &[u8], ind: &mut usize) -> Option<{name}> {{\n")
		.unwrap();

	write!(source, "\tmatch decode_vuint(data, ind)? {{\n").unwrap();
	for variant in variants.iter().filter_map(|v| v.as_ref()) {
		let EnumVariant { name: var_name, tag, def, .. } = variant;
		if def.is_some() {
			write!(source, "\t\t{tag} => decode_{name}_{var_name}(data, ind),\n").unwrap();
		} else {
			write!(source, "\t\t{tag} => Some({name}::{var_name}),\n").unwrap();
		}
	}
	source.push_str("\t\t_ => None,\n\t}\n}\n");

	for variant in variants.iter().filter_map(|v| v.as_ref().filter(|v| v.def.is_some())) {
		let EnumVariant { name: var_name, def: Some(def), .. } = variant else { unreachable!() };
		write!(source, "pub fn decode_{name}_{var_name}(data: &[u8], ind: &mut usize)").unwrap();
		write!(source, " -> Option<{name}> {{").unwrap();
		decode_struct(source, def, &format!("{name}::{var_name}"), ctx);
		source.push_str("}\n");
	}

	source.push('\n');
}

fn decode_struct(source: &mut String, def: &StructDef, name: &str, ctx: &Ctx) {
	let fields = def.fields.iter().filter_map(|f| f.as_ref()).collect::<Vec<_>>();
	for chunk in fields.chunks(4) {
		source.push_str("\t");
		for Field { name, .. } in chunk {
			write!(source, "let mut f_{name} = None; ").unwrap();
		}
		source.push('\n')
	}

	source.push_str("\tfor _ in 0..decode_vuint(data, ind)? {\n");
	source.push_str("\t\tlet header = decode_vuint(data, ind)?;\n");
	source.push_str("\t\tlet tag = header >> 3;\n");

	let mut is_first = true;
	for field in &fields {
		let Field { name, tag, typeid, .. } = field;
		if is_first {
			write!(source, "\t\tif tag == {tag} {{\n").unwrap();
		} else {
			write!(source, " else if tag == {tag} {{\n").unwrap();
		}
		decode_field(source, name, typeid, ctx);
		source.push_str("\t\t}");
		is_first = false;
	}

	source.push_str(" else { skip_field(data, ind, header)? }\n");
	source.push_str("\t}\n");

	for field in fields.iter().filter(|f| !f.is_optional) {
		let Field { name, .. } = field;
		write!(source, "\tlet Some(f_{name}) = f_{name} else {{ return None; }};\n").unwrap();
	}

	write!(source, "\tSome({name} {{\n").unwrap();
	for chunk in fields.chunks(4) {
		source.push_str("\t\t");
		for Field { name, .. } in chunk {
			write!(source, "{name}: f_{name}, ").unwrap();
		}
		source.push('\n');
	}
	source.push_str("\t})\n");
}

fn decode_simple_value(source: &mut String, name: &str, ty: &str) {
	write!(source, "\t\t\tf_{name} = Some(decode_{ty}(data, ind)?);\n").unwrap();
}
fn decode_compound_value(source: &mut String, name: &str, ty: &str) {
	source.push_str("\t\t\tdecode_vuint(data, ind)?;\n");
	write!(source, "\t\t\tf_{name} = Some(decode_{ty}(data, ind)?);\n").unwrap();
}
fn write_primitive_decoder(source: &mut String, typeid: u8, is_key: bool) {
	match typeid {
		ANY_TYPEID if is_key => source.push_str("decode_any_key"),
		ANY_TYPEID if !is_key => source.push_str("decode_any"),

		BOOL_TYPEID => source.push_str("decode_bool"),
		U8_TYPEID => source.push_str("decode_u8"),
		U16_TYPEID => source.push_str("decode_u16"),
		U32_TYPEID => source.push_str("decode_u32"),
		U64_TYPEID => source.push_str("decode_u64"),

		I8_TYPEID => source.push_str("decode_i8"),
		I16_TYPEID => source.push_str("decode_i16"),
		I32_TYPEID => source.push_str("decode_i32"),
		I64_TYPEID => source.push_str("decode_i64"),

		VUINT_TYPEID => source.push_str("decode_vuint"),
		VINT_TYPEID => source.push_str("decode_vint"),
		BINT_TYPEID => source.push_str("decode_u8_arr"),

		F32_TYPEID => source.push_str("decode_f32"),
		F64_TYPEID => source.push_str("decode_f64"),

		STR_TYPEID => source.push_str("decode_str"),

		INST_TYPEID => source.push_str("decode_inst"),
		INSTN_TYPEID => source.push_str("decode_instN"),
		DUR_TYPEID => source.push_str("decode_dur"),
		UUID_TYPEID => source.push_str("decode_uuid"),
		_ => (),
	}
}
fn write_value_decoder(source: &mut String, typeid: &TypeId, ctx: &Ctx) {
	let Ctx { file, path_map, provider } = ctx;
	if typeid.ns == 0 {
		match typeid.id as u8 {
			ARR_TYPEID => {
				source.push_str("|data, ind| decode_arr(data, ind, false, ");
				write_value_decoder(source, typeid.item.as_ref().unwrap(), ctx);
				source.push_str(")");
			}
			MAP_TYPEID => {
				source.push_str("|data, ind| decode_map(data, ind, false, ");
				write_primitive_decoder(source, typeid.variant as u8, true);
				source.push_str(", ");
				write_value_decoder(source, typeid.item.as_ref().unwrap(), ctx);
				source.push_str(")");
			}
			id => write_primitive_decoder(source, id, false),
		}
	} else {
		if typeid.ns == file.id {
			write!(source, "decode_{}", file.get_by_id(typeid.id).unwrap().name()).unwrap();
		} else {
			let file = provider.get_by_id(typeid.ns);
			source.push_str(path_map.get(&typeid.ns).unwrap());
			source.push_str("::decode_");
			source.push_str(file.get_by_id(typeid.id).unwrap().name());
		}
	}
}
fn decode_field(source: &mut String, name: &str, typeid: &TypeId, ctx: &Ctx) {
	let Ctx { file, provider, path_map } = ctx;
	if typeid.ns == 0 {
		match typeid.id as u8 {
			ANY_TYPEID => decode_compound_value(source, name, "any"),
			BOOL_TYPEID => decode_simple_value(source, name, "bool"),

			U8_TYPEID => decode_simple_value(source, name, "u8"),
			U16_TYPEID => decode_simple_value(source, name, "u16"),
			U32_TYPEID => decode_simple_value(source, name, "u32"),
			U64_TYPEID => decode_simple_value(source, name, "u64"),

			I8_TYPEID => decode_simple_value(source, name, "i8"),
			I16_TYPEID => decode_simple_value(source, name, "i16"),
			I32_TYPEID => decode_simple_value(source, name, "i32"),
			I64_TYPEID => decode_simple_value(source, name, "i64"),

			F32_TYPEID => decode_simple_value(source, name, "f32"),
			F64_TYPEID => decode_simple_value(source, name, "f64"),

			VUINT_TYPEID => decode_simple_value(source, name, "vuint"),
			VINT_TYPEID => decode_simple_value(source, name, "vint"),
			BINT_TYPEID => decode_simple_value(source, name, "u8_arr"),

			STR_TYPEID => decode_simple_value(source, name, "str"),
			ARR_TYPEID => {
				write!(source, "\t\t\tf_{name} = Some(decode_arr(data, ind, true, ").unwrap();
				write_value_decoder(source, typeid.item.as_ref().unwrap(), ctx);
				source.push_str(")?);\n");
			}
			MAP_TYPEID => {
				write!(source, "\t\t\tf_{name} = Some(decode_map(data, ind, true, ").unwrap();
				write_primitive_decoder(source, typeid.variant as u8, true);
				source.push_str(", ");
				write_value_decoder(source, typeid.item.as_ref().unwrap(), ctx);
				source.push_str(")?);\n");
			}

			INST_TYPEID => decode_simple_value(source, name, "inst"),
			INSTN_TYPEID => decode_compound_value(source, name, "instN"),
			DUR_TYPEID => decode_simple_value(source, name, "dur"),
			UUID_TYPEID => decode_compound_value(source, name, "uuid"),
			_ => (),
		}
	} else {
		if typeid.ns == file.id {
			source.push_str("\t\t\tdecode_vuint(data, ind)?;\n");
			let type_name = file.get_by_id(typeid.id).unwrap().name();
			write!(source, "\t\t\tf_{name} = Some(decode_{type_name}(data, ind)?);\n",).unwrap();
		} else {
			let file = provider.get_by_id(typeid.ns);
			source.push_str("\t\t\tdecode_vuint(data, ind)?;\n");
			write!(source, "\t\t\tf_{name} = Some(").unwrap();
			source.push_str(path_map.get(&typeid.ns).unwrap());
			let type_name = file.get_by_id(typeid.id).unwrap().name();
			write!(source, "::decode_{type_name}(data, ind)?);\n",).unwrap();
		}
	}
}
