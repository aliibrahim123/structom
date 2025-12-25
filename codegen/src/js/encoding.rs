use crate::{
	js::Ctx,
	utils::{encode_header, new_size_ind},
};
use std::fmt::Write;

use structom::{DeclProvider, internal::*};

/// generate encoding functions
pub fn gen_encoding(source: &mut String, rel_path: &str, ctx: &Ctx) {
	let Ctx { file, .. } = ctx;

	for (_, item) in &file.items {
		write!(source, "export function encode_{0}(value: {0}) {{\n", item.name()).unwrap();
		source.push_str("\tlet _buf = new Uint8Array(256);\n");
		source.push_str("\tlet buf = { buf: _buf, pos: 0, view: new DataView(_buf.buffer) };\n");
		source.push_str("\tenc.encode_u8_arr(buf, [");
		encode_header(source, rel_path, item.typeid() as u64);
		source.push_str("]);\n");
		write!(source, "\tencode_int_{}(buf, value);\n", item.name()).unwrap();
		source.push_str("\treturn buf.buf.slice(0, buf.pos);\n");
		source.push_str("}\n\n");

		match item {
			DeclItem::Struct { name, def, .. } => {
				#[rustfmt::skip]
				write!(source, "export function encode_int_{name}(buf: Buffer, value: {name}) {{\n")
					.unwrap();
				encode_struct(source, def, ctx);
				source.push_str("}\n");

				write!(source, "export function decode_{name}(buf: Buffer, cur: Cursor)").unwrap();
				write!(source, ": {name} {{\n").unwrap();
				write!(source, "let value = {{}} as any as {name};\n").unwrap();
				decode_struct(source, def, ctx);
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
	write!(source, "export function encode_int_{name}(buf: Buffer, value: {name}) {{\n").unwrap();
	// encode based on variant
	source.push_str("\tswitch (value.type) {");
	for variant in variants.iter().filter_map(|v| v.as_ref()) {
		let EnumVariant { name: var_name, tag, .. } = variant;
		// has fields
		if variant.def.is_some() {
			write!(source, "\n\t\tcase '{var_name}':  {{\n").unwrap();
			write!(source, "\t\t\tenc.encode_vuint(buf, {tag});\n",).unwrap();
			write!(source, "\t\t\treturn encode_{name}_{var_name}(buf, value);\n",).unwrap();
			source.push_str("\t\t}");
		// case unit enum
		} else {
			write!(source, "\n\t\tcase '{var_name}': return enc.encode_vuint(buf, {tag});",)
				.unwrap();
		}
	}
	source.push_str("\n\t}\n}\n");

	// generate encode functions for variants with fields
	for variant in variants.iter().filter_map(|v| v.as_ref().filter(|v| v.def.is_some())) {
		let EnumVariant { name: var_name, def: Some(def), .. } = variant else { unreachable!() };
		write!(source, "export function encode_{name}_{var_name}(buf: Buffer, ").unwrap();
		write!(source, "value: {name} & {{ type: '{var_name}' }}) {{\n").unwrap();
		encode_struct(source, def, ctx);
		source.push_str("}\n");
	}
}

/// generate encoding function for struct
fn encode_struct(source: &mut String, def: &StructDef, ctx: &Ctx) {
	// split fields into optional and required
	#[rustfmt::skip]
	let (opt_fields, req_fields): (Vec<_>, Vec<_>) = 
		def.fields.iter().filter_map(|f| f.as_ref()).partition(|field| field.is_optional);

	// encode fields count
	// case only required, direct count
	if opt_fields.is_empty() {
		write!(source, "\tenc.encode_vuint(buf, {});\n", req_fields.len()).unwrap();
	// case there optionals, count them if have value
	} else {
		write!(source, "\tenc.encode_vuint(buf, {}\n", req_fields.len()).unwrap();
		for field in &opt_fields {
			write!(source, "\t\t+ ('{}' in value ? 1 : 0)\n", field.name).unwrap();
		}
		source.push_str("\t);\n");
	}

	// encode required fields
	for field in req_fields {
		encode_field(source, &field, ctx);
	}
	// encode optional fields if have value
	for field in opt_fields {
		write!(source, "\tif ('{}' in value) {{\n", field.name).unwrap();
		encode_field(source, &field, ctx);
		source.push_str("\t}\n");
	}
}
// encode code for common field value types
// encode header then value
fn encode_simple_value(source: &mut String, ty: &str, name: &str, tag: u32, size: u32) {
	write!(source, "\tenc.encode_vuint(buf, {});\n", tag << 3 | size).unwrap();
	write!(source, "\tenc.encode_{ty}(buf, value.{name});\n").unwrap();
}
fn encode_compound_value(source: &mut String, ty: &str, name: &str, tag: u32, size: u32) {
	// has len field with predefined value
	write!(source, "\tenc.encode_vuint(buf, {});\n", tag << 3 | 0b101).unwrap();
	write!(source, "\tenc.encode_vuint(buf, {size});\n").unwrap();
	write!(source, "\tenc.encode_{ty}(buf, value.{name});\n").unwrap();
}
fn encode_sized_value(source: &mut String, tag: u32, encoder: impl Fn(&mut String)) {
	let size_ind_inst = new_size_ind();
	write!(source, "\tenc.encode_vuint(buf, {});\n", tag << 3 | 0b101).unwrap();
	// reserve 2 byte space for len
	write!(source, "\tlet size_ind_{size_ind_inst} = buf.pos;\n").unwrap();
	source.push_str("\tenc.encode_u8_arr(buf, [0, 0]);\n\t");
	// encode value
	encoder(source);
	// encode len, expand it if required
	source.push_str(";\n\tenc.encode_vuint_pre_aloc(buf, buf.pos - size_ind_");
	write!(source, "{size_ind_inst} - 2, size_ind_{size_ind_inst}, 2);\n").unwrap();
}
// generate fn that encode primitive types
fn write_primitive_encoder(source: &mut String, typeid: u8) {
	match typeid {
		ANY_TYPEID => source.push_str("enc.encode_any"),

		BOOL_TYPEID => source.push_str("enc.encode_bool"),
		U8_TYPEID => source.push_str("enc.encode_u8"),
		U16_TYPEID => source.push_str("enc.encode_u16"),
		U32_TYPEID => source.push_str("enc.encode_u32"),
		U64_TYPEID => source.push_str("enc.encode_u64"),

		I8_TYPEID => source.push_str("enc.encode_i8"),
		I16_TYPEID => source.push_str("enc.encode_i16"),
		I32_TYPEID => source.push_str("enc.encode_i32"),
		I64_TYPEID => source.push_str("enc.encode_i64"),

		VUINT_TYPEID => source.push_str("enc.encode_vuint"),
		VINT_TYPEID => source.push_str("enc.encode_vint"),
		BINT_TYPEID => source.push_str("undefined"),

		F32_TYPEID => source.push_str("enc.encode_f32"),
		F64_TYPEID => source.push_str("enc.encode_f64"),

		STR_TYPEID => source.push_str("enc.encode_str"),

		INST_TYPEID => source.push_str("enc.encode_inst"),
		INSTN_TYPEID => source.push_str("enc.encode_instN"),
		DUR_TYPEID => source.push_str("enc.encode_dur"),
		UUID_TYPEID => source.push_str("enc.encode_uuid"),
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
				source.push_str("(buf, value) => enc.encode_arr(buf, value, ");
				// item encoder
				write_value_encoder(source, typeid.item.as_ref().unwrap(), ctx);
				source.push_str(")");
			}
			MAP_TYPEID => {
				source.push_str("(buf, value) => enc.encode_map(buf, value, ");
				// key encoder
				write_primitive_encoder(source, typeid.variant as u8);
				source.push_str(", ");
				// value encoder
				write_value_encoder(source, typeid.item.as_ref().unwrap(), ctx);
				source.push_str(")");
			}
			id => write_primitive_encoder(source, id),
		}
	// user defined
	} else {
		// same file
		if typeid.ns == file.id {
			write!(source, "encode_int_{}", file.get_by_id(typeid.id).unwrap().name()).unwrap();
		// different file
		} else {
			// write ns.encode_type
			let file = provider.get_by_id(typeid.ns);
			source.push_str("ns_");
			source.push_str(path_map.get(&typeid.ns).unwrap());
			source.push_str(".encode_int_");
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
				write!(source, "enc.encode_any(buf, value.{name})").unwrap()
			}),

			BOOL_TYPEID => encode_simple_value(source, "bool", name, *tag, 0b000),

			U8_TYPEID => encode_simple_value(source, "u8", name, *tag, 0b000),
			U16_TYPEID => encode_simple_value(source, "u16", name, *tag, 0b001),
			U32_TYPEID => encode_simple_value(source, "u32", name, *tag, 0b010),
			U64_TYPEID => encode_simple_value(source, "u64", name, *tag, 0b011),

			I8_TYPEID => encode_simple_value(source, "i8", name, *tag, 0b000),
			I16_TYPEID => encode_simple_value(source, "i16", name, *tag, 0b001),
			I32_TYPEID => encode_simple_value(source, "i32", name, *tag, 0b010),
			I64_TYPEID => encode_simple_value(source, "i64", name, *tag, 0b011),

			F32_TYPEID => encode_simple_value(source, "f32", name, *tag, 0b010),
			F64_TYPEID => encode_simple_value(source, "f64", name, *tag, 0b011),

			VUINT_TYPEID => encode_simple_value(source, "vuint", name, *tag, 0b100),
			VINT_TYPEID => encode_simple_value(source, "vint", name, *tag, 0b100),
			BINT_TYPEID => (),

			STR_TYPEID => encode_simple_value(source, "str", name, *tag, 0b101),
			ARR_TYPEID => encode_sized_value(source, *tag, |source| {
				write!(source, "enc.encode_arr(buf, value.{name}, ").unwrap();
				// item encoder
				write_value_encoder(source, typeid.item.as_ref().unwrap(), ctx);
				source.push_str(", true)");
			}),
			MAP_TYPEID => encode_sized_value(source, *tag, |source| {
				write!(source, "enc.encode_map(buf, value.{name}, ").unwrap();
				// key encoder
				write_primitive_encoder(source, typeid.variant as u8);
				source.push_str(", ");
				// value encoder
				write_value_encoder(source, typeid.item.as_ref().unwrap(), ctx);
				source.push_str(", true)");
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
				write!(source, "encode_int_{item_name}(buf, value.{name})",).unwrap();
			// different file
			} else {
				// write ns.encode_value
				let file = provider.get_by_id(typeid.ns);
				let item_name = file.get_by_id(typeid.id).unwrap().name();
				source.push_str("ns_");
				source.push_str(path_map.get(&typeid.ns).unwrap());
				write!(source, ".encode_int_{item_name}(buf, value.{name})").unwrap();
			}
		});
	};
}

