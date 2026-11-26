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
	decl::{DeclFile, DeclFileBox, DeclNs, file::DeclFileBuilder},
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
	fn get_file(&self, ns: DeclNs) -> Option<&DeclFile>;
	fn get_file_box(&self, ns: DeclNs) -> Option<DeclFileBox>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Void;
impl DeclProv for Void {
	fn import_file<'file>(
		&'file self, _path: &str, _value: Option<&Value>,
	) -> ImportResult<'file> {
		ImportResult::Err(ImportError::Other(
			"the `Void` provider doesnt provide anything".into(),
		))
	}
	fn get_file(&self, _ns: DeclNs) -> Option<&DeclFile> {
		None
	}
	fn get_file_box(&self, _ns: DeclNs) -> Option<DeclFileBox> {
		None
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddError {
	DuplicateName,
	AddedBefore,
	NonContiguous,
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
	pub fn add(&mut self, file: DeclFileBox) -> Result<(), AddError> {
		let Entry::Vacant(name_entry) = self.name_index.entry(file.path.clone()) else {
			return Err(AddError::DuplicateName);
		};
		let Entry::Vacant(ns_entry) = self.ns_index.entry(file.ns()) else {
			return Err(AddError::AddedBefore);
		};
		name_entry.insert_entry(self.files.len());
		ns_entry.insert_entry(self.files.len());
		self.files.push(file);
		Ok(())
	}
	pub fn add_multiple(
		&mut self, files: impl IntoIterator<Item = DeclFileBox>,
	) -> Result<(), AddError> {
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
	fn get_file(&self, ns: DeclNs) -> Option<&DeclFile> {
		self.ns_index.get(&ns).map(|&index| self.files[index].deref())
	}
	fn get_file_box(&self, ns: DeclNs) -> Option<DeclFileBox> {
		self.ns_index.get(&ns).map(|&index| self.files[index].clone())
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
	pub fn add(&self, file: DeclFileBox) -> Result<(), AddError> {
		let index = &mut *self.index.borrow_mut();
		let Entry::Vacant(name_entry) = index.name_index.entry(file.path.clone()) else {
			return Err(AddError::DuplicateName);
		};
		let Entry::Vacant(ns_entry) = index.ns_index.entry(file.ns()) else {
			return Err(AddError::AddedBefore);
		};
		let len = unsafe { (*self.files.get()).len() };
		name_entry.insert_entry(len);
		ns_entry.insert_entry(len);
		unsafe { (*self.files.get()).push(file) }
		Ok(())
	}
	pub fn add_multiple(
		&self, files: impl IntoIterator<Item = DeclFileBox>,
	) -> Result<(), AddError> {
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
	fn get_file(&self, ns: DeclNs) -> Option<&DeclFile> {
		let ns_index = &self.index.borrow().ns_index;
		ns_index.get(&ns).map(|&index| self.index_file(index).unwrap().deref())
	}
	fn get_file_box(&self, ns: DeclNs) -> Option<DeclFileBox> {
		let ns_index = &self.index.borrow().ns_index;
		ns_index.get(&ns).map(|&index| self.index_file(index).unwrap().clone())
	}
}

#[derive(Debug, Clone)]
pub struct BatchList {
	start: u64,
	files: Box<[DeclFileBox]>,
	name_index: HashMap<LeanStr, usize>,
}
impl BatchList {
	pub fn new(files: impl IntoIterator<Item = DeclFileBox>) -> Result<Self, AddError> {
		let files = Box::from_iter(files);
		let start = files.get(0).map_or(0, |file| file.ns().0);
		if files.iter().enumerate().any(|(ind, file)| file.ns().0 != start + ind as u64) {
			return Err(AddError::NonContiguous);
		}
		let name_index =
			files.iter().map(|f| (f.path.clone(), (f.ns().0 - start) as _)).collect();
		Ok(Self { start, files, name_index })
	}
	pub fn start(&self) -> DeclNs {
		DeclNs(self.start)
	}
	pub fn files(&self) -> &[DeclFileBox] {
		&self.files
	}
}
impl DeclProv for BatchList {
	fn import_file<'file>(
		&'file self, path: &str, _value: Option<&Value>,
	) -> ImportResult<'file> {
		match self.name_index.get(path) {
			Some(&index) => ImportResult::Ok(self.files[index].clone()),
			None => ImportResult::Err(ImportError::NotFound),
		}
	}
	fn get_file(&self, ns: DeclNs) -> Option<&DeclFile> {
		self.files.get(ns.0.checked_sub(self.start)? as usize).map(|f| f.deref())
	}
	fn get_file_box(&self, ns: DeclNs) -> Option<DeclFileBox> {
		self.files.get(ns.0.checked_sub(self.start)? as usize).map(|f| f.clone())
	}
}

#[derive(Debug, Clone)]
enum LocalFile<'build> {
	Box(DeclFileBox),
	Build(&'build DeclFileBuilder),
}

#[derive(Clone)]
pub struct LocalScope<'files> {
	local: LocalFile<'files>,
	provider: &'files dyn DeclProv,
}
impl LocalScope<'_> {
	pub fn new_box<'files>(
		local: DeclFileBox, provider: &'files dyn DeclProv,
	) -> LocalScope<'files> {
		LocalScope { local: LocalFile::Box(local), provider }
	}
	pub fn new_build<'files>(
		local: &'files DeclFileBuilder, provider: &'files dyn DeclProv,
	) -> LocalScope<'files> {
		LocalScope { local: LocalFile::Build(local), provider }
	}
	pub fn local(&self) -> &DeclFile {
		match &self.local {
			LocalFile::Box(file) => file,
			LocalFile::Build(file) => file,
		}
	}
	pub fn provider(&self) -> &dyn DeclProv {
		self.provider
	}
}
impl DeclProv for LocalScope<'_> {
	fn import_file<'file>(
		&'file self, path: &str, meta: Option<&Value>,
	) -> ImportResult<'file> {
		self.provider.import_file(path, meta)
	}
	fn get_file(&self, ns: DeclNs) -> Option<&DeclFile> {
		let local = self.local();
		if local.ns() == ns { Some(local) } else { self.provider.get_file(ns) }
	}

	fn get_file_box(&self, ns: DeclNs) -> Option<DeclFileBox> {
		if let LocalFile::Box(local) = &self.local
			&& local.ns() == ns
		{
			Some(local.clone())
		} else {
			self.provider.get_file_box(ns)
		}
	}
}
impl Debug for LocalScope<'_> {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		f.debug_struct("LocalFile")
			.field("file", &self.local)
			.field("fallback", &"...")
			.finish()
	}
}

