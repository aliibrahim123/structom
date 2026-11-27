mod body;
mod file;
mod id;
mod item;
pub(crate) mod private;
pub mod provider;
pub mod rich;
mod soft_mut;
mod typeid;
mod union;

pub use crate::decl::{
	file::{DeclFile, DeclFileBox},
	id::DeclNs,
	provider::DeclProv,
};
use crate::decl::{
	item::Item,
	items::{TypeId, TypeIdError},
};

pub mod items {
	pub use super::body::*;
	pub use super::file::DeclFileBuilder;
	pub use super::id::{FieldId, ItemId, VariantTag};
	pub use super::item::*;
	pub use super::soft_mut::*;
	pub use super::typeid::*;
	pub use super::union::*;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AssignError {
	UsedId,
	UnassignedId,
	FullCapacity,
	IdOutOfRange,
	DuplicateName,
	InvalidName,
	UnresolvedType,
	InvalidType,
}

pub fn resolve_item<'prov>(
	typeid: &TypeId, prov: &'prov dyn DeclProv,
) -> Result<&'prov Item, TypeIdError> {
	prov.get_file(typeid.ns())
		.and_then(|file| file.item_by_id(typeid.item()))
		.ok_or(TypeIdError::UnknownItem)
}
