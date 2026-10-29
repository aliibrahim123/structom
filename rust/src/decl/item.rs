use lean_string::LeanStr;
use thin_vec::ThinVec;

use crate::{
	decl::{
		AssignError, DeclNs,
		body::{Body, UnassignedSpace},
		id::ItemId,
		items::TypeId,
		rich::{Metadata, RichNotation},
		union::Union,
	},
	value::Value,
};

#[derive(Debug, Clone)]
pub struct GenericParam {
	name: LeanStr,
	id: ItemId,
}
impl GenericParam {
	pub fn name(&self) -> &str {
		self.name.as_str()
	}
	pub fn id(&self) -> ItemId {
		self.id
	}
	pub fn as_typeid(&self) -> TypeId {
		TypeId::new_unchecked(DeclNs::GENERIC_PARAMS, self.id)
	}
}

#[derive(Debug, Clone)]
pub struct GenericParams(ThinVec<GenericParam>);
impl GenericParams {
	pub(crate) fn new() -> Self {
		Self(ThinVec::new())
	}
	pub fn len(&self) -> usize {
		self.0.len()
	}
	pub fn add(&mut self, name: &str) -> Result<ItemId, AssignError> {
		if self.0.iter().any(|param| param.name == name) {
			return Err(AssignError::DuplicateName);
		}
		let id = ItemId(self.0.len() as u16);
		self.0.push(GenericParam { name: LeanStr::from(name), id });
		Ok(id)
	}
	pub fn get_by_id(&self, id: ItemId) -> Option<&GenericParam> {
		self.0.get(id.0 as usize)
	}
	pub fn get_by_name(&self, name: &str) -> Option<&GenericParam> {
		self.0.iter().find(|param| param.name == name)
	}
	pub fn as_slice(&self) -> &[GenericParam] {
		&self.0
	}
}

#[derive(Debug)]
pub enum ItemKind {
	Struct(Body),
	Union(Union),
	#[doc(hidden)]
	Reserved,
}

#[derive(Debug)]
pub struct Item {
	pub(crate) name: LeanStr,
	pub(crate) id: ItemId,
	pub(crate) kind: ItemKind,
	default_value: Option<Value>,
	generic_params: GenericParams,
	rich_notation: RichNotation,
	metadata: Metadata,
}
impl Item {
	pub fn struct_(name: &str, body: impl Into<Body>) -> Result<Self, UnassignedSpace> {
		let mut body = body.into();
		if let Body::Fields(fields) = &mut body {
			fields.finalize()?;
		}
		Ok(Self::new_lean(LeanStr::from(name), ItemKind::Struct(body)))
	}
	pub fn union(name: &str, union: Union) -> Self {
		Self::new_lean(LeanStr::from(name), ItemKind::Union(union))
	}
	pub(crate) fn new_lean(name: LeanStr, kind: ItemKind) -> Self {
		Self {
			name,
			id: ItemId::UNASSIGNED,
			kind,
			default_value: None,
			rich_notation: RichNotation::new(),
			metadata: Metadata::new(),
			generic_params: GenericParams::new(),
		}
	}
	pub fn set_default_value(&mut self, default_value: Value) {
		self.default_value = Some(default_value);
	}
	pub fn name(&self) -> &str {
		&self.name
	}
	pub fn id(&self) -> ItemId {
		self.id
	}
	pub fn kind(&self) -> &ItemKind {
		&self.kind
	}
	pub fn default_value(&self) -> Option<&Value> {
		self.default_value.as_ref()
	}
	pub fn generic_params(&self) -> &GenericParams {
		&self.generic_params
	}
	pub fn generic_params_mut(&mut self) -> &mut GenericParams {
		&mut self.generic_params
	}
	pub fn rich_notation(&self) -> &RichNotation {
		&self.rich_notation
	}
	pub fn rich_notation_mut(&mut self) -> &mut RichNotation {
		&mut self.rich_notation
	}
	pub fn metadata(&self) -> &Metadata {
		&self.metadata
	}
	pub fn metadata_mut(&mut self) -> &mut Metadata {
		&mut self.metadata
	}
}
