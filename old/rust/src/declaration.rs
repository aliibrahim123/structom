use std::{collections::HashMap, fmt::Display};

use crate::{
	builtins::{ARR_TYPEID, BUILT_INS_NAMES, MAP_TYPEID},
	errors::ImportError,
};

/// encapsulate the content of a decleration file.
///
/// can only be created through [`parse_declaration_file`](crate::parse_declaration_file).
#[derive(Debug)]
pub struct DeclFile {
	/// name of the file, passed though name argument in [`parse_declaration_file`](crate::parse_declaration_file)
	pub name: String,
	/// a globally unique identifier for the file
	pub id: u64,
	#[doc(hidden)]
	pub items: HashMap<u16, DeclItem>,
	pub(crate) items_by_name: HashMap<String, u16>,
}

#[derive(Debug)]
pub struct TypeId {
	/// identifier of the owner declaration file
	pub ns: u64,
	/// identifier inside the owner declaration file
	pub id: u16,
	pub variant: u16,
	pub item: Option<Box<TypeId>>,
	pub metadata: Option<Box<Vec<(String, String)>>>,
}

#[derive(Debug)]
pub struct Field {
	pub name: String,
	pub tag: u32,
	pub typeid: TypeId,
	pub is_optional: bool,
}
#[derive(Default, Debug)]
pub struct StructDef {
	pub fields: HashMap<u32, Field>,
	pub fields_by_name: HashMap<String, u32>,
	pub required_fields: u32,
}

#[derive(Debug)]
pub struct EnumVariant {
	pub name: String,
	pub tag: u32,
	pub def: Option<StructDef>,
}

#[derive(Debug)]
pub enum DeclItem {
	Struct {
		name: String,
		typeid: u16,
		def: StructDef,
	},
	Enum {
		name: String,
		typeid: u16,
		variants: Vec<EnumVariant>,
		variants_by_id: HashMap<u32, usize>,
		variants_by_name: HashMap<String, usize>,
	},
}

/// trait for types providing decleration files.
///
/// decleration providers are used by functions that need access to decleration files.
///
/// decleration providers can provide decleration files from any source, guaranteed to be valid and the same for every access.
pub trait DeclProvider {
	/// get a decleration file by its id.
	///
	/// this method can not fail, it is used for decleration files that were created before.
	fn get(&self, id: u64) -> &DeclFile;

	/// load a decleration file by its name.
	///   
	/// this method return `ImportError` on fail, when the requested decleration file can not be found or it cant be parsed.
	fn load<'a>(&'a self, name: &str) -> Result<&'a DeclFile, ImportError>;
}

impl DeclFile {
	pub(crate) fn new(name: String) -> Self {
		static mut DECLARE_ID_COUNTER: u64 = 0;
		let id = unsafe {
			DECLARE_ID_COUNTER += 1;
			DECLARE_ID_COUNTER
		};
		DeclFile { name, id, items: HashMap::new(), items_by_name: HashMap::new() }
	}

	pub(crate) fn add_item(&mut self, item: DeclItem) {
		self.items_by_name.insert(item.name().to_string(), item.typeid());
		self.items.insert(item.typeid(), item);
	}

	#[doc(hidden)]
	pub fn get_by_name(&self, name: &str) -> Option<&DeclItem> {
		self.items.get(self.items_by_name.get(name)?)
	}
	#[doc(hidden)]
	pub fn get_by_id(&self, id: u16) -> Option<&DeclItem> {
		self.items.get(&id)
	}
}

impl PartialEq<DeclFile> for DeclFile {
	fn eq(&self, other: &DeclFile) -> bool {
		self.id == other.id
	}
}

impl DeclItem {
	pub fn name(&self) -> &str {
		match self {
			Self::Struct { name, .. } => name,
			Self::Enum { name, .. } => name,
		}
	}
	pub fn typeid(&self) -> u16 {
		match self {
			Self::Struct { typeid, .. } => *typeid,
			Self::Enum { typeid, .. } => *typeid,
		}
	}

