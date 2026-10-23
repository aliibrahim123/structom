#[cfg(feature = "fs-provider")]
use core::cell::Ref;
use core::{
	cell::{RefCell, UnsafeCell},
	fmt::Debug,
	ops::Deref,
};

use alloc_crate::{boxed::Box, collections::HashMap, string::ToString, vec::Vec};
use lean_string::LeanStr;

use crate::decl::{ConstructError, DeclFileBox, DeclNs, file::DeclFile};
#[cfg(feature = "fs-provider")]
use crate::parse::parse_decl_file;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportError {
	NotFound,
	#[cfg(feature = "parse")]
	Parse(crate::parse::ParseError),
	Other(Box<str>),
}
#[cfg(feature = "parse")]
impl From<crate::parse::ParseError> for ImportError {
	fn from(value: crate::parse::ParseError) -> Self {
		Self::Parse(value)
	}
}

pub enum ImportResult<'prov> {
	Ok(DeclFileBox),
	Future(
		Box<dyn Future<Output = Result<DeclFileBox, ImportError>> + 'prov + Send + Sync>,
	),
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
	fn import_file<'file>(&'file self, path: &str) -> ImportResult<'file>;
	fn get_file(&self, id: DeclNs) -> Option<DeclFileBox>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Void {}
impl DeclProv for Void {
	fn import_file<'file>(&'file self, _path: &str) -> ImportResult<'file> {
		ImportResult::Err(ImportError::Other(
			"the `Void` provider doesnt provide anything".into(),
		))
	}
	fn get_file(&self, _id: DeclNs) -> Option<DeclFileBox> {
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
		if self.name_index.contains_key(file.path()) {
			return Err(ConstructError::DuplicateName);
		}
		if self.ns_index.contains_key(&file.ns()) {
			return Err(ConstructError::AddedBefore);
		}
		self.name_index.insert(file.0.path.clone(), self.files.len());
		self.ns_index.insert(file.ns(), self.files.len());
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
	fn import_file<'file>(&'file self, path: &str) -> ImportResult<'file> {
		match self.name_index.get(path) {
			Some(&index) => ImportResult::Ok(self.files[index].clone()),
			None => ImportResult::Err(ImportError::NotFound),
		}
	}
	fn get_file(&self, id: DeclNs) -> Option<DeclFileBox> {
		self.ns_index.get(&id).map(|&index| self.files[index].clone())
	}
}

#[derive(Default)]
pub struct Router {
	providers: HashMap<LeanStr, Box<dyn DeclProv>>,
	file_index: RefCell<HashMap<DeclNs, DeclFileBox>>,
}
impl Debug for Router {
	fn fmt(&self, f: &mut alloc_crate::fmt::Formatter<'_>) -> alloc_crate::fmt::Result {
		f.debug_struct("Router")
			.field("providers", &self.providers.keys().collect::<Vec<_>>())
			.finish()
	}
}
impl Router {
	pub fn new() -> Router {
		Self::default()
	}
	pub fn add_provider(
		&mut self, prefix: &str, provider: Box<dyn DeclProv>,
	) -> Result<(), ConstructError> {
		if self.providers.contains_key(prefix) {
			return Err(ConstructError::DuplicateName);
		}
		self.providers.insert(LeanStr::from(prefix), provider);
		Ok(())
	}
	pub fn get_provider(&self, path: &str) -> Option<&dyn DeclProv> {
		let (prefix, _) = path.split_once('/')?;
		self.providers.get(prefix).map(|b| b.as_ref())
	}
}
impl DeclProv for Router {
	fn import_file<'file>(&'file self, path: &str) -> ImportResult<'file> {
		let Some((prefix, rest)) = path.split_once('/') else {
			return ImportResult::Err(ImportError::NotFound);
		};
		match self.get_provider(prefix) {
			Some(provider) => match provider.import_file(rest) {
				ImportResult::Ok(file) => {
					self.file_index.borrow_mut().insert(file.ns(), file.clone());
					ImportResult::Ok(file)
				}
				other @ _ => other,
			},
			None => ImportResult::Err(ImportError::NotFound),
		}
	}
	fn get_file(&self, id: DeclNs) -> Option<DeclFileBox> {
		if let Some(file) = self.file_index.borrow().get(&id) {
			return Some(file.clone());
		}
		for provider in self.providers.values() {
			if let Some(file) = provider.get_file(id) {
				return Some(file);
			}
		}
		None
	}
}

#[cfg(feature = "fs-provider")]
pub use alloc_crate::{
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
	Parse(crate::parse::ParseError),
}

#[cfg(feature = "fs-provider")]
#[derive(Debug, Clone)]
pub struct FsProv {
	root: PathBuf,
	list: RefCell<List>,
}
#[cfg(feature = "fs-provider")]
impl FsProv {
	pub fn new(root: impl AsRef<Path>) -> Result<FsProv, IoError> {
		let root = canonicalize(root)?;
		Ok(Self { root, list: RefCell::new(List::new()) })
	}
	pub fn root(&self) -> &Path {
		&self.root
	}
	pub fn files(&self) -> impl Deref<Target = [DeclFileBox]> {
		Ref::map(self.list.borrow(), |list| list.files())
	}
	pub fn load(&self, path: &str) -> Result<DeclFileBox, FsLoadError> {
		let path = self.root.join(path);
		let Some(path_str) = path.to_str() else { return Err(FsLoadError::NonUtf8Path) };
		if let ImportResult::Ok(file) = self.list.borrow().import_file(path_str) {
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

		_ = self.list.borrow_mut().add(file.clone());
		Ok(file)
	}
}
#[cfg(feature = "fs-provider")]
impl DeclProv for FsProv {
	fn import_file<'file>(&'file self, path: &str) -> ImportResult<'file> {
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
	fn get_file(&self, id: DeclNs) -> Option<DeclFileBox> {
		self.list.borrow().get_file(id)
	}
}
