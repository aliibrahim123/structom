use core::fmt;
use std::fmt::{Display, Formatter};

/// error encountered during parsing.
#[derive(Debug, Clone, PartialEq)]
pub enum ParserError {
	/// errors related to syntax (unexpected tokens, invalid literials...).
	SyntaxError(String),
	/// errors related to types (undefined types, undefined fields, ...).
	TypeError(String),

	ImportError(String),
}

impl Display for ParserError {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		match self {
			ParserError::SyntaxError(msg) => write!(f, "Syntax Error: {}", msg),
			ParserError::TypeError(msg) => write!(f, "Type Error: {}", msg),
			ParserError::ImportError(msg) => write!(f, "Import Error: {}", msg),
		}
	}
}

pub fn end_of_input(_len: usize) -> ParserError {
	ParserError::SyntaxError("end of input".to_string())
}

pub fn unexpected_token(token: impl ToString, ind: usize) -> ParserError {
	let token = token.to_string();
	if token == "end_of_file" {
		return end_of_input(ind);
	}
	ParserError::SyntaxError(format!("unexpected token `{token}` at {ind}"))
}

#[derive(Debug, Clone, PartialEq)]
pub enum ImportError {
	NotFound,
	Parse(ParserError),
	Other(String),
}
