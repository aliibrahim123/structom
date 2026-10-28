mod base;
mod decl;
mod error;
pub(crate) mod head;
pub(crate) mod str_cursor;
mod token_cursor;
mod tokenize;
pub(crate) mod tokens;
mod value;

use alloc_crate::vec::Vec;

use crate::decl::DeclFileBox;
pub use {error::ParseError, tokens::Span};

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseOptions {
	pub metadata: bool,
	pub dump_unresolved: bool,
}
impl Default for ParseOptions {
	fn default() -> Self {
		Self { metadata: false, dump_unresolved: false }
	}
}

pub fn parse_decl_file(src: &str, path: &str) -> Result<DeclFileBox, Vec<ParseError>> {
	todo!()
}
