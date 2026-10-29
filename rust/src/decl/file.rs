use core::{mem, ops::Deref};

use alloc_crate::{sync::Arc, vec::Vec};
use lean_string::LeanStr;
use thin_vec::ThinVec;

use crate::decl::{
	ConstructError, DeclNs,
	id::ItemStore,
	item::{Item, ItemKind},
	items::{ItemId, TypeId},
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
	pub fn item_typeid(
		&self, id: ItemId, params: impl IntoIterator<Item = TypeId>,
	) -> Result<TypeId, ConstructError> {
		let item = self.item_by_id(id).ok_or(ConstructError::UnassignedId)?;
		let params = ThinVec::from_iter(params);
		if params.len() != item.generic_params().len() {
			return Err(ConstructError::MismatchParams);
		}
		let mut typeid = TypeId::new_unchecked(self.ns, id);
		typeid.params = params;
		Ok(typeid)
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
	fn new_with_id(ns: DeclNs, path: &str) -> Self {
		Self(DeclFile {
			ns,
			path: path.into(),
			items: ItemStore::new(),
			metadata: Metadata::new(),
		})
	}
	pub fn new(path: &str) -> Self {
		Self::new_with_id(DeclNs::new(), path)
	}
	pub fn new_batch<'path>(paths: impl IntoIterator<Item = &'path str>) -> Vec<Self> {
		let paths = Vec::from_iter(paths);
		let start = DeclNs::new_n(paths.len() as u64);
		let paths = paths.into_iter().enumerate().map(|(ind, path)| {
			return Self::new_with_id(DeclNs(start + ind as u64), path);
		});
		paths.collect()
	}
	pub fn add_item(&mut self, item: Item) -> Result<ItemId, ConstructError> {
		self.0.items.add(item.name.clone(), item).map(|(item, id)| {
			item.id = ItemId(id as u16);
			item.id
		})
	}
	pub fn add_item_at(
		&mut self, item: Item, id: usize,
	) -> Result<ItemId, ConstructError> {
		self.0.items.add_at(item.name.clone(), item, id as usize).map(|(item, id)| {
			item.id = ItemId(id as u16);
			item.id
		})
	}
	pub fn reserve<'params>(
		&mut self, name: Option<&str>, id: Option<usize>,
		generic_params: impl IntoIterator<Item = &'params str>,
	) -> Result<ItemId, ConstructError> {
		let mut item =
			Item::new_lean(LeanStr::from(name.unwrap_or("_")), ItemKind::Reserved);
		for param in generic_params {
			item.generic_params_mut().add(param)?;
		}
		match id {
			Some(id) => self.add_item_at(item, id),
			None => self.add_item(item),
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
	pub fn metadata_mut(&mut self) -> &mut Metadata {
		&mut self.0.metadata
	}
	pub fn finalize(self) -> Result<DeclFileBox, ConstructError> {
		if self.0.items.items.iter().any(|item| {
			return matches!(item.kind, ItemKind::Reserved) && item.name != "_";
		}) {
			return Err(ConstructError::UnassignedId);
		}
		Ok(DeclFileBox(Arc::new(self.0)))
	}
}
impl Deref for DeclFileBuilder {
	type Target = DeclFile;
	fn deref(&self) -> &Self::Target {
		&self.0
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
