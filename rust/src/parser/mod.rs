mod declaration;
pub mod tokenizer;
mod utils;

use crate::{
	DeclProvider, Error, Value,
	declaration::{DeclFile, DeclItem},
	errors::unexpected_token,
	parser::{declaration::parse_declaration, tokenizer::tokenize},
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
	source: &str,
	name: String,
	options: &ParseOptions,
	provider: &dyn DeclProvider,
) -> Result<DeclFile, Error> {
	let tokens = tokenize(source)?;
	let mut ind = 0;

	let mut file = DeclFile::new(name);
	parse_declaration(&mut file, &tokens, &mut ind, options, provider)?;

	if ind != tokens.len() - 1 {
		return Err(unexpected_token(&tokens[ind], tokens[ind].ind()));
	}
	if (file.items.len() == 0) {
		return Err(Error::TypeError(format!(
			"no declaration in file \"{}\"",
			file.name
		)));
	}

	Ok(file)
}
pub fn parse_value(
	source: &str,
	provider: &dyn DeclProvider,
	declarations: &[DeclItem],
) -> Result<Value, Error> {
	todo!()
}
pub fn parse(source: &str, provider: &dyn DeclProvider) -> Result<Value, Error> {
	let tokens = tokenize(source)?;

	Ok(Value::default())
}
