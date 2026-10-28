use core::{
	cell::{RefCell, UnsafeCell},
	fmt::Debug,
	ops::Deref,
	pin::Pin,
};

use alloc_crate::{boxed::Box, string::ToString, vec::Vec};
use hashbrown::{HashMap, hash_map::Entry};
use lean_string::LeanStr;

#[cfg(feature = "fs-provider")]
use crate::parse::parse_decl_file;
use crate::{
	decl::{ConstructError, DeclFile, DeclFileBox, DeclNs},
	value::Value,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportError {
	NotFound,
	#[cfg(feature = "parse")]
	Parse(Vec<crate::parse::ParseError>),
	Other(Box<str>),
}
#[cfg(feature = "parse")]
impl From<crate::parse::ParseError> for ImportError {
	fn from(value: crate::parse::ParseError) -> Self {
		use alloc_crate::prelude::v1::vec;
		Self::Parse(vec![value])
	}
}

pub enum ImportResult<'prov> {
	Ok(DeclFileBox),
	Future(Pin<Box<dyn Future<Output = Result<DeclFileBox, ImportError>> + 'prov>>),
	Err(ImportError),
}
impl Debug for ImportResult<'_> {
	fn fmt(&self, f: &mut alloc_crate::fmt::Formatter<'_>) -> alloc_crate::fmt::Result {
		match self {
			ImportResult::Ok(file) => f.debug_tuple("Ok").field(file).finish(),
			ImportResult::Future(_) => {
				f.debug_tuple("Future").field(&"<future>").finish()
			}
			ImportResult::Err(err) => f.debug_tuple("Err").field(err).finish(),
		}
	}
}

pub trait DeclProv {
	fn import_file<'file>(
		&'file self, path: &str, meta: Option<&Value>,
	) -> ImportResult<'file>;
	fn get_file(&self, id: DeclNs) -> Option<&DeclFile>;
	fn get_file_box(&self, id: DeclNs) -> Option<DeclFileBox>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Void {}
impl DeclProv for Void {
	fn import_file<'file>(
		&'file self, _path: &str, _value: Option<&Value>,
	) -> ImportResult<'file> {
		ImportResult::Err(ImportError::Other(
			"the `Void` provider doesnt provide anything".into(),
		))
	}
	fn get_file(&self, _id: DeclNs) -> Option<&DeclFile> {
		None
	}
	fn get_file_box(&self, _id: DeclNs) -> Option<DeclFileBox> {
		None
	}
}

#[derive(Debug, Clone, Default)]
pub struct List {
	files: Vec<DeclFileBox>,
	name_index: HashMap<LeanStr, usize>,
	ns_index: HashMap<DeclNs, usize>,
}
impl List {
	pub fn new() -> List {
		Self::default()
	}
	pub fn add(&mut self, file: DeclFileBox) -> Result<(), ConstructError> {
		let Entry::Vacant(name_entry) = self.name_index.entry(file.path.clone()) else {
			return Err(ConstructError::DuplicateName);
		};
		let Entry::Vacant(ns_entry) = self.ns_index.entry(file.ns()) else {
			return Err(ConstructError::AddedBefore);
		};
		name_entry.insert_entry(self.files.len());
		ns_entry.insert_entry(self.files.len());
		self.files.push(file);
		Ok(())
	}
	pub fn add_multiple(
		&mut self, files: impl IntoIterator<Item = DeclFileBox>,
	) -> Result<(), ConstructError> {
		for file in files {
			self.add(file)?;
		}
		Ok(())
	}
	pub fn files(&self) -> &[DeclFileBox] {
		&self.files
	}
}
impl DeclProv for List {
	fn import_file<'file>(
		&'file self, path: &str, _value: Option<&Value>,
	) -> ImportResult<'file> {
		match self.name_index.get(path) {
			Some(&index) => ImportResult::Ok(self.files[index].clone()),
			None => ImportResult::Err(ImportError::NotFound),
		}
	}
	fn get_file(&self, id: DeclNs) -> Option<&DeclFile> {
		self.ns_index.get(&id).map(|&index| self.files[index].deref())
	}
	fn get_file_box(&self, id: DeclNs) -> Option<DeclFileBox> {
		self.ns_index.get(&id).map(|&index| self.files[index].clone())
	}
}

#[derive(Debug, Default, Clone)]
struct ListIndex {
	name_index: HashMap<LeanStr, usize>,
	ns_index: HashMap<DeclNs, usize>,
}

