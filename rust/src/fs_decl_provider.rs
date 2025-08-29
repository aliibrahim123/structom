use std::{
	cell::RefCell,
	collections::HashMap,
	fs::{canonicalize, read_to_string},
	io,
	path::{Path, PathBuf},
};

use crate::{DeclFile, DeclProvider, Error, ParseOptions, parse_declaration_file};

#[derive(Debug)]
pub enum LoadFileError {
	IO(io::Error),
	Parse(Error),
}
#[derive(Debug)]
pub struct FSProvider {
	root: PathBuf,
	parse_options: ParseOptions,
	cache: RefCell<ProviderCache>,
}
#[derive(Debug, Default)]
struct ProviderCache {
	files: HashMap<u64, Box<DeclFile>>,
	files_by_name: HashMap<PathBuf, u64>,
}

impl FSProvider {
	pub fn new(path: impl Into<PathBuf>) -> io::Result<Self> {
		FSProvider::with_options(path, ParseOptions::default())
	}
	pub fn with_options(path: impl Into<PathBuf>, parse_options: ParseOptions) -> io::Result<Self> {
		Ok(Self { root: canonicalize(path.into())?, parse_options, cache: Default::default() })
	}

	pub fn load_file<'a>(&'a self, path: impl AsRef<Path>) -> Result<&'a DeclFile, LoadFileError> {
		let path = Path::join(&self.root, path.as_ref());
		println!("loading {:?}", path);
		{
			let cache = self.cache.borrow();
			if let Some(id) = cache.files_by_name.get(&path) {
				return Ok(unsafe {
					&*(cache.files.get(&id).unwrap().as_ref() as *const DeclFile)
				});
			}
		}

		if !path.starts_with(&self.root) {
			return Err(LoadFileError::IO(io::Error::from(io::ErrorKind::NotFound)));
		}
		let source = read_to_string(&path).map_err(LoadFileError::IO)?;
		let file_name = path.to_str().unwrap().to_string();
		let file = parse_declaration_file(&source, file_name, &self.parse_options, self)
			.map_err(LoadFileError::Parse)?;

		let mut cache = self.cache.borrow_mut();
		let id = file.id;
		cache.files.insert(file.id, Box::new(file));
		cache.files_by_name.insert(path, id);

		Ok(unsafe { &*(cache.files.get(&id).unwrap().as_ref() as *const DeclFile) })
	}
}
impl DeclProvider for FSProvider {
	fn get_by_id(&self, id: u64) -> &DeclFile {
		unsafe { &*(self.cache.borrow().files.get(&id).unwrap().as_ref() as *const DeclFile) }
	}
	fn get_by_name<'a>(&'a self, name: &str) -> Option<&'a DeclFile> {
		self.load_file(name).inspect_err(|e| println!("{:?}", e)).ok()
	}
}
