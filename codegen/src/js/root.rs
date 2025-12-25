use crate::Entry;
use std::fmt::Write;

/// generate root mod
pub fn gen_root(inputs: &Vec<Entry>, in_dir: &str) -> String {
	let mut source = String::new();
	// header
	write!(source, "// generated from {in_dir}\n").unwrap();
	source.push_str("import * as enc from \"structom\";\n\n");

	// import modules
	for Entry { resolved_path, .. } in inputs {
		write!(source, "import * as ns_{resolved_path} from \"./{resolved_path}.ts\";\n").unwrap();
	}

	// decode fn
	source.push_str("\nexport function decode<T = any>(data: ArrayBuffer): T {\n");
	source.push_str("\tlet buf = { buf: new Uint8Array(data), pos: 0, view: new DataView(data) };");
	source.push_str("\n\tlet cur = { pos: 0 };\n");

	// match decl_path
	source.push_str("\tswitch (enc.decode_str(buf, cur)) {\n");
	for Entry { decl, rel_path, resolved_path } in inputs {
		// match typeid
		write!(source, "\t\tcase '{rel_path}': {{ switch (enc.decode_vuint(buf, cur)) {{\n")
			.unwrap();
		// try decode
		for (_, item) in &decl.items {
			write!(source, "\t\t\tcase {}: return ns_{resolved_path}", item.typeid()).unwrap();
			write!(source, ".decode_{}(buf, cur) as any;\n", item.name()).unwrap();
		}
		source.push_str("\t\t}};\n");
	}
	source.push_str("\t}\n\treturn undefined as any;\n}\n");

	source
}
