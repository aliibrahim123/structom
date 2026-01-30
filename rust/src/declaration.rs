use std::collections::HashMap;

use crate::{builtins::BUILT_INS_NAMES, errors::ImportError};

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
	pub ns: u64,
	pub id: u16,
	pub variant: u16,
	pub item: Option<Box<TypeId>>,
	pub metadata: Option<Vec<(String, String)>>,
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
	pub fields: Vec<Option<Field>>,
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
		variants: Vec<Option<EnumVariant>>,
		variants_by_name: HashMap<String, u32>,
	},
}

pub type LoadResult<'a> = Result<&'a DeclFile, ImportError>;

/// trait for types providing decleration files.
///
/// decleration providers are used by functions that need access to decleration files.
///
/// decleration providers can provide decleration files from any source, guaranteed to be valid and the same for every access.
pub trait DeclProvider {
	/// get a decleration file by its id.
	///
	/// this method can not fail, it is used for decleration files that were created before.
	fn get<'a>(&'a self, id: u64) -> &'a DeclFile;

	/// get a decleration file by its name.
	///   
	/// this method return `None` on fail, when the requested decleration file can not be found or it cant be parsed.
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
		let id = self.items_by_name.get(name);
		id.and_then(|id| self.get_by_id(*id))
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
		Self::Enum { name, typeid, variants: vec![], variants_by_name: HashMap::new() }
	}

	pub fn add_variant(&mut self, variant: EnumVariant) -> Result<(), ()> {
		match self {
			Self::Enum { variants, variants_by_name, .. } => {
				variants_by_name.insert(variant.name.to_string(), variant.tag);
				add_item(variants, variant.tag as usize, variant)
			}
			_ => Err(()),
		}
	}
	pub fn get_variant_by_name(&self, name: &str) -> Option<&EnumVariant> {
		match self {
			Self::Enum { variants_by_name, .. } => {
				variants_by_name.get(name).and_then(|v| self.get_variant_by_id(*v))
			}
			_ => None,
		}
	}
	pub fn get_variant_by_id(&self, tag: u32) -> Option<&EnumVariant> {
		match self {
			Self::Enum { variants, .. } => variants.get(tag as usize).and_then(|v| v.as_ref()),
			_ => None,
		}
	}
}

impl StructDef {
	pub fn add_field(&mut self, field: Field) -> Result<(), ()> {
		self.required_fields += !field.is_optional as u32;
		self.fields_by_name.insert(field.name.to_string(), field.tag);
		add_item(&mut self.fields, field.tag as usize, field)
	}
	pub fn get_field_by_name(&self, name: &str) -> Option<&Field> {
		let id = self.fields_by_name.get(name);
		id.and_then(|v| self.get_field_by_id(*v))
	}
	pub fn get_field_by_id(&self, tag: u32) -> Option<&Field> {
		self.fields.get(tag as usize).and_then(|v| v.as_ref())
	}
}

impl Field {
	pub fn new(name: String, tag: u32, typeid: TypeId, is_optional: bool) -> Self {
		Self { name, tag, typeid, is_optional }
	}
}

impl TypeId {
	pub fn new(ns: u64, id: u16, metadata: Option<Vec<(String, String)>>) -> Self {
		Self { ns, id, variant: 0, item: None, metadata }
	}
	pub fn with_variant(
		ns: u64, id: u16, variant: u16, sub_type: Option<TypeId>,
		metadata: Option<Vec<(String, String)>>,
	) -> Self {
		Self { ns, id, variant, item: sub_type.map(|t| Box::new(t)), metadata }
	}

	pub const ANY: Self = Self { ns: 0, id: 1, variant: 0, item: None, metadata: None };

	pub fn is_any(&self) -> bool {
		self.ns == 0 && self.id == 1
	}

	pub fn name(&self, provider: &dyn DeclProvider) -> String {
		if self.ns == 0 {
			// arr
			if self.id == 0x22 {
				return format!("arr<{}>", self.item.as_ref().unwrap().name(provider));
			}
			// map
			if self.id == 0x23 {
				return format!(
					"map<{}, {}>",
					BUILT_INS_NAMES[&self.variant],
					self.item.as_ref().unwrap().name(provider)
				);
			}
			// other builtin
			BUILT_INS_NAMES[&self.id].to_string()
		// user defined
		} else {
			let file = provider.get(self.ns);

			format!("`{}`.{}", file.name, file.get_by_id(self.id).unwrap().name())
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

fn add_item<'a, T>(vec: &mut Vec<Option<T>>, id: usize, item: T) -> Result<(), ()> {
	if id < vec.len() {
		return Err(());
	};
	for _ in vec.len()..id {
		vec.push(None)
	}
	vec.push(Some(item));
	Ok(())
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
	/// always return `None`
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
/// 	some_provider.get_by_name("file1").unwrap(),
/// 	other_provider.get_by_name("file2").unwrap(),
/// ]);
/// provider.get_by_name("file2"); // => Some(DeclFile { name: "file2" })
/// provider.get_by_name("doesnt exist"); // => None
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

		// add files
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
		self.files_by_name.get(name).map(|f| *f).ok_or(ImportError::NotFound)
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

		// add files
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
		let file = self.files_by_name.get(name).map(|ind| self.files.get(ind).unwrap());
		file.ok_or(ImportError::NotFound)
	}
}
