mod base;
mod error;
mod str_cursor;
mod token_cursor;
mod tokenize;
mod tokens;
mod value;

use crate::decl::DeclFileBox;
pub use {error::ParseError, tokens::Span};

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseOptions {
	pub metadata: bool,
}
impl Default for ParseOptions {
	fn default() -> Self {
		Self { metadata: true }
	}
}

pub fn parse_decl_file(src: &str, path: &str) -> Result<DeclFileBox, ParseError> {
	todo!()
}
