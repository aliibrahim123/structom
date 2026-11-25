use core::ops::Deref;

use crate::{
	decl::{
		body::{Body, Field, FieldType, Fields},
		id::{FieldId, VariantTag},
		item::{Item, ItemKind},
		items::{TypeId, Union, Variant},
		rich::{Metadata, RichNotation},
	},
	value::Value,
};

#[derive(Debug)]
pub struct SoftMut<'a, T>(pub(crate) &'a mut T);
impl<'src, T> Deref for SoftMut<'src, T> {
	type Target = T;
	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl SoftMut<'_, Item> {
	pub fn set_default_value(&mut self, value: Value) {
		self.0.default_value = Some(value);
	}
	pub fn default_value_mut(&mut self) -> &mut Option<Value> {
		&mut self.0.default_value
	}
	pub fn as_struct<'a>(&'a mut self) -> Option<SoftMut<'a, Body>> {
		match &mut self.0.kind {
			ItemKind::Struct(body) => Some(SoftMut(body)),
			_ => None,
		}
	}
	pub fn as_union<'a>(&'a mut self) -> Option<SoftMut<'a, Union>> {
		match &mut self.0.kind {
			ItemKind::Union(union) => Some(SoftMut(union)),
			_ => None,
		}
	}
	pub fn rich_notation_mut(&mut self) -> &mut RichNotation {
		&mut self.0.rich_notation
	}
	pub fn metadata_mut(&mut self) -> &mut Metadata {
		&mut self.0.metadata
	}
}

impl SoftMut<'_, Union> {
	pub fn variant_mut<'a>(
		&'a mut self, tag: VariantTag,
	) -> Option<SoftMut<'a, Variant>> {
		self.0.variants.get_by_id_mut(tag.0).map(SoftMut)
	}
}

impl SoftMut<'_, Variant> {
	pub fn body<'a>(&'a mut self) -> Option<SoftMut<'a, Body>> {
		self.0.body.as_mut().map(SoftMut)
	}
	pub fn rich_notation_mut(&mut self) -> &mut RichNotation {
		&mut self.0.rich_notation
	}
	pub fn metadata_mut(&mut self) -> &mut Metadata {
		&mut self.0.metadata
	}
}

impl SoftMut<'_, Body> {
	pub fn as_fields<'a>(&'a mut self) -> Option<SoftMut<'a, Fields>> {
		match &mut self.0 {
			Body::Fields(fields) => Some(SoftMut(fields)),
			_ => None,
		}
	}
	pub fn as_tagged<'a>(&'a mut self) -> Option<SoftMut<'a, TypeId>> {
		match &mut self.0 {
			Body::Tagged(fields) => Some(SoftMut(fields)),
			_ => None,
		}
	}
}

impl SoftMut<'_, Fields> {
	pub fn field_mut<'a>(&'a mut self, id: FieldId) -> Option<SoftMut<'a, Field>> {
		self.0.fields.get_by_id_mut(id.0 as u32).map(SoftMut)
	}
}

impl SoftMut<'_, Field> {
	pub fn typeid_mut<'a>(&'a mut self) -> SoftMut<'a, TypeId> {
		SoftMut(&mut self.0.typeid)
	}
	pub fn set_default_value(&mut self, value: Value) {
		if matches!(self.0.type_, FieldType::Reserved) {
			return;
		}
		self.0.type_ = FieldType::Optional { default_value: Some(value) };
	}
	pub fn default_value_mut(&mut self) -> Option<&mut Value> {
		match &mut self.0.type_ {
			FieldType::Optional { default_value } => default_value.as_mut(),
			_ => None,
		}
	}
	pub fn metadata_mut(&mut self) -> &mut Metadata {
		&mut self.0.metadata
	}
}

impl SoftMut<'_, TypeId> {
	pub fn param_mut<'a>(&'a mut self, ind: usize) -> Option<SoftMut<'a, TypeId>> {
		self.0.params.get_mut(ind).map(SoftMut)
	}
	pub fn metadata_mut(&mut self) -> &mut Metadata {
		&mut self.0.metadata
	}
}
