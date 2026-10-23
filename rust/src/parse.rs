mod error;
mod source;
mod tokenize;

use crate::decl::DeclFileBox;
pub use {error::ParseError, source::Span};

pub fn parse_decl_file(src: &str, path: &str) -> Result<DeclFileBox, ParseError> {
	todo!()
}