/// generate decoding function for enum
fn decode_enum(source: &mut String, item: &DeclItem, ctx: &Ctx) {
	let DeclItem::Enum { name, variants, .. } = item else { unreachable!() };

	// main function
	write!(source, "export function decode_{name}(buf: Buffer, cur: Cursor): {name} {{\n").unwrap();

	// switch on tag
	write!(source, "\tswitch (enc.decode_vuint(buf, cur) as number) {{\n").unwrap();
	for variant in variants.iter().filter_map(|v| v.as_ref()) {
		let EnumVariant { name: var_name, tag, def, .. } = variant;
		if def.is_some() {
			write!(source, "\t\tcase {tag}: return decode_{name}_{var_name}(buf, cur);\n").unwrap();
		} else {
			write!(source, "\t\tcase {tag}: return {{ type: '{var_name}' }};\n").unwrap();
		}
	}
	source.push_str("\t}\n\treturn undefined as any;\n}\n");

	// generate decode functions for variants with fields
	for variant in variants.iter().filter_map(|v| v.as_ref().filter(|v| v.def.is_some())) {
		let EnumVariant { name: var_name, def: Some(def), .. } = variant else { unreachable!() };
		write!(source, "export function decode_{name}_{var_name}(buf: Buffer, cur: Cursor) {{\n")
			.unwrap();
		write!(source, "let value = {{ type: '{var_name}' }} as any ").unwrap();
		write!(source, "as {name} & {{ type: '{var_name}' }};\n").unwrap();
		decode_struct(source, def, ctx);
		source.push_str("}\n");
	}

	source.push('\n');
}

