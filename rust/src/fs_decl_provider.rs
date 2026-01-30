use std::{
	cell::RefCell,
	collections::HashMap,
	fs::{canonicalize, read_to_string},
	io,
	path::{Path, PathBuf, absolute},
};

use crate::{
	DeclFile, DeclProvider, ParseOptions, ParserError, errors::ImportError, parse_declaration_file,
};

/// provider that loads declerations from the file system.
///
/// this provider syncronously loads decleration files from the file system, and caches them for future use.
///
/// this provider works only in a specifed root directory, and loads files of any extension.
///
/// it can fails safely when loading.
///
/// ## example
/// ```
/// let provider = FSProvider::new("/path/to/decls").unwrap();
///
/// // cache common files
/// provider.load_file("commons.stomd").unwrap();
///
/// // loads other.stomd, commons.stomd is cached
/// parse(
/// 	"import \"commons.stomd\" import \"other.stomd\" ... ",
/// 	&ParseOptions::default(), &provider
/// ).unwrap();
///
/// // fails in loading not_found.stomd
/// assert!(parse("import \"not_found.stomd\" ... ", &ParseOptions::default(), &provider).is_err() == true);
/// ```
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
	/// creates a `FSProvider` working on a given root directory with default options.
	pub fn new(root: impl Into<PathBuf>) -> io::Result<Self> {
		FSProvider::with_options(root, ParseOptions::default())
	}
	/// creates a `FSProvider` working on a given root directory with given options.
	pub fn with_options(root: impl Into<PathBuf>, parse_options: ParseOptions) -> io::Result<Self> {
		Ok(Self { root: canonicalize(root.into())?, parse_options, cache: Default::default() })
	}

	/// load a declaration file at a given path.
	///
	/// returns a reference to the cached file if used before, else load it and returns `LoadFileError` if an error occurs.
	pub fn load_file<'a>(&'a self, path: impl AsRef<Path>) -> Result<&'a DeclFile, ImportError> {
		let path = absolute(Path::join(&self.root, path.as_ref()))
			.map_err(|e| ImportError::Other(e.to_string()))?;
		{
			let cache = self.cache.borrow();
			if let Some(id) = cache.files_by_name.get(&path) {
				return Ok(unsafe {
					&*(cache.files.get(&id).unwrap().as_ref() as *const DeclFile)
				});
			}
		}

		if !path.starts_with(&self.root) {
			return Err(ImportError::Other(format!(
				"importing outside root \"{}\"",
				path.display()
			)));
		}
		let source = read_to_string(&path).map_err(|e| ImportError::Other(e.to_string()))?;
		let file_name = path.to_str().unwrap().to_string();
		let file = parse_declaration_file(&source, file_name, &self.parse_options, self)
			.map_err(ImportError::Parse)?;

		let mut cache = self.cache.borrow_mut();
		let id = file.id;
		cache.files.insert(file.id, Box::new(file));
		cache.files_by_name.insert(path, id);

		Ok(unsafe { &*(cache.files.get(&id).unwrap().as_ref() as *const DeclFile) })
	}
}
impl DeclProvider for FSProvider {
	fn get(&self, id: u64) -> &DeclFile {
		unsafe { &*(self.cache.borrow().files.get(&id).unwrap().as_ref() as *const DeclFile) }
	}
	/// gets a decleration file with a given name.
	///
	/// load it if not loaded before, returns `None` if not found or can not be parsed.
	fn load<'a>(&'a self, name: &str) -> Result<&'a DeclFile, ImportError> {
		self.load_file(name)
	}
}