#[cfg(feature = "fs-provider")]
use alloc_crate::{
	fs::{canonicalize, read_to_string},
	io::{Error as IoError, ErrorKind},
	path::{Path, PathBuf},
};

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
}
#[cfg(feature = "fs-provider")]
impl DeclProv for FsProv {
	fn import_file<'file>(
		&'file self, path: &str, _value: Option<&Value>,
	) -> ImportResult<'file> {
		let path = self.root.join(path);
		let Some(path_str) = path.to_str() else {
			return ImportResult::Err(ImportError::Other("path is not utf8".into()));
		};
		if let ImportResult::Ok(file) = self.list.import_file(path_str, None) {
			return ImportResult::Ok(file);
		}

		let path = match canonicalize(&path) {
			Ok(path) => path,
			Err(err) if err.kind() == ErrorKind::NotFound => {
				return ImportResult::Err(ImportError::NotFound);
			}
			Err(err) => {
				return ImportResult::Err(ImportError::Other(err.to_string().into()));
			}
		};

		if !path.starts_with(&self.root) {
			return ImportResult::Err(ImportError::Other(
				"path is outside the root".into(),
			));
		}

		let src = match read_to_string(path) {
			Ok(src) => src,
			Err(err) => {
				return ImportResult::Err(ImportError::Other(err.to_string().into()));
			}
		};
		let file = match parse_decl_file(&src, path_str) {
			Ok(src) => src,
			Err(err) => return ImportResult::Err(ImportError::Parse(err)),
		};

		_ = self.list.add(file.clone());
		ImportResult::Ok(file)
	}
	fn get_file_box(&self, ns: DeclNs) -> Option<DeclFileBox> {
		self.list.get_file_box(ns)
	}
	fn get_file(&self, ns: DeclNs) -> Option<&DeclFile> {
		self.list.get_file(ns)
	}
}
