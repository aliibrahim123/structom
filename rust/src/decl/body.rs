use lean_string::LeanStr;

use crate::{
	decl::{
		AssignError,
		id::ItemStore,
		items::{FieldId, TypeId},
		rich::Metadata,
	},
	value::Value,
};

#[derive(Debug, Clone)]
pub(crate) enum FieldType {
	Required,
	Optional { default_value: Option<Value> },
	Reserved,
}

#[derive(Debug, Clone)]
pub struct Field {
	id: FieldId,
	name: LeanStr,
	pub(crate) typeid: TypeId,
	pub(crate) type_: FieldType,
	pub(crate) metadata: Metadata,
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
	pub fn is_reserved(&self) -> bool {
		matches!(self.type_, FieldType::Reserved)
	}
	pub fn is_optional(&self) -> bool {
		matches!(self.type_, FieldType::Optional { .. })
	}
	pub fn default_value(&self) -> Option<&Value> {
		match &self.type_ {
			FieldType::Required | FieldType::Reserved => None,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnassignedSpace;

#[derive(Debug, Clone)]
pub struct Fields {
	is_fixed: bool,
	tree_children: Option<FieldId>,
	pub(crate) fields: ItemStore<Field, { u16::MAX as u32 }, false>,
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
	pub fn add_field(&mut self, field: Field) -> Result<FieldId, (Field, AssignError)> {
		self.fields.add(field.name.clone(), field).map(|(field, id)| {
			field.id = FieldId(id as u16);
			field.id
		})
	}
	pub fn add_field_at(
		&mut self, field: Field, id: u16,
	) -> Result<FieldId, (Field, AssignError)> {
		self.fields.add_at(field.name.clone(), field, id as _).map(|(field, id)| {
			field.id = FieldId(id as u16);
			field.id
		})
	}
	pub fn reserve(
		&mut self, typeid: TypeId, id: Option<u16>,
	) -> Result<FieldId, AssignError> {
		let mut field = Field::new("_", typeid);
		field.type_ = FieldType::Reserved;
		match id {
			Some(id) => self.add_field_at(field, id).map_err(|(_, err)| err),
			None => self.add_field(field).map_err(|(_, err)| err),
		}
	}
	pub fn as_tree(&mut self, children: FieldId) -> Result<(), AssignError> {
		if !self.fields.has_index(children.0 as u32) {
			return Err(AssignError::UnassignedId);
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
	pub fn count(&self) -> usize {
		self.fields.items.len()
	}
	pub fn as_slice(&self) -> &[Field] {
		&self.fields.items
	}
	pub fn finalize(&mut self) -> Result<(), UnassignedSpace> {
		let max_id =
			self.fields.items.iter().map(|field| field.id.0 as i32).max().unwrap_or(-1);
		match self.fields.items.len() as i32 == max_id + 1 {
			true => Ok(()),
			false => Err(UnassignedSpace),
		}
	}
}

#[derive(Debug, Clone)]
pub enum Body {
	Fields(Fields),
	Tagged(TypeId),
}

impl From<Fields> for Body {
	fn from(fields: Fields) -> Self {
		Self::Fields(fields)
	}
}
impl Body {
	pub fn as_fields(&self) -> Option<&Fields> {
		match self {
			Self::Fields(fields) => Some(fields),
			_ => None,
		}
	}
	pub fn as_tagged(&self) -> Option<&TypeId> {
		match self {
			Self::Tagged(typeid) => Some(typeid),
			_ => None,
		}
	}
}
