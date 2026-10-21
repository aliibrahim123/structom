use core::mem;

use alloc_crate::sync::Arc;
use lean_string::LeanStr;

use crate::{
	decl::{
		ConstructError, DeclNs,
		id::ItemStore,
		items::{Body, ItemId, Union},
		rich::{Metadata, RichNotation},
	},
	value::Value,
};

#[derive(Debug)]
pub enum ItemKind {
	Struct(Body),
	Union(Union),
	#[doc(hidden)]
	Reserved,
}

#[derive(Debug)]
pub struct Item {
	name: LeanStr,
	id: ItemId,
	kind: ItemKind,
	default_value: Option<Value>,
	rich_notation: RichNotation,
	metadata: Metadata,
}
impl Item {
	pub fn struct_(name: &str, body: impl Into<Body>) -> Result<Self, ConstructError> {
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

#[derive(Debug)]
struct DeclFileInner {
	ns: DeclNs,
	items: ItemStore<Item, { u16::MAX as u32 }, false>,
}

#[derive(Debug)]
pub struct DeclFileBuilder(DeclFileInner);
impl Default for DeclFileBuilder {
	fn default() -> Self {
		Self(DeclFileInner { ns: DeclNs::new(), items: ItemStore::new() })
	}
}
impl DeclFileBuilder {
	pub fn new() -> Self {
		Self::default()
	}
	pub fn add_item(&mut self, item: Item) -> Result<ItemId, ConstructError> {
		self.0.items.add(item.name.clone(), item).map(|(item, id)| {
			item.id = ItemId(id as u16);
			item.id
		})
	}
	pub fn add_item_at(&mut self, item: Item, id: usize) -> Result<(), ConstructError> {
		self.0.items.add_at(item.name.clone(), item, id as usize).map(|(item, id)| {
			item.id = ItemId(id as u16);
		})
	}
	pub fn reserve(
		&mut self, name: &str, id: Option<usize>,
	) -> Result<ItemId, ConstructError> {
		match id {
			Some(id) => self
				.add_item_at(Item::new_lean(LeanStr::from(name), ItemKind::Reserved), id)
				.map(|_| ItemId(id as u16)),
			None => {
				self.add_item(Item::new_lean(LeanStr::from(name), ItemKind::Reserved))
			}
		}
	}
	pub fn define_reserved(
		&mut self, id: ItemId, mut item: Item,
	) -> Result<(), ConstructError> {
		let Some(index) = self.0.items.get_index(id.0 as u32) else {
			return Err(ConstructError::UnassignedId);
		};
		let slot = self.0.items.items.get_mut(index as usize).unwrap();
		if matches!(slot.kind, ItemKind::Reserved) {
			return Err(ConstructError::UsedId);
		}
		item.id = id;
		mem::swap(&mut item.name, &mut slot.name);
		*slot = item;
		Ok(())
	}
	pub fn ns(&self) -> DeclNs {
		self.0.ns
	}
	pub fn item_by_name(&self, name: &str) -> Option<&Item> {
		self.0.items.get_by_name(name)
	}
	pub fn item_by_id(&self, id: ItemId) -> Option<&Item> {
		self.0.items.get_by_id(id.0 as u32)
	}
	pub fn items(&self) -> impl Iterator<Item = &Item> {
		self.0.items.items.iter()
	}

	pub fn finalize(self) -> DeclFile {
		DeclFile(Arc::new(self.0))
	}
}

#[derive(Debug, Clone)]
pub struct DeclFile(Arc<DeclFileInner>);
impl DeclFile {
	pub fn ns(&self) -> DeclNs {
		self.0.ns
	}
	pub fn item_by_name(&self, name: &str) -> Option<&Item> {
		self.0.items.get_by_name(name)
	}
	pub fn item_by_id(&self, id: ItemId) -> Option<&Item> {
		self.0.items.get_by_id(id.0 as u32)
	}
	pub fn items(&self) -> impl Iterator<Item = &Item> {
		self.0.items.items.iter()
	}
}
