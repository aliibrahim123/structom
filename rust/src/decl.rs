mod body;
mod file;
mod id;
mod item;
pub mod provider;
pub mod rich;
mod typeid;
mod union;

pub use crate::decl::{
	file::{DeclFile, DeclFileBox},
	id::DeclNs,
	provider::DeclProv,
};

pub mod items {
	pub use super::body::*;
	pub use super::file::DeclFileBuilder;
	pub use super::id::{FieldId, ItemId, VariantTag};
	pub use super::item::*;
	pub use super::typeid::*;
	pub use super::union::*;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConstructError {
	UsedId,
	AddedBefore,
	FullCapacity,
	IdOutOfRange,
	UnassignedSpace,
	UnassignedId,
	DuplicateName,
	InvalidName,
	DuplicateNotation,
	MismatchParams,
	NonContiguous,
}