	pub fn new_enum(name: String, typeid: u16) -> Self {
		Self::Enum {
			name,
			typeid,
			variants: Vec::new(),
			variants_by_id: HashMap::new(),
			variants_by_name: HashMap::new(),
		}
	}

	pub fn add_variant(&mut self, variant: EnumVariant) {
		match self {
			Self::Enum { variants, variants_by_id, variants_by_name, .. } => {
				let ind = variants.len();
				variants_by_name.insert(variant.name.to_string(), ind);
				variants_by_id.insert(variant.tag, ind);
				variants.push(variant);
			}
			_ => panic!("why"),
		}
	}
	pub fn get_variant_by_name(&self, name: &str) -> Option<&EnumVariant> {
		match self {
			Self::Enum { variants, variants_by_name, .. } => {
				variants.get(*variants_by_name.get(name)?)
			}
			_ => None,
		}
	}
	pub fn get_variant_by_id(&self, tag: u32) -> Option<&EnumVariant> {
		match self {
			Self::Enum { variants, variants_by_id, .. } => variants.get(*variants_by_id.get(&tag)?),
			_ => None,
		}
	}
}

impl StructDef {
	pub fn add_field(&mut self, field: Field) {
		self.required_fields += !field.is_optional as u32;
		self.fields_by_name.insert(field.name.to_string(), field.tag);
		self.fields.insert(field.tag, field);
	}
	pub fn get_field_by_name(&self, name: &str) -> Option<&Field> {
		self.fields.get(self.fields_by_name.get(name)?)
	}
	pub fn get_field_by_id(&self, tag: u32) -> Option<&Field> {
		self.fields.get(&tag)
	}
}

impl TypeId {
	pub fn new(ns: u64, id: u16, metadata: Option<Vec<(String, String)>>) -> Self {
		Self { ns, id, variant: 0, item: None, metadata: metadata.map(Box::new) }
	}
	pub fn with_variant(
		ns: u64, id: u16, variant: u16, item: Option<TypeId>,
		metadata: Option<Vec<(String, String)>>,
	) -> Self {
		Self { ns, id, variant, item: item.map(Box::new), metadata: metadata.map(Box::new) }
	}

	pub const ANY: Self = Self { ns: 0, id: 1, variant: 0, item: None, metadata: None };

	pub fn item(&self) -> &Self {
		self.item.as_ref().unwrap()
	}
	pub fn is_any(&self) -> bool {
		self.ns == 0 && self.id == 1
	}
	pub fn is_builtin(&self) -> bool {
		self.ns == 0
	}
	pub fn arr(item: TypeId, metadata: Option<Vec<(String, String)>>) -> Self {
		Self::with_variant(0, ARR_TYPEID, 0, Some(item), metadata)
	}
	pub fn map(key: u16, value: TypeId, metadata: Option<Vec<(String, String)>>) -> Self {
		Self::with_variant(0, MAP_TYPEID, key, Some(value), metadata)
	}

	pub fn name<'a>(&'a self, provider: &'a dyn DeclProvider) -> TypeIdFormatter<'a> {
		TypeIdFormatter(self, provider)
	}
}
pub struct TypeIdFormatter<'a>(&'a TypeId, &'a dyn DeclProvider);
impl Display for TypeIdFormatter<'_> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let Self(typeid, provider) = *self;
		if typeid.is_builtin() {
			match typeid.id {
				ARR_TYPEID => write!(f, "arr<{}>", typeid.item().name(provider)),
				MAP_TYPEID => {
					let key = BUILT_INS_NAMES[&typeid.variant];
					write!(f, "map<{key}, {}>", typeid.item().name(provider))
				}
				id => write!(f, "{}", BUILT_INS_NAMES[&id]),
			}
		} else {
			let file = provider.get(typeid.ns);
			let item = file.get_by_id(typeid.id).unwrap();
			write!(f, "`{}`.{}", file.name, item.name())
		}
	}
}
impl PartialEq for TypeId {
	fn eq(&self, other: &Self) -> bool {
		if self.is_any() {
			return true;
		}
		self.ns == other.ns
			&& self.id == other.id
			&& self.variant == other.variant
			&& self.item == other.item
	}
}