/// generate decoding function for struct
fn decode_struct(source: &mut String, def: &StructDef, ctx: &Ctx) {
	let fields = def.fields.iter().filter_map(|f| f.as_ref()).collect::<Vec<_>>();

	// header
	source.push_str("\tlet count = enc.decode_vuint(buf, cur);\n");
	source.push_str("\tfor (let i = 0; i < count; i++) {\n");
	source.push_str("\t\tlet header = enc.decode_vuint(buf, cur) as number;\n");
	source.push_str("\t\tlet tag = header >> 3;\n");

	// fields
	let mut is_first = true;
	for field in &fields {
		let Field { name, tag, typeid, .. } = field;
		if is_first {
			write!(source, "\t\tif (tag === {tag}) {{\n").unwrap();
		} else {
			write!(source, " else if (tag === {tag}) {{\n").unwrap();
		}
		decode_field(source, name, typeid, ctx);
		source.push_str("\t\t}");
		is_first = false;
	}

	// skip field if nout found
	source.push_str(" else { enc.skip_field(buf, cur, header) }\n");
	source.push_str("\t}\n");

	source.push_str("\treturn value;\n");
}

// encode code for common field value types
fn decode_simple_value(source: &mut String, name: &str, ty: &str) {
	write!(source, "\t\t\tvalue.{name} = enc.decode_{ty}(buf, cur);\n").unwrap();
}
fn decode_compound_value(source: &mut String, name: &str, ty: &str) {
	source.push_str("\t\t\tenc.decode_vuint(buf, cur);\n");
	write!(source, "\t\t\tvalue.{name} = enc.decode_{ty}(buf, cur);\n").unwrap();
}
/// generate fn that decode primitive types
fn write_primitive_decoder(source: &mut String, typeid: u8) {
	match typeid {
		ANY_TYPEID => source.push_str("enc.decode_any"),

		BOOL_TYPEID => source.push_str("enc.decode_bool"),
		U8_TYPEID => source.push_str("enc.decode_u8"),
		U16_TYPEID => source.push_str("enc.decode_u16"),
		U32_TYPEID => source.push_str("enc.decode_u32"),
		U64_TYPEID => source.push_str("enc.decode_u64"),

		I8_TYPEID => source.push_str("enc.decode_i8"),
		I16_TYPEID => source.push_str("enc.decode_i16"),
		I32_TYPEID => source.push_str("enc.decode_i32"),
		I64_TYPEID => source.push_str("enc.decode_i64"),

		VUINT_TYPEID => source.push_str("enc.decode_vuint"),
		VINT_TYPEID => source.push_str("enc.decode_vint"),

		F32_TYPEID => source.push_str("enc.decode_f32"),
		F64_TYPEID => source.push_str("enc.decode_f64"),

		STR_TYPEID => source.push_str("enc.decode_str"),

		INST_TYPEID => source.push_str("enc.decode_inst"),
		INSTN_TYPEID => source.push_str("enc.decode_instN"),
		DUR_TYPEID => source.push_str("enc.decode_dur"),
		UUID_TYPEID => source.push_str("enc.decode_uuid"),
		_ => (),
	}
}
/// write a fn that decode a value
fn write_value_decoder(source: &mut String, typeid: &TypeId, ctx: &Ctx) {
	let Ctx { file, path_map, provider } = ctx;
	if typeid.ns == 0 {
		match typeid.id as u8 {
			ARR_TYPEID => {
				source.push_str("(buf, cur) => enc.decode_arr(buf, cur, ");
				write_value_decoder(source, typeid.item.as_ref().unwrap(), ctx);
				source.push_str(")");
			}
			MAP_TYPEID => {
				source.push_str("(buf, cur) => enc.decode_map(buf, cur, ");
				write_primitive_decoder(source, typeid.variant as u8);
				source.push_str(", ");
				write_value_decoder(source, typeid.item.as_ref().unwrap(), ctx);
				source.push_str(")");
			}
			id => write_primitive_decoder(source, id),
		}
	} else {
		if typeid.ns == file.id {
			write!(source, "decode_{}", file.get_by_id(typeid.id).unwrap().name()).unwrap();
		} else {
			let file = provider.get_by_id(typeid.ns);
			source.push_str("ns_");
			source.push_str(path_map.get(&typeid.ns).unwrap());
			source.push_str(".decode_");
			source.push_str(file.get_by_id(typeid.id).unwrap().name());
		}
	}
}
/// decode code for one field
fn decode_field(source: &mut String, name: &str, typeid: &TypeId, ctx: &Ctx) {
	let Ctx { file, provider, path_map } = ctx;
	// builtins
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
			BINT_TYPEID => (),

			STR_TYPEID => decode_simple_value(source, name, "str"),
			INST_TYPEID => decode_simple_value(source, name, "inst"),
			INSTN_TYPEID => decode_compound_value(source, name, "instN"),
			DUR_TYPEID => decode_simple_value(source, name, "dur"),
			UUID_TYPEID => decode_compound_value(source, name, "uuid"),

			ARR_TYPEID => {
				write!(source, "\t\t\tvalue.{name} = enc.decode_arr(buf, cur, ").unwrap();
				write_value_decoder(source, typeid.item.as_ref().unwrap(), ctx);
				source.push_str(", true);\n");
			}
			MAP_TYPEID => {
				write!(source, "\t\t\tvalue.{name} = enc.decode_map(buf, cur, ").unwrap();
				write_primitive_decoder(source, typeid.variant as u8);
				source.push_str(", ");
				write_value_decoder(source, typeid.item.as_ref().unwrap(), ctx);
				source.push_str(", true);\n");
			}
			_ => (),
		}
	// user defined
	} else {
		// same file
		if typeid.ns == file.id {
			source.push_str("\t\t\tenc.decode_vuint(buf, cur);\n");
			let type_name = file.get_by_id(typeid.id).unwrap().name();
			write!(source, "\t\t\tvalue.{name} = decode_{type_name}(buf, cur);\n",).unwrap();
		// different file
		} else {
			let file = provider.get_by_id(typeid.ns);
			source.push_str("\t\t\tenc.decode_vuint(buf, cur);\n");
			write!(source, "\t\t\tvalue.{name} = ns_").unwrap();
			source.push_str(path_map.get(&typeid.ns).unwrap());
			let type_name = file.get_by_id(typeid.id).unwrap().name();
			write!(source, ".decode_{type_name}(buf, cur);\n",).unwrap();
		}
	}
}
