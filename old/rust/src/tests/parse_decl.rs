use crate::{
	DeclFile, DeclProvider, FixedSetProviderRef, ImportError, ParseError, ParseOptions,
	VoidProvider as Void,
	builtins::{
		ARR_TYPEID, BUILT_INS_NAMES, MAP_TYPEID, STR_TYPEID, U8_TYPEID, U16_TYPEID, VUINT_TYPEID,
	},
	internal::{DeclItem, Field, StructDef, TypeId},
	parse_declaration_file,
};

macro_rules! assert_struct {
	($struct:expr, {$($name:literal = $value:pat),+}) => {
		let _struct = $struct;
		$(assert!(matches!(_struct.get_field_by_name($name).unwrap(), $value));)+
	};
}
macro_rules! assert_enum {
	($enum:expr, {$($var:literal = { $(tag = $tag:literal)? $(,)? $($field:literal = $value:pat),* }),+}) => {
		let _enum = $enum;
		$({
			let var = _enum.get_variant_by_name($var).unwrap();
			$(assert_eq!(var.tag, $tag);)?
			$(assert!(matches!(var.def.as_ref().unwrap().get_field_by_name($field).unwrap(), $value));)*
		})+
	};
}
macro_rules! typeid_pat {
	($id: ident) => {
		TypeId { id: $id, .. }
	};
	($id: literal) => {
		TypeId { id: $id, .. }
	};
}
macro_rules! typeid {
	($id:expr) => {
		TypeId::new(0, $id, None)
	};
	(ns: $ns:expr, $id:expr) => {
		TypeId::new($ns, $id, None)
	};
	($id:expr, $item:expr) => {
		TypeId::with_variant(0, $id, 0, Some($item), None)
	};
	($id:expr, $variant:expr, $item:expr) => {
		TypeId::with_variant(0, $id, $variant, Some($item), None)
	};
}

fn get_struct<'a>(file: &'a DeclFile, name: &str) -> &'a StructDef {
	let DeclItem::Struct { def, .. } = file.get_by_name(name).unwrap() else { panic!() };
	def
}

fn parse(source: &str) -> Result<DeclFile, ParseError> {
	let options = &ParseOptions { metadata: true, ..Default::default() };
	parse_declaration_file(source, "test".to_string(), options, &Void {})
}
fn parse_as(source: &str, name: &str) -> Result<DeclFile, ParseError> {
	let options = &ParseOptions { metadata: true, ..Default::default() };
	parse_declaration_file(source, name.to_string(), options, &Void {})
}
fn parse_with(source: &str, provider: &dyn DeclProvider) -> Result<DeclFile, ParseError> {
	let options = &ParseOptions { metadata: true, ..Default::default() };
	parse_declaration_file(source, "test".to_string(), options, provider)
}
fn parse_as_with(
	source: &str, name: &str, provider: &dyn DeclProvider,
) -> Result<DeclFile, ParseError> {
	let options = &ParseOptions { metadata: true, ..Default::default() };
	parse_declaration_file(source, name.to_string(), options, provider)
}

#[test]
fn core() {
	let file = parse("struct A { v: u8 } enum B [3] { C } struct C { v: u8 }").unwrap();
	assert_eq!(file.get_by_name("A").unwrap().typeid(), 0);
	assert_eq!(file.get_by_name("B").unwrap().typeid(), 3);
	assert_eq!(file.get_by_name("C").unwrap().typeid(), 4);

	assert!(parse("struct A { v: u8 } 123",).is_err());

	assert!(parse("struct A [3] { v: u8 } enum B [1] { C }",).is_err());

	let source = "struct A { a: struct { v: struct { v: u8 } }, b: enum { A } } enum B { C } ";
	let file = parse(source).unwrap();
	assert_eq!(file.get_by_id(0).unwrap().name(), "A");
	assert_eq!(file.get_by_id(1).unwrap().name(), "anonymous_struct_1");
	assert_eq!(file.get_by_id(2).unwrap().name(), "anonymous_struct_2");
	assert_eq!(file.get_by_id(3).unwrap().name(), "anonymous_enum_3");
	assert_eq!(file.get_by_id(4).unwrap().name(), "B");
}

#[test]
fn struct_decl() {
	let file = parse("struct A { v: u8 }").unwrap();
	assert_struct!(get_struct(&file, "A"), { "v" = Field { typeid: typeid_pat!(U8_TYPEID), .. } });

	assert!(parse("struct A {}",).is_err());

	let file = parse("struct A { a: vuint, \"b\": str, }").unwrap();
	assert_struct!(get_struct(&file, "A"), {
		"a" = Field { typeid: typeid_pat!(VUINT_TYPEID), .. },
		"b" = Field { typeid: typeid_pat!(STR_TYPEID), .. }
	});

	assert!(parse("struct A { a: u8, a: u8 }",).is_err());

	let file = parse("struct A { v?: u8 }").unwrap();
	assert_struct!(get_struct(&file, "A"), { "v" = Field { is_optional: true, .. } });

	let file = parse("struct A { [2] a: u8, b: u8 }").unwrap();
	assert_struct!(get_struct(&file, "A"), {
		"a" = Field { tag: 2, .. }, "b" = Field { tag: 3, .. }
	});

	assert!(parse("struct A { [2] a: u8, [1] b: u8 }",).is_err());

	let file = parse("struct A [2] { v: u8 }").unwrap();
	assert_eq!(file.get_by_name("A").unwrap().typeid(), 2);

	let file = parse("struct A { a: struct { v: u8 } }").unwrap();
	assert_struct!(get_struct(&file, "anonymous_struct_1"), {
		"v" = Field { typeid: typeid_pat!(U8_TYPEID), .. }
	});
}

