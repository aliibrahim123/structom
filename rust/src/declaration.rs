use std::collections::HashMap;

#[derive(Debug)]
pub struct DeclarationFile {
	pub name: String,
	pub id: u64,
	pub items: Vec<Option<DeclarationItem>>,
	pub items_by_name: HashMap<String, u16>,
}

#[derive(PartialEq, Hash, Debug)]
pub struct TypeId {
	pub ns: u64,
	pub id: u16,
	pub variant: Option<u16>,
}

#[derive(Debug)]
struct Field {
	pub name: String,
	pub tag: u32,
	pub item: TypeId,
}
#[derive(Default, Debug)]
struct StructDef {
	fields: Vec<Option<Field>>,
	fields_by_name: HashMap<String, u32>,
}

#[derive(Debug)]
struct EnumVariant {
	name: String,
	tag: u32,
	def: Option<StructDef>,
}

#[derive(Debug)]
pub enum DeclarationItem {
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

static mut DECLARE_ID_COUNTER: u64 = 0;
impl DeclarationFile {
	pub fn new(name: String) -> Self {
		let id = unsafe {
			DECLARE_ID_COUNTER += 1;
			DECLARE_ID_COUNTER
		};
		DeclarationFile {
			name,
			id,
			items: vec![],
			items_by_name: HashMap::new(),
		}
	}

	pub fn add_item(&mut self, item: DeclarationItem) -> Result<(), ()> {
		self.items_by_name
			.insert(item.name().to_string(), item.typeid());
		add_item(&mut self.items, item.typeid() as usize, item)
	}

	pub fn get_by_name(&self, name: &str) -> Option<&DeclarationItem> {
		let id = self.items_by_name.get(name);
		id.and_then(|id| self.get_by_id(*id))
	}
	pub fn get_by_id(&self, id: u16) -> Option<&DeclarationItem> {
		self.items.get(id as usize).and_then(|item| item.as_ref())
	}
}

impl DeclarationItem {
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

	pub fn add_variant(&mut self, variant: EnumVariant) -> Result<(), ()> {
		match self {
			Self::Enum {
				variants,
				variants_by_name,
				..
			} => {
				variants_by_name.insert(variant.name.to_string(), variant.tag);
				add_item(variants, variant.tag as usize, variant)
			}
			_ => Err(()),
		}
	}
	pub fn get_variant_by_name(&self, name: &str) -> Option<&EnumVariant> {
		match self {
			Self::Enum {
				variants_by_name, ..
			} => variants_by_name
				.get(name)
				.and_then(|v| self.get_variant_by_id(*v)),
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
		self.fields_by_name
			.insert(field.name.to_string(), field.tag);
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
