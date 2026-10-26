use alloc_crate::{borrow::Cow, fmt::Write, format, string::String, vec::Vec};
use gramex::{Matcher, result::Expected, str::hex};
use lean_string::LeanStr;

use crate::{
	parse::{
		ParseError, ParseOptions, Span, str_cursor,
		str_cursor::Cursor as StrCursor,
		tokenize::parse_digits,
		tokens::{
			Block, Source, StrKind, Token, TokenKind, Tokens, angle_close, angle_open,
			brace_close, brace_open, bracket_close, bracket_open, comma, ident,
			paren_close, paren_open, str_tk,
		},
	},
	utils::{StrExt, VecLike},
};
pub use gramex::cursor::Cursor as _;

#[derive(Debug, Clone)]
pub struct Ident {
	pub val: LeanStr,
	pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Str<'src> {
	pub val: Cow<'src, str>,
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
		let err = ParseError::from_match_error(&err, self.src, Span::EMPTY, |buf| {
			let token = self.tokens.0[err.off];
			let found = self.src.source_of(token.span);
			_ = match token.kind {
				TokenKind::StrTk { kind } if found.len() > 32 => {
					let end = if kind == StrKind::Multiline { "\"\"\"" } else { "\"" };
					write!(buf, "`{}...{end}`", &found[0..32],)
				}
				_ => write!(buf, "`{found}`"),
			};
			token.span
		});
		self.errors.push(err);
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
	pub fn try_str_single(&mut self) -> Option<Str<'src>> {
		let Some(Token { kind: TokenKind::StrTk { kind: StrKind::Normal }, span }) =
			self.peek().copied()
		else {
			return None;
		};
		let mut buf = None;
		let mut cur =
			StrCursor::new(self.src, span.start(), self.errors, Span::point(span.end()));
		resolve_escapes(&mut cur, &mut buf);
		match buf {
			Some(buf) => Some(Str { val: Cow::Owned(buf), span }),
			None => Some(Str { val: Cow::Borrowed(self.src.source_of(span)), span }),
		}
	}
	pub fn skip_block(&mut self, block: Block) {
		let Some(close_off) = block.try_open(self) else { return };
		if let Some(close_off) = close_off {
			self.off += close_off as usize;
			return;
		}
		let mut depth = 0;
		while depth > 0 {
			if let Some(closed_off) = block.try_open(self) {
				match closed_off {
					Some(closed_off) => self.off += closed_off as usize,
					None => depth += 1,
				}
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
	fn test_open(self, cur: &mut Cursor) -> bool {
		match self {
			Block::Paren => cur.test(paren_open),
			Block::Brace => cur.test(brace_open),
			Block::Bracket => cur.test(bracket_open),
			Block::Angle => cur.test(angle_open),
		}
	}
	fn try_open(self, cur: &mut Cursor) -> Option<Option<u32>> {
		match self {
			Block::Paren => cur.try_eat(paren_open).map(Token::get_close_off),
			Block::Brace => cur.try_eat(brace_open).map(Token::get_close_off),
			Block::Bracket => cur.try_eat(bracket_open).map(Token::get_close_off),
			Block::Angle => cur.try_eat(angle_open).map(Token::get_close_off),
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

fn parse_unicode_escape(cur: &mut StrCursor, start: usize) -> Result<char, ()> {
	cur.eat('{')?;
	let (dg_count, has_dash) = parse_digits(cur, hex);
	let span = Span::new(start, cur.off);
	cur.eat('}');
	if dg_count > 6 {
		cur.error("unicode escape can only have 1..6 hex digits".into(), span);
		return Err(());
	}
	let code = cur.src.source_of(span);
	let code = if has_dash { &code[..code.len() - 1] } else { code };
	let code = u32::from_str_radix(code, 16).unwrap();
	let Some(code) = char::from_u32(code) else {
		cur.error("invalid unicode escape".into(), span);
		return Err(());
	};
	Ok(code)
}

fn resolve_escapes(cur: &mut StrCursor, buf: &mut Option<String>) {
	let mut last_ind = cur.off;
	while cur.try_until('\\') {
		let start = cur.off;
		let buf = buf.get_or_insert_default();
		buf.push_str(&cur.src.str[last_ind..cur.off - 1]);
		cur.skip();
		let char = match cur.try_eat_token().unwrap() {
			'n' => Some('\n'),
			't' => Some('\t'),
			'r' => Some('\r'),
			'\\' => Some('\\'),
			'"' => Some('"'),
			'u' => parse_unicode_escape(cur, start).ok(),
			char => {
				cur.error(
					format!("invalid escape code `\\{char}`"),
					Span::new(start, cur.off),
				);
				None
			}
		};
		char.inspect(|char| buf.push(*char));
		last_ind = cur.off;
	}
	if let Some(buf) = buf {
		buf.push_str(&cur.src.str[last_ind..cur.off]);
	}
}
