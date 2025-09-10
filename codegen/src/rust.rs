mod encoding;
mod root;
mod type_def;
mod value;

use std::{
	collections::HashMap,
	fmt::Write,
	fs::write,
	path::{Path, absolute},
};

use structom::{DeclFile, FSProvider};

use crate::{
	Entry,
	rust::{encoding::gen_encoding, root::gen_root, type_def::gen_type_def, value::gen_value_conv},
};

/// DeclFile.id => mod path
pub type PathMap = HashMap<u64, String>;

/// generation common state
pub struct Ctx<'a> {
	file: &'a DeclFile,
	provider: &'a FSProvider,
	path_map: &'a PathMap,
}

/// generate serialization code for rust lang
pub fn to_rust(
	inputs: &Vec<Entry>, in_dir: &str, out_dir: &Path, provider: &FSProvider,
) -> Result<(), String> {
	// prepare path map
	let mut path_map = HashMap::new();
	for Entry { resolved_path, decl, .. } in inputs {
		path_map.insert(decl.id, ["super", &resolved_path].join("::"));
	}

	// generate files
	for Entry { resolved_path, rel_path, decl } in inputs {
		let mut source = String::new();
		let ctx = Ctx { file: decl, provider, path_map: &path_map };

		// write header and imports
		write!(source, "// generated from file: {}\n\n", &decl.name).unwrap();
		source.push_str("use std::collections::HashMap;\n");
		source.push_str("use structom::{Value, Key, encoding::*};\n\n");

		gen_type_def(&mut source, rel_path, &ctx);
		gen_encoding(&mut source, &ctx);
		gen_value_conv(&mut source, &ctx);

		let output = absolute(out_dir.join(resolved_path).with_extension("rs")).unwrap();
		write(&output, source)
			.map_err(|_| format!("unable to write file \"{}\"", output.display()))?;
	}

	// generate root mod
	let root_path = absolute(out_dir.join("mod.rs")).unwrap();
	write(&root_path, gen_root(inputs, in_dir))
		.map_err(|_| format!("unable to write file \"{}\"", root_path.display()))?;

	Ok(())
}