pub fn resolve_typeid<'a>(typeid: &TypeId, provider: &'a dyn DeclProvider) -> &'a DeclItem {
	provider.get(typeid.ns).get_by_id(typeid.id).unwrap()
}

/// decleration provider with no decleration files.
///
/// ## example
/// ```
/// parse("{ only_builtin_used: true }", &ParseOptions::default(), &VoidProvider{});
/// ```
#[derive(Debug, Default)]
pub struct VoidProvider {}
impl DeclProvider for VoidProvider {
	/// panic.
	fn get(&self, _id: u64) -> &DeclFile {
		panic!("how did we get here")
	}
	/// always return `ImportError::NotFound`
	fn load<'a>(&'a self, _name: &str) -> Result<&'a DeclFile, ImportError> {
		Err(ImportError::NotFound)
	}
}
/// decleration provider with fixed set of decleration files.
///
/// this provider is same as [`FixedSetProvider`], but it take references to the declaration files, not owning them. so it can redirect declarations from other provider.
///
/// ## example
/// ```
/// let provider = FixedSetProviderRef::new(&[
/// 	some_provider.load("file1").unwrap(),
/// 	other_provider.load("file2").unwrap(),
/// ]);
/// provider.load("file2"); // => Some(DeclFile { name: "file2" })
/// provider.load("doesnt exist"); // => ImportError::NotFound
/// ```
#[derive(Debug, Clone)]
pub struct FixedSetProviderRef<'a> {
	files: HashMap<u64, &'a DeclFile>,
	files_by_name: HashMap<&'a str, &'a DeclFile>,
}
impl<'a> FixedSetProviderRef<'a> {
	/// create a new `FixedSetProviderRef` with the passed decleration files.
	pub fn new(declarations: &[&'a DeclFile]) -> Self {
		let mut files = HashMap::new();
		let mut files_by_name = HashMap::new();

		for &file in declarations {
			files.insert(file.id, file);
			files_by_name.insert(file.name.as_str(), file);
		}

		Self { files, files_by_name }
	}
}
impl DeclProvider for FixedSetProviderRef<'_> {
	fn get(&self, id: u64) -> &DeclFile {
		self.files.get(&id).unwrap()
	}
	fn load<'a>(&'a self, name: &str) -> Result<&'a DeclFile, ImportError> {
		self.files_by_name.get(name).copied().ok_or(ImportError::NotFound)
	}
}

/// decleration provider with fixed set of decleration files.
///
/// ## example
/// ```
/// let provider = FixedSetProvider::new(vec![
/// 	parse_declaration_file(/* ... */, "file1", &ParseOptions::default(), &VoidProvider{}).unwrap(),
/// 	parse_declaration_file(/* ... */, "file2", &ParseOptions::default(), &VoidProvider{}).unwrap(),
/// ]);
/// provider.get_by_name("file2"); // => Some(DeclFile { name: "file2" })
/// provider.get_by_name("doesnt exist"); // => None
/// ```
#[derive(Debug)]
pub struct FixedSetProvider {
	files: HashMap<u64, DeclFile>,
	files_by_name: HashMap<String, u64>,
}
impl FixedSetProvider {
	/// create a new `FixedSetProvider` with the passed decleration files.
	pub fn new(declarations: Vec<DeclFile>) -> Self {
		let mut files = HashMap::new();
		let mut files_by_name = HashMap::new();

		for file in declarations.into_iter() {
			files_by_name.insert(file.name.clone(), file.id);
			files.insert(file.id, file);
		}

		Self { files, files_by_name }
	}
}
impl DeclProvider for FixedSetProvider {
	fn get(&self, id: u64) -> &DeclFile {
		self.files.get(&id).unwrap()
	}
	fn load<'a>(&'a self, name: &str) -> Result<&'a DeclFile, ImportError> {
		let Some(file) = self.files_by_name.get(name) else {
			return Err(ImportError::NotFound);
		};
		self.files.get(file).ok_or(ImportError::NotFound)
	}
}
