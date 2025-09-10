use crate::{Entry, rust::PathMap};
use std::fmt::Write;

/// generate root mod
pub fn gen_root(inputs: &Vec<Entry>, in_dir: &str) -> String {
	let mut source = String::new();
	// header
	write!(source, "// generated from {in_dir}\n").unwrap();
	source.push_str("use std::any::Any;\n");
	source.push_str("use structom::encoding::*;\n\n");

	// link modules
	for Entry { resolved_path, .. } in inputs {
		write!(source, "pub mod {resolved_path};\n").unwrap();
	}

	// decode fn
	source.push_str("\npub fn decode(data: &[u8]) -> Option<Box<dyn Any>> {\n");
	source.push_str("\tlet mut ind = 0;\n");
	// match decl_path
	source.push_str("\tlet value: Box<dyn Any> = match decode_str(data, &mut ind)?.as_str() {\n");
	for Entry { decl, rel_path, resolved_path } in inputs {
		// match typeid
		write!(source, "\t\t{rel_path:?} => match decode_vuint(data, &mut ind)? {{\n").unwrap();
		// try decode
		for (_, item) in &decl.items {
			write!(source, "\t\t\t{} => Box::new({resolved_path}", item.typeid()).unwrap();
			write!(source, "::decode_{}(data, &mut ind)?),\n", item.name()).unwrap();
		}
		source.push_str("\t\t\t_ => return None,\n");
		source.push_str("\t\t},\n");
	}
	source.push_str("\t\t_ => return None,\n\t};\n");
	// check if remains data
	source.push_str("\tif ind != data.len() { None } else { Some(value) }\n");
	source.push_str("}\n");

	source
}
