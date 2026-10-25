use core::fmt;

use alloc_crate::{
	boxed::Box,
	fmt::{Display, Write},
	format,
	string::String,
};
use gramex::result::{MatchError, MatchErrorKind};

use crate::{
	parse::tokens::{Source, Span},
	utils::StrExt,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ParseError {
	full: Box<str>,
	span: Span,
	msg_len: u32,
	file_len: u32,
}
impl ParseError {
	pub(crate) fn from_match_error(err: MatchError, src: &Source) -> Self {
		let (msg, span) = match err.kind {
			MatchErrorKind::MisMatch(expected) => {
				let char = src.str.char_at(err.off).unwrap();
				(
					format!("expected {expected}, found `{char}`",),
					Span::new(err.off, err.off + char.len_utf8()),
				)
			}
			MatchErrorKind::InComplete(expected) => {
				(format!("incomplete input, expected {expected}"), Span::EMPTY)
			}
			_ => unreachable!(),
		};
		Self::new(msg, span, src)
	}
	pub(crate) fn new(mut msg: String, span: Span, src: &Source) -> Self {
		let msg_len = msg.len() as _;
		msg.push_str(MSG_FILE_SEP);
		msg.push_str(src.path);

		if span.is_point() {
			let (line, col) = src.pos_of(span.start());
			write!(msg, ":{line}:{col}",).unwrap();
		} else if span != Span::EMPTY {
			let (start_line, start_col) = src.pos_of(span.start());
			let (end_line, end_col) = src.pos_of(span.end());
			write!(msg, ":{start_line}:{start_col}-{end_line}:{end_col}",).unwrap();
		}
		Self { full: msg.into_boxed_str(), span, msg_len, file_len: src.path.len() as _ }
	}
	pub fn msg(&self) -> &str {
		&self.full[..self.msg_len as usize]
	}
	pub fn span(&self) -> Span {
		self.span
	}
	pub fn file(&self) -> &str {
		let file_start = self.msg_len as usize + MSG_FILE_SEP.len();
		&self.full[file_start..file_start + self.file_len as usize]
	}
}
const MSG_FILE_SEP: &str = "\n  --> ";
impl Display for ParseError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "{}", self.full)
	}
}
impl core::error::Error for ParseError {}
