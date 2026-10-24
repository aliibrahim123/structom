use core::{mem, ops::Deref};

use alloc_crate::sync::Arc;
use lean_string::LeanStr;

use crate::decl::{
	ConstructError, DeclNs,
	id::ItemStore,
	item::{Item, ItemKind},
	items::ItemId,
	rich::Metadata,
};

#[derive(Debug)]
pub struct DeclFile {
	ns: DeclNs,
	pub(crate) path: LeanStr,
	items: ItemStore<Item, { u16::MAX as u32 }, false>,
	metadata: Metadata,
}
impl DeclFile {
	pub fn ns(&self) -> DeclNs {
		self.ns
	}
	pub fn path(&self) -> &str {
		&self.path
	}
	pub fn item_by_name(&self, name: &str) -> Option<&Item> {
		self.items.get_by_name(name)
	}
	pub fn item_by_id(&self, id: ItemId) -> Option<&Item> {
		self.items.get_by_id(id.0 as u32)
	}
	pub fn items(&self) -> &[Item] {
		&self.items.items
	}
	pub fn metadata(&self) -> &Metadata {
		&self.metadata
	}
}

#[derive(Debug)]
pub struct DeclFileBuilder(DeclFile);
impl DeclFileBuilder {
	pub fn new(path: &str) -> Self {
		Self(DeclFile {
			ns: DeclNs::new(),
			path: path.into(),
			items: ItemStore::new(),
			metadata: Metadata::new(),
		})
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
		self.0.ns()
	}
	pub fn path(&self) -> &str {
		&self.0.path
	}
	pub fn item_by_name(&self, name: &str) -> Option<&Item> {
		self.0.item_by_name(name)
	}
	pub fn item_by_id(&self, id: ItemId) -> Option<&Item> {
		self.0.item_by_id(id)
	}
	pub fn items(&self) -> &[Item] {
		self.0.items()
	}
	pub fn metadata(&self) -> &Metadata {
		&self.0.metadata
	}
	pub fn metadata_mut(&mut self) -> &mut Metadata {
		&mut self.0.metadata
	}
	pub fn finalize(self) -> Result<DeclFileBox, ConstructError> {
		if self.0.items.items.iter().any(|item| matches!(item.kind, ItemKind::Reserved)) {
			return Err(ConstructError::UnassignedId);
		}
		Ok(DeclFileBox(Arc::new(self.0)))
	}
}

#[derive(Debug, Clone)]
pub struct DeclFileBox(pub(crate) Arc<DeclFile>);
impl Deref for DeclFileBox {
	type Target = DeclFile;
	fn deref(&self) -> &Self::Target {
		&self.0
	}
}
