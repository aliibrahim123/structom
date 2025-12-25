use crate::js::Ctx;
use std::{collections::HashSet, fmt::Write};
use structom::{DeclProvider, internal::*};

/// generate type definition for a decleration file
pub fn gen_type_def(source: &mut String, used_files: &mut HashSet<u64>, ctx: &Ctx) {
	let Ctx { file, .. } = ctx;

	for (_, item) in &file.items {
		match item {
			DeclItem::Struct { name, def, .. } => {
				write!(source, "export interface {name} {{\n").unwrap();
				write_struct(source, def, used_files, ctx);
				source.push_str("}\n");
			}
			DeclItem::Enum { .. } => write_enum(source, item, used_files, ctx),
		}
	}

	source.push('\n');
}

/// write enum definition
fn write_enum(source: &mut String, item: &DeclItem, used_files: &mut HashSet<u64>, ctx: &Ctx) {
	let DeclItem::Enum { name, variants, .. } = item else { unreachable!() };

	write!(source, "export type {name} = ").unwrap();

	let (unit_vars, fieldfull_vars): (Vec<_>, Vec<_>) =
		variants.iter().filter_map(|v| v.as_ref()).partition(|v| v.def.is_none());

	let mut is_first = true;
	if !unit_vars.is_empty() {
		source.push_str("{ type: ");
		for EnumVariant { name, .. } in unit_vars {
			source.push_str(if is_first { "" } else { " | " });
			is_first = false;
			write!(source, "'{name}'").unwrap();
		}
		source.push_str(" }");
	}

	for EnumVariant { name, def, .. } in fieldfull_vars {
		source.push_str(if is_first { "" } else { " | " });
		is_first = false;

		write!(source, "{{\n\ttype: '{name}',\n").unwrap();
		write_struct(source, def.as_ref().unwrap(), used_files, ctx);
		source.push('}');
	}
	source.push_str(";\n");
}

/// write struct definition
fn write_struct(source: &mut String, def: &StructDef, used_files: &mut HashSet<u64>, ctx: &Ctx) {
	// write every fields
	for field in def.fields.iter().flat_map(|f| f.as_ref()) {
		write!(source, "\t{}{}: ", field.name, if field.is_optional { "?" } else { "" }).unwrap();
		write_type(source, &field.typeid, used_files, ctx).unwrap();
		source.push_str(",\n");
	}
}

/// convert built-in typeid to a js type
fn resolve_built_in_type(typeid: u8) -> &'static str {
	match typeid as u8 {
		ANY_TYPEID => "Value",
		BOOL_TYPEID => "boolean",
		STR_TYPEID => "string",

		U8_TYPEID | U16_TYPEID | U32_TYPEID | I8_TYPEID | I16_TYPEID | I32_TYPEID => "number",
		U64_TYPEID | I64_TYPEID | VINT_TYPEID | VUINT_TYPEID => "bigint",
		F32_TYPEID | F64_TYPEID => "number",
		BINT_TYPEID => "undefined",

		UUID_TYPEID => "UUID",
		DUR_TYPEID => "Dur",
		INST_TYPEID | INSTN_TYPEID => "Date",

		_ => unreachable!(),
	}
}
/// convert a typeid to a js type
fn write_type(
	source: &mut String, typeid: &TypeId, used_files: &mut HashSet<u64>, ctx: &Ctx,
) -> Option<()> {
	// built-ins
	Some(if typeid.ns == 0 {
		match typeid.id as u8 {
			ARR_TYPEID => {
				source.push_str("Array<");
				// item type
				write_type(source, typeid.item.as_ref()?, used_files, ctx);
				source.push('>');
			}
			MAP_TYPEID => {
				source.push_str("Map<");
				// key type
				source.push_str(resolve_built_in_type(typeid.variant as u8));
				source.push_str(", ");
				// value type
				write_type(source, typeid.item.as_ref()?, used_files, ctx);
				source.push('>');
			}
			id => source.push_str(resolve_built_in_type(id)),
		}
	// user-defined type
	} else {
		let Ctx { file, provider, path_map } = ctx;
		// same file
		if typeid.ns == file.id {
			source.push_str(file.get_by_id(typeid.id)?.name());
		// other file
		} else {
			// write ns.type_name
			used_files.insert(typeid.ns);
			let file = provider.get_by_id(typeid.ns);
			source.push_str("ns_");
			source.push_str(path_map.get(&file.id)?);
			source.push('.');
			source.push_str(file.get_by_id(typeid.id)?.name());
		}
	})
}
