use core::fmt;
use std::{
	fmt::{Display, Formatter, Write},
	ops::Deref,
};

use crate::parser::tokenizer::Pos;

#[derive(Debug, Clone, PartialEq)]
struct ParseErrorData {
	// zero if no position
	pos: Pos,
	msg: Box<str>,

	// offset and size of file in msg
	at_offset: u16,
	at_size: u16,
}

/// error encountered during parsing.
#[derive(Debug, Clone, PartialEq)]
pub struct ParseError {
	data: Box<ParseErrorData>,
}

impl Display for ParseError {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		write!(f, "{}", self.data.msg)
	}
}

impl ParseError {
	/// position in source
	pub fn pos(&self) -> Option<Pos> {
		if self.data.pos == (0, 0) { None } else { Some(self.data.pos) }
	}
	/// file where the error occured
	pub fn at(&self) -> &str {
		let ParseErrorData { msg, at_offset, at_size, .. } = self.data.deref();
		&msg[*at_offset as usize..(at_offset + at_size) as usize]
	}

	pub(crate) fn with_pos<T>(mut msg: String, pos: Pos, file: &str) -> Result<T, ParseError> {
		let at_offset = (msg.len() + " at \"".len()) as u16;
		write!(msg, " at \"{file}:{pos}\"").unwrap();
		let at_size = file.len() as u16;
		let msg = msg.into_boxed_str();
		Err(Self { data: Box::new(ParseErrorData { pos, msg, at_offset, at_size }) })
	}
	pub(crate) fn new<T>(mut msg: String, file: &str) -> Result<T, ParseError> {
		let at_offset = (msg.len() + " at \"".len()) as u16;
		write!(msg, " at \"{file}\"").unwrap();
		let pos = Pos::new(0, 0);
		let at_size = file.len() as u16;
		let msg = msg.into_boxed_str();
		Err(Self { data: Box::new(ParseErrorData { pos, msg, at_offset, at_size }) })
	}
}

/// create an error, with/without position
macro_rules! err {
	($str:expr, $file:expr) => {
		ParseError::new($str, $file)
	};
	($str:expr, $pos:expr, $file:expr) => {
		ParseError::with_pos($str, $pos, $file)
	};
}
pub(crate) use err;

#[derive(Debug, Clone, PartialEq)]
pub enum ImportError {
	NotFound,
	Parse(ParseError),
	Other(String),
}
