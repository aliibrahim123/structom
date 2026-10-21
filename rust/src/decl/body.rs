use lean_string::LeanStr;

use crate::{
	decl::{
		ConstructError,
		id::ItemStore,
		items::{FieldId, TypeId},
		rich::Metadata,
	},
	value::Value,
};

#[derive(Debug)]
enum FieldType {
	Required,
	Optional { default_value: Option<Value> },
}

#[derive(Debug)]
pub struct Field {
	id: FieldId,
	name: LeanStr,
	typeid: TypeId,
	type_: FieldType,
	metadata: Metadata,
}
impl Field {
	pub fn new(name: &str, typeid: TypeId) -> Self {
		Self::new_lean(LeanStr::from(name), typeid)
	}
	pub(crate) fn new_lean(name: LeanStr, typeid: TypeId) -> Self {
		Self {
			id: FieldId::UNASSIGNED,
			name,
			typeid,
			type_: FieldType::Required,
			metadata: Metadata::new(),
		}
	}
	pub fn set_optional(&mut self, default_value: Option<Value>) {
		self.type_ = FieldType::Optional { default_value };
	}
	pub fn name(&self) -> &str {
		&self.name
	}
	pub fn typeid(&self) -> &TypeId {
		&self.typeid
	}
	pub fn id(&self) -> FieldId {
		self.id
	}
	pub fn is_optional(&self) -> bool {
		!matches!(self.type_, FieldType::Required)
	}
	pub fn default_value(&self) -> Option<&Value> {
		match &self.type_ {
			FieldType::Required => None,
			FieldType::Optional { default_value } => default_value.as_ref(),
		}
	}
	pub fn metadata(&self) -> &Metadata {
		&self.metadata
	}
	pub fn metadata_mut(&mut self) -> &mut Metadata {
		&mut self.metadata
	}
}

#[derive(Debug)]
pub struct Fields {
	is_fixed: bool,
	tree_children: Option<FieldId>,
	fields: ItemStore<Field, { u16::MAX as u32 }, false>,
}
impl Default for Fields {
	fn default() -> Self {
		Self::new(false)
	}
}
impl Fields {
	pub fn new(is_fixed: bool) -> Self {
		Self { is_fixed, fields: ItemStore::new(), tree_children: None }
	}
	pub fn add_field(&mut self, field: Field) -> Result<FieldId, ConstructError> {
		self.fields.add(field.name.clone(), field).map(|(field, id)| {
			field.id = FieldId(id as u16);
			field.id
		})
	}
	pub fn add_field_at(
		&mut self, field: Field, id: usize,
	) -> Result<(), ConstructError> {
		self.fields.add_at(field.name.clone(), field, id).map(|(field, id)| {
			field.id = FieldId(id as u16);
		})
	}
	pub fn as_tree(&mut self, children: FieldId) -> Result<(), ConstructError> {
		if !self.fields.has_index(children.0 as u32) {
			return Err(ConstructError::UnassignedId);
		}
		self.tree_children = Some(children);
		Ok(())
	}
	pub fn is_fixed(&self) -> bool {
		self.is_fixed
	}
	pub fn field_by_id(&self, id: FieldId) -> Option<&Field> {
		self.fields.get_by_id(id.0 as u32)
	}
	pub fn field_by_name(&self, name: &str) -> Option<&Field> {
		self.fields.get_by_name(name)
	}
	pub fn iter(&self) -> impl Iterator<Item = &Field> {
		self.fields.items.iter()
	}
	pub fn finalize(&mut self) -> Result<(), ConstructError> {
		let max_id = self.iter().map(|field| field.id.0 as i32).max().unwrap_or(-1);
		match self.fields.items.len() as i32 == max_id + 1 {
			true => Ok(()),
			false => Err(ConstructError::UnassignedSpace),
		}
	}
}

#[derive(Debug)]
pub enum Body {
	Fields(Fields),
	Tagged(TypeId),
}

impl From<Fields> for Body {
	fn from(fields: Fields) -> Self {
		Self::Fields(fields)
	}
}
