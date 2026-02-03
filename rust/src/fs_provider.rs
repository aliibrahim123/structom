use std::{
	cell::RefCell,
	collections::HashMap,
	fmt::Debug,
	fs::{canonicalize, read_to_string},
	io::{self, ErrorKind},
	path::{Path, PathBuf, absolute},
};

use elsa::FrozenMap;

use crate::{DeclFile, DeclProvider, ParseOptions, errors::ImportError, parse_declaration_file};

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
/// provider.load("commons.stomd").unwrap();
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
pub struct FSProvider {
	root: PathBuf,
	parse_options: ParseOptions,
	// files are only appended under shared reference since of `DeclProvider` protocol
	files: FrozenMap<u64, Box<DeclFile>>,
	files_by_name: RefCell<HashMap<PathBuf, u64>>,
}

impl Debug for FSProvider {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("FSProvider")
			.field("root", &self.root)
			.field("parse_options", &self.parse_options)
			.finish()
	}
}

impl FSProvider {
	/// creates a `FSProvider` working on a given root directory with default options.
	///
	/// fail if the root [canonicalize](std::fs::canonicalize) fail.
	pub fn new(root: impl Into<PathBuf>) -> io::Result<Self> {
		FSProvider::with_options(root, ParseOptions { relative_paths: true, metadata: false })
	}
	/// creates a `FSProvider` working on a given root directory with given options.
	/// ///
	/// fail if the root [canonicalize](std::fs::canonicalize) fail.
	pub fn with_options(root: impl Into<PathBuf>, parse_options: ParseOptions) -> io::Result<Self> {
		Ok(Self {
			root: canonicalize(root.into())?,
			parse_options,
			files: FrozenMap::new(),
			files_by_name: RefCell::new(HashMap::new()),
		})
	}

	/// load a declaration file at a given path if not loaded before.
	pub fn load_file<'a>(&'a self, path: impl AsRef<Path>) -> Result<&'a DeclFile, ImportError> {
		let path = absolute(Path::join(&self.root, path.as_ref()))
			.map_err(|e| ImportError::Other(e.to_string()))?;

		// already loaded
		if let Some(id) = self.files_by_name.borrow().get(&path) {
			return Ok(&self.files[id]);
		}

		// load
		if !path.starts_with(&self.root) {
			let msg = format!("importing outside root \"{}\"", path.display());
			return Err(ImportError::Other(msg));
		}

		let source = read_to_string(&path).map_err(|e| {
			if e.kind() == ErrorKind::NotFound {
				ImportError::NotFound
			} else {
				ImportError::Other(e.to_string())
			}
		})?;
		let file_name = path.to_string_lossy().to_string();
		let file = parse_declaration_file(&source, file_name, &self.parse_options, self)
			.map_err(ImportError::Parse)?;

		let id = file.id;
		self.files_by_name.borrow_mut().insert(path, file.id);
		self.files.insert(file.id, Box::new(file));

		Ok(&self.files[&id])
	}
}
impl DeclProvider for FSProvider {
	fn get(&self, id: u64) -> &DeclFile {
		&self.files[&id]
	}
	/// gets a decleration file with a given name.
	///
	/// load it if not loaded before, returns `None` if not found or can not be parsed.
	fn load<'a>(&'a self, name: &str) -> Result<&'a DeclFile, ImportError> {
		self.load_file(name)
	}
}