#[derive(Debug, Default)]
pub struct GrowableList {
	files: UnsafeCell<Vec<DeclFileBox>>,
	index: RefCell<ListIndex>,
}
impl Clone for GrowableList {
	fn clone(&self) -> Self {
		Self {
			files: UnsafeCell::new(unsafe { (*self.files.get()).clone() }),
			index: self.index.clone(),
		}
	}
}
impl GrowableList {
	pub fn new() -> GrowableList {
		Self::default()
	}
	pub fn add(&self, file: DeclFileBox) -> Result<(), ConstructError> {
		let index = &mut *self.index.borrow_mut();
		let Entry::Vacant(name_entry) = index.name_index.entry(file.path.clone()) else {
			return Err(ConstructError::DuplicateName);
		};
		let Entry::Vacant(ns_entry) = index.ns_index.entry(file.ns()) else {
			return Err(ConstructError::AddedBefore);
		};
		let len = unsafe { (*self.files.get()).len() };
		name_entry.insert_entry(len);
		ns_entry.insert_entry(len);
		unsafe { (*self.files.get()).push(file) }
		Ok(())
	}
	pub fn add_multiple(
		&self, files: impl IntoIterator<Item = DeclFileBox>,
	) -> Result<(), ConstructError> {
		for file in files {
			self.add(file)?;
		}
		Ok(())
	}
	fn index_file(&self, ind: usize) -> Option<&DeclFileBox> {
		unsafe { (&*self.files.get()).get(ind) }
	}
	pub fn files(&self) -> impl Iterator<Item = &DeclFile> {
		struct Iter<'a> {
			list: &'a GrowableList,
			ind: usize,
		}
		impl<'a> Iterator for Iter<'a> {
			type Item = &'a DeclFile;
			fn next(&mut self) -> Option<Self::Item> {
				let file = self.list.index_file(self.ind).map(|v| v.deref());
				file.inspect(|_| self.ind += 1)
			}
		}
		Iter { list: self, ind: 0 }
	}
}
impl DeclProv for GrowableList {
	fn import_file<'file>(
		&'file self, path: &str, _value: Option<&Value>,
	) -> ImportResult<'file> {
		match self.index.borrow().name_index.get(path) {
			Some(&index) => ImportResult::Ok(self.index_file(index).unwrap().clone()),
			None => ImportResult::Err(ImportError::NotFound),
		}
	}
	fn get_file(&self, id: DeclNs) -> Option<&DeclFile> {
		let ns_index = &self.index.borrow().ns_index;
		ns_index.get(&id).map(|&index| self.index_file(index).unwrap().deref())
	}
	fn get_file_box(&self, id: DeclNs) -> Option<DeclFileBox> {
		let ns_index = &self.index.borrow().ns_index;
		ns_index.get(&id).map(|&index| self.index_file(index).unwrap().clone())
	}
}

#[cfg(feature = "fs-provider")]
use alloc_crate::{
	fs::{canonicalize, read_to_string},
	io::{Error as IoError, ErrorKind},
	path::{Path, PathBuf},
};

#[cfg(feature = "fs-provider")]
#[derive(Debug)]
pub enum FsLoadError {
	NotFound,
	OutsideRoot,
	NonUtf8Path,
	Io(IoError),
	Parse(Vec<crate::parse::ParseError>),
}

#[cfg(feature = "fs-provider")]
#[derive(Debug, Clone)]
pub struct FsProv {
	root: PathBuf,
	list: GrowableList,
}
#[cfg(feature = "fs-provider")]
impl FsProv {
	pub fn new(root: impl AsRef<Path>) -> Result<FsProv, IoError> {
		let root = canonicalize(root)?;
		Ok(Self { root, list: GrowableList::new() })
	}
	pub fn root(&self) -> &Path {
		&self.root
	}
	pub fn files(&self) -> impl Iterator<Item = &DeclFile> {
		self.list.files()
	}
	pub fn load(&self, path: &str) -> Result<DeclFileBox, FsLoadError> {
		let path = self.root.join(path);
		let Some(path_str) = path.to_str() else { return Err(FsLoadError::NonUtf8Path) };
		if let ImportResult::Ok(file) = self.list.import_file(path_str, None) {
			return Ok(file);
		}

		let path = canonicalize(&path).map_err(|err| {
			if err.kind() == ErrorKind::NotFound {
				FsLoadError::NotFound
			} else {
				FsLoadError::Io(err)
			}
		})?;
		if !path.starts_with(&self.root) {
			return Err(FsLoadError::OutsideRoot);
		}

		let src = read_to_string(path).map_err(FsLoadError::Io)?;
		let file = parse_decl_file(&src, path_str).map_err(FsLoadError::Parse)?;

		_ = self.list.add(file.clone());
		Ok(file)
	}
}
#[cfg(feature = "fs-provider")]
impl DeclProv for FsProv {
	fn import_file<'file>(
		&'file self, path: &str, _value: Option<&Value>,
	) -> ImportResult<'file> {
		match self.load(path) {
			Ok(file) => ImportResult::Ok(file),
			Err(FsLoadError::NotFound) => ImportResult::Err(ImportError::NotFound),
			Err(FsLoadError::NonUtf8Path) => {
				ImportResult::Err(ImportError::Other("path is not utf8".into()))
			}
			Err(FsLoadError::OutsideRoot) => {
				ImportResult::Err(ImportError::Other("path is outside the root".into()))
			}
			Err(FsLoadError::Io(err)) => {
				ImportResult::Err(ImportError::Other(err.to_string().into()))
			}
			Err(FsLoadError::Parse(err)) => ImportResult::Err(ImportError::Parse(err)),
		}
	}
	fn get_file_box(&self, id: DeclNs) -> Option<DeclFileBox> {
		self.list.get_file_box(id)
	}
	fn get_file(&self, id: DeclNs) -> Option<&DeclFile> {
		self.list.get_file(id)
	}
}
