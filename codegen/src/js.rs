mod encoding;
mod root;
mod type_def;

use std::{
	collections::HashMap,
	fmt::Write,
	fs::write,
	path::{Path, absolute},
};
use structom::{DeclFile, FSProvider};

use crate::js::{root::gen_root, type_def::gen_type_def};
use crate::utils::errors;
use crate::{Entry, js::encoding::gen_encoding};

/// DeclFile.id => mod path
pub type PathMap<'a> = HashMap<u64, &'a String>;

/// generation common state
pub struct Ctx<'a> {
	file: &'a DeclFile,
	provider: &'a FSProvider,
	path_map: &'a PathMap<'a>,
}

/// generate serialization code for rust lang
pub fn to_js(
	inputs: &Vec<Entry>, in_dir: &str, out_dir: &Path, provider: &FSProvider,
) -> Result<(), String> {
	// prepare path map
	let mut path_map = HashMap::new();
	for Entry { resolved_path, decl, .. } in inputs {
		path_map.insert(decl.id, resolved_path);
	}
	// generate files
	for Entry { resolved_path, rel_path, decl, .. } in inputs {
		let mut source = String::new();
		let ctx = Ctx { file: decl, provider, path_map: &path_map };

		// write header and imports
		write!(source, "// generated from file: {}\n\n", &decl.name).unwrap();
		source.push_str("import type { Value, UUID, Dur, Buffer, Cursor } from \"structom\";\n");
		source.push_str("import * as enc from \"structom\";\n");

		// gen type def
		let mut used_files = Vec::new();
		let mut type_def_source = String::new();
		gen_type_def(&mut type_def_source, &mut used_files, &ctx);

		// write refrenced decleration imports
		for id in used_files {
			let path = path_map.get(&id).unwrap();
			write!(source, "import * as ns_{path} from \"./{path}.ts\";\n").unwrap();
		}
		source.push('\n');
		source.push_str(&type_def_source);

		gen_encoding(&mut source, rel_path, &ctx);

		let output = absolute(out_dir.join(resolved_path).with_extension("ts")).unwrap();
		write(&output, source).map_err(errors::write_file(&output.display()))?;
	}

	// generate root mod
	let root_path = absolute(out_dir.join("index.ts")).unwrap();
	write(&root_path, gen_root(inputs, in_dir))
		.map_err(|_| format!("unable to write file \"{}\"", root_path.display()))?;

	Ok(())
}