#[test]
fn enum_decl() {
	let file = parse("enum A { A, B, C }").unwrap();
	assert_enum!(file.get_by_name("A").unwrap(), {
		"A" = { tag = 0 },
		"B" = { tag = 1 },
		"C" = { tag = 2 }
	});

	assert!(parse("enum A {}",).is_err());

	let file = parse("enum A { A, [2] B, C }").unwrap();
	assert_enum!(file.get_by_name("A").unwrap(), {
		"A" = { tag = 0 },
		"B" = { tag = 2 },
		"C" = { tag = 3 }
	});

	assert!(parse("enum A { A, [3] B, [1] C }",).is_err());

	let file = parse("enum A { A, B { v: u8 }, C }").unwrap();
	assert_enum!(file.get_by_name("A").unwrap(), {
		"B" = { "v" = Field { typeid: typeid_pat!(U8_TYPEID), .. } }
	});

	let file = parse("enum A [2] { A, B, C }").unwrap();
	assert_eq!(file.get_by_name("A").unwrap().typeid(), 2);

	let file = parse("enum A { A, B { v: enum { A, B, C } } }").unwrap();
	assert_enum!(file.get_by_name("anonymous_enum_1").unwrap(), {
		"A" = { tag = 0 },
		"B" = { tag = 1 },
		"C" = { tag = 2 }
	});
}

#[test]
fn type_id() {
	fn get(file: &DeclFile) -> &TypeId {
		let DeclItem::Struct { def, .. } = file.get_by_name("A").unwrap() else { panic!() };
		&def.get_field_by_name("v").unwrap().typeid
	}

	for (id, name) in BUILT_INS_NAMES.iter() {
		if matches!(*name, "arr" | "map") {
			continue;
		}
		let file = parse(&format!("struct A {{ v: {name} }}")).unwrap();

		assert_eq!(get(&file), &typeid!(*id));
	}

	let file = parse("struct A { v: arr<u8> }").unwrap();
	assert_eq!(get(&file), &typeid!(ARR_TYPEID, typeid!(U8_TYPEID)));

	let file = parse("struct A { v: map<u8, str> }").unwrap();
	assert_eq!(get(&file), &typeid!(MAP_TYPEID, U8_TYPEID, typeid!(STR_TYPEID)));

	let file = parse("enum B { A, B, C } struct A { v: B }").unwrap();
	assert_eq!(get(&file), &typeid!(ns: file.id, 0));

	let other = parse_as("enum B { A, B, C }", "other").unwrap();
	let provider = FixedSetProviderRef::new(&[&other]);
	let file = parse_with("import \"other\" struct A { v: B }", &provider).unwrap();
	assert_eq!(get(&file), &typeid!(ns: other.id, 0));

	let file =
		parse_with("import \"other\" as others struct A { v: others.B }", &provider).unwrap();
	assert_eq!(get(&file), &typeid!(ns: other.id, 0));

	let file =
		parse_with("import \"other\" struct B { v: u8 } struct A { v: B }", &provider).unwrap();
	assert_eq!(get(&file), &typeid!(ns: file.id, 0));

	let file = parse(r#"struct A { v: struct { v: u8 } }"#).unwrap();
	assert_eq!(get(&file), &typeid!(ns: file.id, 1));

	let file = parse(r#"struct A { v: enum { A, B, C } }"#).unwrap();
	assert_eq!(get(&file), &typeid!(ns: file.id, 1));

	let file = parse(r#"struct A { v: @m1("a") @m2("b") u8 }"#).unwrap();
	assert_eq!(
		**get(&file).metadata.as_ref().unwrap(),
		&[("m1".to_string(), "a".to_string()), ("m2".to_string(), "b".to_string())]
	);

	assert!(parse(r#"struct A { v: Unknown }"#,).is_err());
}

#[test]
fn import() {
	assert!(parse_with("import \"unknown\"", &Void {}).is_err());

	let file1 = parse_as("struct A { v: u8 }", "file1").unwrap();
	let provider = FixedSetProviderRef::new(&[&file1]);

	assert!(parse_with("import \"file1\" struct A { v: u8 }", &provider).is_ok());

	assert!(parse_with("import \"file1\" import \"file1\"", &provider).is_err());

	assert!(parse_with("import \"file1\" as file1 import \"file1\" as file1", &provider).is_err());

	assert!(parse_with("import \"file1\" as A struct A { v: u8 }", &provider).is_err());

	assert!(parse_with("struct A { v: u8 } import \"file1\" as A", &provider).is_err());

	let file = parse_with("import \"file1\" struct A { v: u16 }", &provider).unwrap();
	assert_struct!(get_struct(&file, "A"), { "v" = Field { typeid: typeid_pat!(U16_TYPEID), .. } });

	struct Provider1 {}
	impl DeclProvider for Provider1 {
		fn load<'a>(&'a self, _name: &str) -> Result<&'a DeclFile, ImportError> {
			return Err(ImportError::Other("test".to_string()));
		}
		fn get<'a>(&'a self, _id: u64) -> &'a DeclFile {
			panic!()
		}
	}
	assert!(parse_with("import \"\"", &Provider1 {}).is_err());

	let file2 = parse_as("struct A { v: u8 }", "a/b/c").unwrap();
	let provider = FixedSetProviderRef::new(&[&file2]);

	assert!(parse_as_with("import \"./c\" struct B { v: A }", "a/b/d", &provider).is_ok());
	assert!(parse_as_with("import \"../b/c\" struct B { v: A }", "a/e/d", &provider).is_ok());
	assert!(
		parse_as_with("import \"../../../a/b/c\" struct B { v: A }", "d/e/f/g", &provider).is_ok()
	);
	assert!(parse_as_with("import \"../../../a/b/c\" struct B { v: A }", "d", &provider).is_ok());
}
