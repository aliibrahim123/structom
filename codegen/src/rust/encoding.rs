use crate::{
	rust::Ctx,
	utils::{field_len},
};
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
				decode_fields(source, def, name, ctx);
				source.push_str("}\n\n");
			}
			DeclItem::Enum { .. } => {
				encode_enum(source, item, ctx);

				decode_enum(source, item, ctx);
			}
		}
	}
}

fn encode_enum(source: &mut String, item: &DeclItem, ctx: &Ctx) {
	let DeclItem::Enum { name, variants, .. } = item else { unreachable!() };

	write!(source, "pub fn encode_{name}(data: &mut Vec<u8>, value: &{name}) {{\n").unwrap();
	source.push_str("\tmatch value {");
	for variant in variants {
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

	for variant in variants.iter().filter(|v| v.def.is_some()) {
		let EnumVariant { name: var_name, def: Some(def), .. } = variant else { unreachable!() };
		write!(source, "pub fn encode_{name}_{var_name}(data: &mut Vec<u8>, ").unwrap();
		write!(source, "value: &{name}) {{\n").unwrap();
		encode_struct(source, def, &format!("{name}::{var_name}"), ctx);
		source.push_str("}\n");
	}
}

fn encode_struct(source: &mut String, def: &StructDef, type_name: &str, ctx: &Ctx) {
	// extract fields into f_name, because of enum variants
	write!(source, "\tlet {type_name} {{").unwrap();
	for Field { name, .. } in def.fields.values() {
		write!(source, "\n\t\t{name}: f_{name}, ").unwrap();
	}
	source.push_str("\n\t} = value else { unreachable!() };\n");

	#[rustfmt::skip]
	let (opt_fields, req_fields): (Vec<_>, Vec<_>) = 
		def.fields.values().partition(|field| field.is_optional);

	// encode fields count
	if opt_fields.is_empty() {
		write!(source, "\tencode_vuint(data, {});\n", req_fields.len()).unwrap();
	} else {
		write!(source, "\tencode_vuint(data, {}\n", req_fields.len()).unwrap();
		for field in &opt_fields {
			write!(source, "\t\t+ if f_{}.is_some() {{1}} else {{0}}\n", field.name).unwrap();
		}
		source.push_str("\t);\n");
	}

	for field in req_fields {
		encode_field(source, &field, ctx);
	}
	for field in opt_fields {
		write!(source, "\tif let Some(f_{0}) = f_{0} {{\n", field.name).unwrap();
		encode_field(source, &field, ctx);
		source.push_str("\t}\n");
	}
}

// encode header then value
fn encode_simple_value(source: &mut String, ty: &str, name: &str, tag: u32, size: u32, is_copy: bool) {
	write!(source, "\tencode_vuint(data, {});\n", tag << 3 | size).unwrap();
	write!(source, "\tencode_{ty}(data, {}f_{name});\n", if is_copy { "*" } else { "&" }).unwrap();
}
/// has len field with predefined value
fn encode_compound_value(source: &mut String, ty: &str, name: &str, tag: u32, size: u32) {
	write!(source, "\tencode_vuint(data, {});\n", tag << 3 | field_len::LEN).unwrap();
	write!(source, "\tencode_vuint(data, {size});\n").unwrap();
	write!(source, "\tencode_{ty}(data, f_{name});\n").unwrap();
}
fn encode_sized_value(source: &mut String, tag: u32, encoder: impl Fn(&mut String)) {
	write!(source, "\tencode_vuint(data, {});\n", tag << 3 | 0b101).unwrap();
	// reserve 2 byte space for len
	source.push_str("\tlet size_ind = data.len();\n");
	source.push_str("\tdata.extend_from_slice(&[0,0]);\n\t");
	
	encoder(source);
	// encode len, expand it if required
	source.push_str(
		";\n\tencode_vuint_pre_aloc(data, (data.len() - size_ind - 2) as u64, size_ind, 2);\n",
	);
}

fn write_primitive_encoder(source: &mut String, typeid: u16, is_key: bool) {
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
		BINT_TYPEID => source.push_str("encode_bint"),

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

fn write_value_encoder(source: &mut String, typeid: &TypeId, ctx: &Ctx) {
	let Ctx { file, path_map, provider } = ctx;
	if typeid.is_builtin() {
		match typeid.id {
			ARR_TYPEID => {
				source.push_str("|data, value| encode_arr(data, value, false, ");
				write_value_encoder(source, typeid.item(), ctx);
				source.push_str(")");
			}
			MAP_TYPEID => {
				source.push_str("|data, value| encode_map(data, value, false, ");
				write_primitive_encoder(source, typeid.variant, true);
				source.push_str(", ");
				write_value_encoder(source, typeid.item(), ctx);
				source.push_str(")");
			}
			id => write_primitive_encoder(source, id, false),
		}
	// user defined
	} else {
		// same file
		if typeid.ns == file.id {
			write!(source, "encode_{}", file.get_by_id(typeid.id).unwrap().name()).unwrap();
		} else {
			// write mod_path::encode_type
			let file = provider.get(typeid.ns);
			source.push_str(path_map.get(&typeid.ns).unwrap());
			source.push_str("::encode_");
			source.push_str(file.get_by_id(typeid.id).unwrap().name());
		}
	}
}
fn encode_field(source: &mut String, field: &Field, ctx: &Ctx) {
	let Ctx { file, path_map, provider } = ctx;
	let Field { name, typeid, tag, .. } = field;
	if typeid.is_builtin() {
		match typeid.id {
			ANY_TYPEID => encode_sized_value(source, *tag, |source| {
				write!(source, "encode_any(data, f_{name})").unwrap()
			}),

			BOOL_TYPEID => encode_simple_value(source, "bool", name, *tag, field_len::U8, true),

			U8_TYPEID => encode_simple_value(source, "u8", name, *tag, field_len::U8, true),
			U16_TYPEID => encode_simple_value(source, "u16", name, *tag, field_len::U16, true),
			U32_TYPEID => encode_simple_value(source, "u32", name, *tag, field_len::U32, true),
			U64_TYPEID => encode_simple_value(source, "u64", name, *tag, field_len::U64, true),

			I8_TYPEID => encode_simple_value(source, "i8", name, *tag, field_len::U8, true),
			I16_TYPEID => encode_simple_value(source, "i16", name, *tag, field_len::U16, true),
			I32_TYPEID => encode_simple_value(source, "i32", name, *tag, field_len::U32, true),
			I64_TYPEID => encode_simple_value(source, "i64", name, *tag, field_len::U64, true),

			F32_TYPEID => encode_simple_value(source, "f32", name, *tag, field_len::U32, true),
			F64_TYPEID => encode_simple_value(source, "f64", name, *tag, field_len::U64, true),

			VUINT_TYPEID => encode_simple_value(source, "vuint", name, *tag, field_len::VINT, true),
			VINT_TYPEID => encode_simple_value(source, "vint", name, *tag, field_len::VINT, true),
			BINT_TYPEID => encode_simple_value(source, "bint", name, *tag, field_len::LEN, false),

			STR_TYPEID => encode_simple_value(source, "str", name, *tag, field_len::LEN, false),
			ARR_TYPEID => encode_sized_value(source, *tag, |source| {
				write!(source, "encode_arr(data, f_{name}, true, ").unwrap();
				write_value_encoder(source, typeid.item(), ctx);
				source.push(')');
			}),
			MAP_TYPEID => encode_sized_value(source, *tag, |source| {
				write!(source, "encode_map(data, f_{name}, true, ").unwrap();
				write_primitive_encoder(source, typeid.variant, true);
				source.push_str(", ");
				write_value_encoder(source, typeid.item(), ctx);
				source.push(')');
			}),

			INST_TYPEID => encode_simple_value(source, "inst", name, *tag, field_len::U64, false),
			INSTN_TYPEID => encode_compound_value(source, "instN", name, *tag, 12),
			DUR_TYPEID => encode_simple_value(source, "dur", name, *tag, field_len::U64, false),
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
			} else {
				// write mod_path::encode_value
				let file = provider.get(typeid.ns);
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
	for EnumVariant { name: var_name, tag, def, .. } in variants {
		if def.is_some() {
			write!(source, "\t\t{tag} => decode_{name}_{var_name}(data, ind),\n").unwrap();
		} else {
			write!(source, "\t\t{tag} => Some({name}::{var_name}),\n").unwrap();
		}
	}
	source.push_str("\t\t_ => None,\n\t}\n}\n");

	for variant in variants.iter().filter(|v| v.def.is_some()) {
		let EnumVariant { name: var_name, def: Some(def), .. } = variant else { unreachable!() };
		write!(source, "pub fn decode_{name}_{var_name}(data: &[u8], ind: &mut usize)").unwrap();
		write!(source, " -> Option<{name}> {{").unwrap();
		decode_fields(source, def, &format!("{name}::{var_name}"), ctx);
		source.push_str("}\n");
	}

	source.push('\n');
}

fn decode_fields(source: &mut String, def: &StructDef, name: &str, ctx: &Ctx) {
	for Field { name, .. } in def.fields.values() {
		write!(source, "\tlet mut f_{name} = None;\n").unwrap();
	}

	source.push_str("\tfor _ in 0..decode_vuint(data, ind)? {\n");
	source.push_str("\t\tlet header = decode_vuint(data, ind)?;\n");
	source.push_str("\t\tlet tag = header >> 3;\n");

	let mut is_first = true;
	for Field { name, tag, typeid, .. } in def.fields.values() {
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

	for  Field { name, .. } in def.fields.values().filter(|f| !f.is_optional) {
		write!(source, "\tlet Some(f_{name}) = f_{name} else {{ return None; }};\n").unwrap();
	}
	write!(source, "\tSome({name} {{\n").unwrap();
	for  Field { name, .. } in def.fields.values() {
		
			write!(source, "\t\t{name}: f_{name}, \n").unwrap();
		

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
fn write_primitive_decoder(source: &mut String, typeid: u16, is_key: bool) {
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
	if typeid.is_builtin() {
		match typeid.id {
			ARR_TYPEID => {
				source.push_str("|data, ind| decode_arr(data, ind, false, ");
				write_value_decoder(source, typeid.item(), ctx);
				source.push_str(")");
			}
			MAP_TYPEID => {
				source.push_str("|data, ind| decode_map(data, ind, false, ");
				write_primitive_decoder(source, typeid.variant , true);
				source.push_str(", ");
				write_value_decoder(source, typeid.item(), ctx);
				source.push_str(")");
			}
			id => write_primitive_decoder(source, id, false),
		}
	} else {
		if typeid.ns == file.id {
			write!(source, "decode_{}", file.get_by_id(typeid.id).unwrap().name()).unwrap();
		} else {
			let file = provider.get(typeid.ns);
			source.push_str(path_map.get(&typeid.ns).unwrap());
			source.push_str("::decode_");
			source.push_str(file.get_by_id(typeid.id).unwrap().name());
		}
	}
}
fn decode_field(source: &mut String, name: &str, typeid: &TypeId, ctx: &Ctx) {
	let Ctx { file, provider, path_map } = ctx;
	if typeid.is_builtin() {
		match typeid.id {
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
			BINT_TYPEID => decode_simple_value(source, name, "bint"),

			STR_TYPEID => decode_simple_value(source, name, "str"),
			ARR_TYPEID => {
				write!(source, "\t\t\tf_{name} = Some(decode_arr(data, ind, true, ").unwrap();
				write_value_decoder(source, typeid.item(), ctx);
				source.push_str(")?);\n");
			}
			MAP_TYPEID => {
				write!(source, "\t\t\tf_{name} = Some(decode_map(data, ind, true, ").unwrap();
				write_primitive_decoder(source, typeid.variant, true);
				source.push_str(", ");
				write_value_decoder(source, typeid.item(), ctx);
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
			let file = provider.get(typeid.ns);
			source.push_str("\t\t\tdecode_vuint(data, ind)?;\n");
			write!(source, "\t\t\tf_{name} = Some(").unwrap();
			source.push_str(&path_map[&typeid.ns]);
			let type_name = file.get_by_id(typeid.id).unwrap().name();
			write!(source, "::decode_{type_name}(data, ind)?);\n",).unwrap();
		}
	}
}
