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
				write_fields(source, def, used_files, ctx);
				source.push_str("}\n");
			}
			DeclItem::Enum { .. } => write_enum(source, item, used_files, ctx),
		}
	}

	source.push('\n');
}

fn write_enum(source: &mut String, item: &DeclItem, used_files: &mut HashSet<u64>, ctx: &Ctx) {
	let DeclItem::Enum { name, variants, .. } = item else { unreachable!() };

	write!(source, "export type {name} = ").unwrap();

	let (unit_vars, fieldfull_vars): (Vec<_>, Vec<_>) =
		variants.iter().partition(|v| v.def.is_none());

	let mut is_first = true;
	if !unit_vars.is_empty() {
		source.push_str("{ type: ");
		for EnumVariant { name, .. } in unit_vars {
			write!(source, "{}'{name}'", if is_first { "" } else { " | " }).unwrap();
			is_first = false;
		}
		source.push_str(" }");
	}

	for EnumVariant { name, def, .. } in fieldfull_vars {
		source.push_str(if is_first { "" } else { " | " });
		is_first = false;

		write!(source, "{{\n\ttype: '{name}',\n").unwrap();
		write_fields(source, def.as_ref().unwrap(), used_files, ctx);
		source.push('}');
	}
	source.push_str(";\n");
}

fn write_fields(source: &mut String, def: &StructDef, used_files: &mut HashSet<u64>, ctx: &Ctx) {
	for Field { name, typeid, is_optional, .. } in def.fields.values() {
		write!(source, "\t{}{}: ", name, if *is_optional { "?" } else { "" }).unwrap();
		write_type(source, &typeid, used_files, ctx).unwrap();
		source.push_str(",\n");
	}
}

/// convert built-in typeid to a js type
fn resolve_built_in_type(typeid: u16) -> &'static str {
	match typeid {
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

fn write_type(
	source: &mut String, typeid: &TypeId, used_files: &mut HashSet<u64>, ctx: &Ctx,
) -> Option<()> {
	Some(if typeid.is_builtin() {
		match typeid.id {
			ARR_TYPEID => {
				source.push_str("Array<");
				write_type(source, typeid.item(), used_files, ctx);
				source.push('>');
			}
			MAP_TYPEID => {
				source.push_str("Map<");
				source.push_str(resolve_built_in_type(typeid.variant));
				source.push_str(", ");
				write_type(source, typeid.item(), used_files, ctx);
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
		} else {
			// write ns.type_name
			used_files.insert(typeid.ns);
			let file = provider.get(typeid.ns);
			source.push_str("ns_");
			source.push_str(path_map.get(&file.id)?);
			source.push('.');
			source.push_str(file.get_by_id(typeid.id)?.name());
		}
	})
}
