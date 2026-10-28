use core::sync::atomic::{AtomicU64, Ordering};

use alloc_crate::vec::Vec;
use hashbrown::hash_map::Entry;
use lean_string::LeanStr;

use crate::{
	decl::{ConstructError, typeid::raw},
	utils::HashMap,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DeclNs(pub(crate) u64);
impl DeclNs {
	pub const BUILT_INS: DeclNs = DeclNs(0);
	pub const GENERIC_PARAMS: DeclNs = DeclNs(1);
	pub(crate) fn new() -> Self {
		static ID_COUNTER: AtomicU64 = AtomicU64::new(2);
		DeclNs(ID_COUNTER.fetch_add(1, Ordering::Relaxed))
	}
	pub fn value(self) -> u64 {
		self.0
	}
}

macro_rules! declare_builtin_itemids {
	[$($name:ident),*] => {
		$(pub const $name: ItemId = ItemId(raw::$name);)*
	};
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ItemId(pub(crate) u16);
impl ItemId {
	pub(crate) const UNASSIGNED: ItemId = ItemId(0xffff);
	declare_builtin_itemids![
		NONE, ANY, BOOL, FAR, U8, U16, U32, U64, I8, I16, I32, I64, F32, F64, F16, STR,
		MAP, LIST, ARR, INST, DUR, BINT, DEC, UUID, COLOR
	];
	pub fn value(self) -> u16 {
		self.0
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FieldId(pub(crate) u16);
impl FieldId {
	pub(crate) const UNASSIGNED: FieldId = FieldId(0xffff);
	pub fn value(self) -> u16 {
		self.0
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VariantTag(pub(crate) u32);
impl VariantTag {
	pub(crate) const UNASSIGNED: VariantTag = VariantTag(0xffffffff);
	pub fn value(self) -> u32 {
		self.0
	}
}

pub(crate) fn check_ident(name: &LeanStr) -> Result<(), ConstructError> {
	let mut chars = name.chars();
	(chars.next().is_some_and(|c| matches!(c, 'a'..='z' | 'A'..='Z' | '_'))
		&& chars.all(|c| matches!(c, 'a'..='z' | 'A'..='Z' | '0'..='9' | '_')))
	.then_some(())
	.ok_or(ConstructError::InvalidName)
}

#[derive(Debug)]
enum IdIndex {
	Sparse(Vec<u32>),
	Map(HashMap<u32, u32>),
}

#[derive(Debug)]
pub(crate) struct ItemStore<T, const MAX_ID: u32, const COUNT_AFTER: bool> {
	pub items: Vec<T>,
	id_index: IdIndex,
	name_index: HashMap<LeanStr, u32>,
	pub id_counter: u32,
}
impl<T, const MAX: u32, const COUNT_AFTER: bool> Default
	for ItemStore<T, MAX, COUNT_AFTER>
{
	fn default() -> Self {
		Self::new()
	}
}
impl<T, const MAX: u32, const COUNT_AFTER: bool> ItemStore<T, MAX, COUNT_AFTER> {
	const UNASSIGNED_INDEX: u32 = u32::MAX;
	const HASHMAP_THRESHOLD: u32 = 64;
	pub fn new() -> Self {
		Self::default()
	}
	pub fn has_index(&self, id: u32) -> bool {
		match &self.id_index {
			IdIndex::Sparse(map) => {
				map.get(id as usize).is_some_and(|i| *i != Self::UNASSIGNED_INDEX)
			}
			IdIndex::Map(map) => map.contains_key(&id),
		}
	}
	pub fn get_index(&self, id: u32) -> Option<u32> {
		match &self.id_index {
			IdIndex::Sparse(map) => map.get(id as usize).copied(),
			IdIndex::Map(map) => map.get(&id).copied(),
		}
	}
	pub fn add_index(&mut self, id: u32, index: u32) {
		match &mut self.id_index {
			IdIndex::Sparse(map) => {
				if id > Self::HASHMAP_THRESHOLD {
					let map = map.iter().enumerate().filter_map(|(id, ind)| {
						(*ind != Self::UNASSIGNED_INDEX).then_some((id as u32, *ind))
					});
					let mut map = HashMap::from_iter(map);
					map.insert(id, index);
					self.id_index = IdIndex::Map(map);
				} else {
					if id as usize >= map.len() {
						map.resize(id as usize + 1, Self::UNASSIGNED_INDEX);
					}
					map[id as usize] = index;
				}
			}
			IdIndex::Map(map) => {
				map.insert(id, index);
			}
		}
	}
	pub fn add(
		&mut self, name: LeanStr, item: T,
	) -> Result<(&mut T, u32), ConstructError> {
		while self.has_index(self.id_counter) {
			if self.id_counter == MAX {
				return Err(ConstructError::FullCapacity);
			}
			self.id_counter += 1;
		}

		self.insert(name, item, self.id_counter)
	}
	pub fn add_at(
		&mut self, name: LeanStr, item: T, id: usize,
	) -> Result<(&mut T, u32), ConstructError> {
		if id > MAX as usize {
			return Err(ConstructError::IdOutOfRange);
		}
		if self.has_index(id as u32) {
			return Err(ConstructError::UsedId);
		}
		self.insert(name, item, id as u32)
	}
	fn insert(
		&mut self, name: LeanStr, item: T, id: u32,
	) -> Result<(&mut T, u32), ConstructError> {
		check_ident(&name)?;
		let Entry::Vacant(entry) = self.name_index.entry(name) else {
			return Err(ConstructError::DuplicateName);
		};
		if COUNT_AFTER {
			self.id_counter = id
		}
		entry.insert(self.items.len() as u32);
		self.add_index(id, self.items.len() as u32);
		Ok((self.items.push_mut(item), id))
	}
	pub fn get_by_id(&self, tag: u32) -> Option<&T> {
		self.get_index(tag).and_then(|i| self.items.get(i as usize))
	}
	pub fn get_by_name(&self, name: &str) -> Option<&T> {
		self.name_index.get(name).and_then(|i| self.items.get(*i as usize))
	}
}
