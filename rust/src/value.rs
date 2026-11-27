use core::{
	hash::Hash,
	ops::{Deref, DerefMut},
};

use alloc_crate::{boxed::Box, vec::Vec};
use compact_str::CompactString;
use indexmap::IndexMap;

use crate::{
	decl::items::FieldId,
	utils::{HashMap, Hasher},
};

#[derive(Debug, Clone, PartialEq, Default)]
pub enum Value {
	#[default]
	None,
	Str(CompactString),
	Map(Map),
	List(Vec<Value>),
	Struct(Struct),
}
impl Value {
	pub fn into_map(self) -> Option<Map> {
		match self {
			Value::Map(map) => Some(map),
			_ => None,
		}
	}
	pub fn as_list(&self) -> Option<&[Value]> {
		match self {
			Value::List(list) => Some(list),
			_ => None,
		}
	}
	pub fn as_str(&self) -> Option<&str> {
		match self {
			Value::Str(str) => Some(str),
			_ => None,
		}
	}
	pub fn as_struct_mut(&mut self) -> Option<&mut Struct> {
		match self {
			Value::Struct(struct_) => Some(struct_),
			_ => None,
		}
	}
}

impl Eq for Value {}
impl Hash for Value {
	fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
		todo!()
	}
}
impl PartialOrd for Value {
	fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
		todo!()
	}
}

#[derive(Debug, Clone, PartialEq, Default, Eq, Hash, PartialOrd)]
pub struct Key(Value);
impl Deref for Key {
	type Target = Value;
	fn deref(&self) -> &Self::Target {
		&self.0
	}
}
impl DerefMut for Key {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.0
	}
}

#[derive(Debug, Clone, PartialEq, Default, Eq, Hash, PartialOrd)]
pub struct Struct(Vec<Value>);
impl Struct {
	pub fn get_field(&self, id: FieldId) -> Option<&Value> {
		self.0.get(id.value() as usize)
	}

	pub fn take_field(&mut self, id: FieldId) -> Option<Value> {
		self.0.get_mut(id.value() as usize).map(core::mem::take)
	}
}

#[derive(Debug, Clone, PartialEq)]
pub struct Map(Box<MapRep>);
#[derive(Debug, Clone, PartialEq)]
enum MapRep {
	Small(Vec<(Key, Value)>),
	Large(IndexMap<Key, Value, Hasher>),
}
impl Map {
	pub fn into_iter(self) -> impl Iterator<Item = (Key, Value)> {
		enum Iter {
			Small(alloc_crate::vec::IntoIter<(Key, Value)>),
			Large(indexmap::map::IntoIter<Key, Value>),
		}
		impl Iterator for Iter {
			type Item = (Key, Value);
			fn next(&mut self) -> Option<Self::Item> {
				match self {
					Iter::Small(iter) => iter.next(),
					Iter::Large(iter) => iter.next(),
				}
			}
		}
		match *self.0 {
			MapRep::Small(map) => Iter::Small(map.into_iter()),
			MapRep::Large(map) => Iter::Large(map.into_iter()),
		}
	}
}
