use alloc_crate::{fmt::Write, string::String, vec::Vec};
use gramex::{Matcher, result::Expected};
use lean_string::LeanString;

use crate::{
	parse::{ParseError, Span, tokens::Source},
	utils::StrExt,
};

pub use gramex::cursor::Cursor as _;

#[derive(Debug)]
pub struct Cursor<'src, 'err> {
	pub src: &'src Source<'src>,
	pub off: usize,
	pub errors: &'err mut Vec<ParseError>,
	pub end_span: Span,
}
impl<'src> gramex::cursor::Cursor<'src> for Cursor<'src, '_> {
	type MatchAble = str;
	type Error = ();

	fn input(&self) -> &'src str {
		self.src.str
	}
	fn off(&self) -> usize {
		self.off
	}
	fn off_mut(&mut self) -> &mut usize {
		&mut self.off
	}
	fn map_error(&mut self, err: gramex::result::MatchError) -> Self::Error {
		let err = ParseError::from_match_error(&err, self.src, self.end_span, |buf| {
			let char = self.src.str.char_at(err.off).unwrap();
			_ = write!(buf, "`{char}`");
			Span::new(err.off, err.off + char.len_utf8())
		});
		self.errors.push(err);
	}
}
impl<'src> Cursor<'src, '_> {
	pub fn new<'err>(
		src: &'src Source<'src>, off: usize, errors: &'err mut Vec<ParseError>,
		end_span: Span,
	) -> Cursor<'src, 'err> {
		Cursor { src, off, errors, end_span }
	}
	pub fn try_char(&mut self, char: char) -> bool {
		self.try_eat(char).is_some()
	}
	pub fn test_next(&self, offset: usize, matcher: impl Matcher<str>) -> bool {
		matcher.test(self.src.str, &mut (self.off + offset))
	}
	pub fn try_until(&mut self, char: char) -> bool {
		let res = self.src.str[self.off..].find(char);
		self.off = res.map_or(self.src.str.len(), |i| self.off + i + char.len_utf8());
		res.is_some()
	}
	pub fn try_until_str(&mut self, chars: &str) -> bool {
		let res = self.src.str[self.off..].find(chars);
		self.off = res.map_or(self.src.str.len(), |i| self.off + i + chars.len());
		res.is_some()
	}
	pub fn try_eat_ident(&mut self) -> Option<&'src str> {
		let len = self.src.str[self.off..]
			.find(|c: char| !matches!(c, 'a'..='z' | 'A'..='Z' | '0'..='9' | '_'))
			.map_or(self.src.str.len(), |i| self.off + i);
		(len > 0).then(|| &self.src.str[self.off..len])
	}
	pub fn expected(&mut self, expected: impl Into<Expected>) {
		_ = <Self as gramex::cursor::Cursor>::expected::<()>(self, expected);
	}
	pub fn error(&mut self, msg: String, span: Span) {
		self.errors.push(ParseError::new(msg, span, self.src));
	}
}
