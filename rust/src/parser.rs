mod declaration;
mod rich_types;
pub mod tokenizer;
mod utils;
mod value;

use crate::{
	DeclProvider, ParserError, Value,
	declaration::{DeclFile, TypeId},
	errors::unexpected_token,
	parser::{declaration::parse_declaration, tokenizer::tokenize},
};

/// parsing options.
#[derive(Debug, Clone)]
pub struct ParseOptions {
	/// whether to keep metadata in result, default: `false`.
	pub metadata: bool,
}

impl Default for ParseOptions {
	fn default() -> Self {
		Self { metadata: false }
	}
}

/// parse a decleration file into a [`DeclFile`].
///
/// it takes a name for the file, parsing options and a decleration provider used to resolve imports.
///
/// ## example
/// ```
/// let file = parse_declaration_file(
/// 	"struct A { v: vint }", "file".to_string(), &ParseOptions::default(), &VoidProvider{}
/// ).unwrap();
/// ```
pub fn parse_declaration_file(
	source: &str, name: String, options: &ParseOptions, provider: &dyn DeclProvider,
) -> Result<DeclFile, ParserError> {
	let tokens = tokenize(source)?;
	let mut ind = 0;

	let mut file = DeclFile::new(name);
	parse_declaration(&mut file, &tokens, &mut ind, provider, options)?;

	// ensure all tokens have been consumed
	if ind != tokens.len() - 1 {
		return Err(unexpected_token(&tokens[ind], tokens[ind].ind()));
	}
	// ensure file is not empty
	if file.items.len() == 0 {
		return Err(ParserError::TypeError(format!("no declaration in file \"{}\"", file.name)));
	}

	Ok(file)
}
/// parse a structom file into a [`Value`].
///
/// the source is made up of optional declerations at top, followed by a root value.
///
/// it takes parsing options and a decleration provider used to resolve imports.
///
/// for info on how the values are represented, see the [`Value`] documentation.
///
/// ## example
/// ```
/// let value = parse(
/// 	"{ nb: 1, str: \"hello\", bool: true, arr: [1, 2, 3] }", &ParseOptions::default(), &VoidProvider{}
/// ).unwrap();
/// ```
pub fn parse(
	source: &str, options: &ParseOptions, provider: &dyn DeclProvider,
) -> Result<Value, ParserError> {
	let tokens = tokenize(source)?;
	let mut ind = 0;

	let mut root_file = DeclFile::new("root".to_string());
	let ctx = parse_declaration(&mut root_file, &tokens, &mut ind, provider, options)?;

	let value = value::parse_value(&tokens, &mut ind, &TypeId::ANY, &ctx, provider, options)?;
	// ensure all tokens have been consumed
	if tokens.len() - 1 != ind {
		return Err(unexpected_token(&tokens[ind], tokens[ind].ind()));
	}

	Ok(value)
}
