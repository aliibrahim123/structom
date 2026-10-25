use alloc_crate::{prelude::v1::format, string::String, vec::Vec};
use gramex::{Matcher, result::Expected};
use lean_string::LeanStr;

use crate::{
	parse::{
		ParseError, ParseOptions, Span,
		tokens::{
			Block, Source, Token, Tokens, angle_close, angle_open, brace_close,
			brace_open, bracket_close, bracket_open, comma, ident, paren_close,
			paren_open,
		},
	},
	utils::VecLike,
};
pub use gramex::cursor::Cursor as _;

#[derive(Debug, Clone)]
pub struct Ident {
	pub val: LeanStr,
	pub span: Span,
}
impl PartialEq<str> for Ident {
	fn eq(&self, other: &str) -> bool {
		self.val == other
	}
}
impl PartialEq<LeanStr> for Ident {
	fn eq(&self, other: &LeanStr) -> bool {
		self.val == *other
	}
}

pub struct Cursor<'src, 'other> {
	pub tokens: &'src Tokens<'src>,
	pub off: usize,
	pub src: &'src Source<'src>,
	pub options: &'other ParseOptions,
	pub errors: &'other mut Vec<ParseError>,
}
impl<'src, 'other> Cursor<'src, 'other> {
	pub fn new(
		tokens: &'src Tokens<'src>, src: &'src Source<'src>,
		options: &'other ParseOptions, errors: &'other mut Vec<ParseError>,
	) -> Self {
		Self { tokens, src, off: 0, options, errors }
	}
}
impl<'src> gramex::cursor::Cursor<'src> for Cursor<'src, '_> {
	type MatchAble = Tokens<'src>;
	type Error = ();
	fn input(&self) -> &'src Self::MatchAble {
		self.tokens
	}
	fn off(&self) -> usize {
		self.off
	}
	fn off_mut(&mut self) -> &mut usize {
		&mut self.off
	}
	fn map_error(&mut self, err: gramex::result::MatchError) -> Self::Error {
		self.errors.push(ParseError::from_match_error(err, self.src));
	}
}
impl<'src> Cursor<'src, '_> {
	pub fn ident(&mut self) -> Result<Ident, ()> {
		let token = self.eat(ident)?;
		Ok(Ident { val: self.src.source_of(token.span).into(), span: token.span })
	}
	pub fn try_ident(&mut self) -> Option<Ident> {
		let token = self.try_eat(ident)?;
		Some(Ident { val: self.src.source_of(token.span).into(), span: token.span })
	}
	pub fn try_symbol(&mut self, matcher: impl Matcher<Tokens<'src>>) -> bool {
		self.try_eat(matcher).is_some()
	}
	pub fn skip_block(&mut self, block: Block) {
		let mut depth = block.try_open(self) as usize;
		while depth > 0 {
			if block.try_open(self) {
				depth += 1;
			} else if block.try_close(self) {
				depth -= 1;
			} else if self.is_end() {
				panic!();
			} else {
				self.skip();
			}
		}
	}
	pub fn skip_until(&mut self, fun: impl Fn(&mut Self) -> bool) {
		while !fun(self) && !self.is_end() {
			if let Some(block) = self.peek().and_then(|token| token.is_block_opener()) {
				self.skip_block(block);
			} else {
				self.skip();
			}
		}
	}
	pub fn eat_delim_list<T, V: VecLike<T> + Default>(
		&mut self, open: impl Matcher<Tokens<'src>>, close: impl Matcher<Tokens<'src>>,
		item: impl FnMut(&mut Self) -> Option<T>,
	) -> V {
		if !self.test(&open) {
			self.expected(open.expected());
		}
		self.try_delim_list(open, close, item)
	}
	pub fn try_delim_list<T, V: VecLike<T> + Default>(
		&mut self, open: impl Matcher<Tokens<'src>>, close: impl Matcher<Tokens<'src>>,
		mut item: impl FnMut(&mut Self) -> Option<T>,
	) -> V {
		let mut items = V::default();
		if !self.try_symbol(open) {
			return items;
		}
		while !self.try_symbol(&close) {
			if self.is_end() {
				panic!();
			}
			if let Some(item) = item(self) {
				items.push(item);
				if !(self.try_symbol(comma) || self.test(&close)) {
					self.expected(comma.expected())
				}
			} else {
				self.skip_until(|cur| cur.try_symbol(comma) || cur.test(&close));
			}
		}
		items
	}
	pub fn expected(&mut self, expected: impl Into<Expected>) {
		_ = <Self as gramex::cursor::Cursor>::expected::<()>(self, expected);
	}
	pub fn error(&mut self, msg: String, span: Span) {
		self.errors.push(ParseError::new(msg, span, self.src));
	}
}
impl Block {
	fn try_open(self, cur: &mut Cursor) -> bool {
		match self {
			Block::Paren => cur.try_symbol(paren_open),
			Block::Brace => cur.try_symbol(brace_open),
			Block::Bracket => cur.try_symbol(bracket_open),
			Block::Angle => cur.try_symbol(angle_open),
		}
	}
	fn try_close(self, cur: &mut Cursor) -> bool {
		match self {
			Block::Paren => cur.try_symbol(paren_close),
			Block::Brace => cur.try_symbol(brace_close),
			Block::Bracket => cur.try_symbol(bracket_close),
			Block::Angle => cur.try_symbol(angle_close),
		}
	}
}
