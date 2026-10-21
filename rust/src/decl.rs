use alloc_crate::boxed::Box;

mod body;
mod file;
mod id;
pub mod rich;
mod typeid;
mod union;

pub use crate::decl::{file::DeclFile, id::DeclNs};

pub mod items {
	pub use super::body::*;
	pub use super::file::{Item, ItemKind};
	pub use super::id::{FieldId, ItemId, VariantTag};
	pub use super::typeid::*;
	pub use super::union::*;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConstructError {
	UsedId,
	FullCapacity,
	IdOutOfRange,
	UnassignedSpace,
	UnassignedId,
	DuplicateName,
	InvalidName,
	DuplicateNotation,
}

pub struct ImportError {}

pub enum ImportResult<'file> {
	Ok(&'file DeclFile),
	Future(Box<dyn Future<Output = Result<DeclFile, ImportError>> + 'file>),
	Err(ImportError),
}

pub trait DeclProv {
	fn import<'file>(&'file self, path: &str) -> ImportResult<'file>;
	fn get(&self, id: DeclNs) -> &DeclFile;
}
