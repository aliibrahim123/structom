mod declaration;
mod rich_types;
pub mod tokenizer;
pub mod utils;
mod value;

use crate::{
	DeclProvider, ParseError, Value,
	declaration::{DeclFile, TypeId},
	errors::{ImportError, err},
	parser::{
		declaration::{DeclContext, parse_declarations},
		tokenizer::tokenize,
		utils::unexpected_token,
		value::ValueCtx,
	},
};

/// parsing options.
#[derive(Debug, Clone)]
pub struct ParseOptions {
	/// whether to keep metadata in result, default: `false`.
	pub metadata: bool,

	/// whether to resolve relative paths, default: `true`.
	///
	/// support only at the start of the path
	///
	/// ```
	/// // in a/folder/file
	/// import "../b/file" // resolved to a/b/file
	/// ```
	pub relative_paths: bool,
}

impl Default for ParseOptions {
	fn default() -> Self {
		Self { metadata: false, relative_paths: true }
	}
}

/// parse a decleration file into a [`DeclFile`].
///
/// return [`ParseError`] if the source is invalid.
///
/// ## example
/// ```
/// let file = parse_declaration_file(
/// 	"struct A { v: vint }", "file".to_string(), &ParseOptions::default(), &VoidProvider{}
/// ).unwrap();
/// ```
pub fn parse_declaration_file(
	source: &str, name: String, options: &ParseOptions, provider: &dyn DeclProvider,
) -> Result<DeclFile, ParseError> {
	let tokens = tokenize(source, &name)?;
	let mut ind = 0;

	let mut file = DeclFile::new(name);
	parse_declarations(&mut file, &tokens, &mut ind, provider, options)?;

	if ind != tokens.len() - 1 {
		return unexpected_token(&tokens[ind], tokens[ind].pos(), &file.name);
	}
	if file.items.len() == 0 {
		return err!(format!("no declaration"), &file.name);
	}

	Ok(file)
}

/// resolve from local declerations then from the provider
struct MiddleProvider<'a> {
	provider: &'a dyn DeclProvider,
	ctx: &'a DeclContext<'a>,
}
impl DeclProvider for MiddleProvider<'_> {
	fn get<'a>(&'a self, id: u64) -> &'a DeclFile {
		if id == self.ctx.file.id { self.ctx.file } else { self.provider.get(id) }
	}
	fn load<'a>(&'a self, name: &str) -> Result<&'a DeclFile, ImportError> {
		self.provider.load(name)
	}
}

/// parse structom object notation into a [`Value`].
///
/// the source is made up of optional declerations at top, followed by a root value.
///
/// returns [`ParseError`] if the source is invalid.
///
/// for the other way see [`stringify`](crate::stringify())
///
/// ## example
/// ```
/// let value = parse(
/// 	"{ nb: 1, str: \"hello\", bool: true, arr: [1, 2, 3] }", &ParseOptions::default(), &VoidProvider{}
/// ).unwrap();
/// ```
pub fn parse(
	source: &str, options: &ParseOptions, provider: &dyn DeclProvider,
) -> Result<Value, ParseError> {
	let tokens = tokenize(source, "root")?;
	let mut ind = 0;

	let mut root_file = DeclFile::new("root".to_string());
	let ctx = parse_declarations(&mut root_file, &tokens, &mut ind, provider, options)?;

	let provider = &MiddleProvider { provider, ctx: &ctx };
	let ctx = ValueCtx { file: "root", decl: &ctx, provider, options };
	let value = value::parse_value(&tokens, &mut ind, &TypeId::ANY, &ctx)?;
	if tokens.len() - 1 != ind {
		return unexpected_token(&tokens[ind], tokens[ind].pos(), "root");
	}

	Ok(value)
}
