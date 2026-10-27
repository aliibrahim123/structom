use core::fmt::Display;

use alloc_crate::{fmt::Write, string::String, vec::Vec};
use gramex::{Matcher, result::Expected};

use crate::{
	parse::{ParseError, Span, tokens::Source},
	utils::StrExt,
};

pub use gramex::cursor::Cursor as _;

#[derive(Debug, Clone)]
pub struct Ident<'src> {
	pub val: &'src str,
	pub span: Span,
}
impl PartialEq<&str> for Ident<'_> {
	fn eq(&self, other: &&str) -> bool {
		self.val == *other
	}
}
impl Display for Ident<'_> {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		f.write_str(self.val)
	}
}

#[derive(Debug)]
pub struct Cursor<'src, 'err> {
	pub slice: &'src str,
	pub off: usize,
	pub src: &'src Source<'src>,
	pub errors: &'err mut Vec<ParseError>,
	pub end_span: Span,
}
impl<'src> gramex::cursor::Cursor<'src> for Cursor<'src, '_> {
	type MatchAble = str;
	type Error = ();

	fn input(&self) -> &'src str {
		self.slice
	}
	fn off(&self) -> usize {
		self.off
	}
	fn off_mut(&mut self) -> &mut usize {
		&mut self.off
	}
	fn map_error(&mut self, err: gramex::result::MatchError) -> Self::Error {
		let err = ParseError::from_match_error(&err, self.src, self.end_span, |buf| {
			let char = self.slice.char_at(err.off).unwrap();
			_ = write!(buf, "`{char}`");
			Span::new(err.off, err.off + char.len_utf8())
		});
		self.errors.push(err);
	}
}
impl<'src> Cursor<'src, '_> {
	pub fn new<'err>(
		src: &'src Source<'src>, range: Option<Span>, errors: &'err mut Vec<ParseError>,
	) -> Cursor<'src, 'err> {
		let (slice, off, end_span) = match range {
			Some(range) => {
				(&src.str[..range.end()], range.start(), Span::point(range.end()))
			}
			None => (src.str, 0, Span::EMPTY),
		};
		Cursor { slice, src, off, errors, end_span }
	}
	pub fn try_char(&mut self, char: char) -> bool {
		self.try_eat(char).is_some()
	}
	pub fn test_next(&self, offset: usize, matcher: impl Matcher<str>) -> bool {
		matcher.test(self.slice, &mut (self.off + offset))
	}
	pub fn try_until(&mut self, char: char) -> bool {
		let res = self.slice[self.off..].find(char);
		self.off = res.map_or(self.slice.len(), |i| self.off + i + char.len_utf8());
		res.is_some()
	}
	pub fn try_until_str(&mut self, chars: &str) -> bool {
		let res = self.slice[self.off..].find(chars);
		self.off = res.map_or(self.slice.len(), |i| self.off + i + chars.len());
		res.is_some()
	}
	pub fn eat_ident(&mut self) -> Option<Ident<'src>> {
		let res = self.try_ident();
		if res.is_none() {
			self.expected("an identifier");
		}
		res
	}
	pub fn try_ident(&mut self) -> Option<Ident<'src>> {
		let start = self.off;
		if !matches!(self.peek(), Some('a'..='z' | 'A'..='Z' | '_')) {
			return None;
		}
		let end = self.slice[self.off + 1..]
			.find(|c: char| !matches!(c, 'a'..='z' | 'A'..='Z' | '0'..='9' | '_'))
			.map_or(self.slice.len(), |i| self.off + i);
		self.off = end;
		Some(Ident { val: &self.slice[start..end], span: Span::new(start, end) })
	}
	pub fn expected(&mut self, expected: impl Into<Expected>) {
		_ = <Self as gramex::cursor::Cursor>::expected::<()>(self, expected);
	}
	pub fn error(&mut self, msg: String, span: Span) {
		self.errors.push(ParseError::new(msg, span, self.src));
	}
}
