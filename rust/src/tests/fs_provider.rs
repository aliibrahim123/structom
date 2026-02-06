use std::{
	fs::{create_dir, remove_dir_all, write},
	path::Path,
};

use chrono::Utc;

use crate::{FSProvider, ParseOptions};

#[test]
fn test() {
	let root = &format!("./fs_decl_test_{}", Utc::now().timestamp());
	let root = Path::new(root);
	assert!(FSProvider::new(root).is_err());

	if let Err(_) = create_dir(root) {
		return;
	};

	let file = r#"
		struct A {
			a: vuint,
			b: str
		}
		enum B { A, B, C }
	"#;
	_ = write(Path::join(root, "commons.stomd"), file);

	let file = r#"i am wrong"#;
	_ = write(Path::join(root, "wrong.stomd"), file);

	_ = create_dir(Path::join(root, "./decls"));
	let file = r#"
		import "../commons.stomd"
		struct C {
			v: B
		}
	"#;
	_ = write(Path::join(root, "./decls/a.stomd"), file);

	let file = r#"
		import "commons.stomd"
		import "./a.stomd"
		struct D {
			a: C,
			b: A,
		}
	"#;
	_ = write(Path::join(root, "./decls/b.stomd"), file);

	let Ok(provider) = FSProvider::new(root) else { panic!("provider didnt init") };

	assert!(provider.load_file("commons.stomd").is_ok());
	assert!(provider.load_file("wrong.stomd").is_err());

	let parse = |source| crate::parse(source, &ParseOptions::default(), &provider);
	assert!(parse("import \"./decls/a.stomd\" C { v: C }").is_ok());
	assert!(parse("import \"./decls/b.stomd\" D { a: { v: A }, b: { a: 1, b: \"a\" } }").is_ok());

	_ = remove_dir_all(root);
}
