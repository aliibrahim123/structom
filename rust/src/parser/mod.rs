mod declaration;
mod rich_types;
mod tokenizer;
mod utils;
mod value;

use crate::{
	DeclProvider, Error, Value,
	declaration::{DeclFile, DeclItem, StructDef, TypeId},
	errors::unexpected_token,
	parser::{
		declaration::{DeclContext, parse_declaration},
		tokenizer::tokenize,
		value::parse_value,
	},
};

#[derive(Debug)]
pub struct ParseOptions {
	pub metadata: bool,
}

impl Default for ParseOptions {
	fn default() -> Self {
		Self { metadata: false }
	}
}

pub fn parse_declaration_file(
	source: &str, name: String, options: &ParseOptions, provider: &dyn DeclProvider,
) -> Result<DeclFile, Error> {
	let tokens = tokenize(source)?;
	let mut ind = 0;

	let mut file = DeclFile::new(name);
	parse_declaration(&mut file, &tokens, &mut ind, provider, options)?;

	if ind != tokens.len() - 1 {
		return Err(unexpected_token(&tokens[ind], tokens[ind].ind()));
	}
	if file.items.len() == 0 {
		return Err(Error::TypeError(format!("no declaration in file \"{}\"", file.name)));
	}

	Ok(file)
}
pub fn parse(
	source: &str, options: &ParseOptions, provider: &dyn DeclProvider,
) -> Result<Value, Error> {
	let tokens = tokenize(source)?;
	let mut ind = 0;

	let mut root_file = DeclFile::new("root".to_string());
	let ctx = parse_declaration(&mut root_file, &tokens, &mut ind, provider, options)?;

	let value = value::parse_value(&tokens, &mut ind, &TypeId::ANY, &ctx, provider, options)?;
	if tokens.len() - 1 != ind {
		return Err(unexpected_token(&tokens[ind], tokens[ind].ind()));
	}

	Ok(value)
}
