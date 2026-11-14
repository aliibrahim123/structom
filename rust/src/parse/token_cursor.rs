use core::{fmt::Display, str::FromStr};

use alloc_crate::{borrow::Cow, fmt::Write, format, string::String, vec::Vec};
use gramex::{Matcher, result::Expected, str::hex};

use crate::{
	parse::{
		ParseError, ParseOptions, Span,
		str_cursor::{Cursor as StrCursor, Ident},
		tokenize::parse_digits,
		tokens::{
			Block, IntBase, Sign, Source, StrKind, Token, TokenKind, Tokens, angle_close,
			angle_open, brace_close, brace_open, bracket_close, bracket_open, comma,
			ident, paren_close, paren_open, unit_comment,
		},
	},
	utils::{IntFromStr, StrExt, VecLike},
};
pub use gramex::cursor::Cursor as _;

#[derive(Debug, Clone)]
pub struct Str<'src> {
	pub val: Cow<'src, str>,
	pub span: Span,
}
impl Display for Str<'_> {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		f.write_str(&self.val)
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
				TokenKind::StrTk { .. } if found.len() > 32 => {
					write!(buf, "a string")
				}
				TokenKind::Bytes { .. } if found.len() > 32 => {
					write!(buf, "a byte string")
				}
				_ => write!(buf, "`{found}`"),
			};
			token.span
		});
		self.errors.push(err);
	}
}
impl<'src> Cursor<'src, '_> {
	pub fn prev(&self) -> Token {
		self.tokens.0[self.off - 1]
	}
	pub fn try_symbol(&mut self, matcher: impl Matcher<Tokens<'src>>) -> bool {
		self.try_eat(matcher).is_some()
	}
	pub fn ident(&mut self) -> Option<Ident<'src>> {
		let span = self.eat(ident).ok()?.span;
		Some(Ident { val: self.src.source_of(span), span })
	}
	pub fn try_ident(&mut self) -> Option<Ident<'src>> {
		let span = self.try_eat(ident)?.span;
		Some(Ident { val: self.src.source_of(span), span })
	}
	pub fn try_ident_kw(&mut self, name: &str) -> bool {
		let matches = self.peek().is_some_and(|token| {
			token.kind == TokenKind::Ident && self.src.source_of(token.span) == name
		});
		matches.then(|| self.skip());
		matches
	}
	pub fn str(&mut self) -> Option<Str<'src>> {
		let Some(&Token { kind: TokenKind::StrTk { kind, unterminated }, span }) =
			self.peek()
		else {
			self.expected("a string");
			return None;
		};
		self.skip();
		match kind {
			StrKind::Normal => Some(resolve_str_normal(self, span, unterminated)),
			StrKind::Multiline => Some(resolve_str_multiline(self, span, unterminated)),
		}
	}
	pub fn enter_str<'cur>(&'cur mut self, str: &'cur Str) -> StrCursor<'cur, 'cur> {
		let Self { src, errors, .. } = self;
		match &str.val {
			Cow::Borrowed(_) => {
				let range = src.str.substr_range(&str.val).unwrap();
				StrCursor::new(src, Some(Span::new(range.start, range.end)), errors)
			}
			Cow::Owned(val) => StrCursor::new_resolved(src, val, str.span, errors),
		}
	}
	pub fn eat_int<T: IntFromStr + Display>(&mut self, kind: &str) -> Option<T> {
		let Some(&Token { kind: TokenKind::Int { base, has_dash, sign }, span }) =
			self.peek()
		else {
			self.expected("an integer");
			return None;
		};
		self.skip();
		let radix = match base {
			IntBase::Dec => 10,
			IntBase::Hex => 16,
			IntBase::Bin => 2,
		};
		let raw = self.src.source_of(span);
		let dg_start = if sign != Sign::None { 1 } else { 0 }
			+ if base != IntBase::Dec { 2 } else { 0 };
		let digits = &raw[dg_start..];
		let digits = if has_dash { &digits.replace('_', "") } else { digits };
		let Some(val) = T::from_str(digits, radix, sign == Sign::Neg) else {
			let (min, max) = T::BOUNDS;
			let msg = format!(
				"number `{raw}` is out of range for {kind}, valid range is `{min}`..`{max}`"
			);
			self.error(msg, span);
			return None;
		};
		Some(val)
	}
	pub fn comma_unless_end(&mut self) {
		if !self.try_symbol(comma) && !self.peek().is_some_and(|t| t.is_block_closer()) {
			self.expected(comma.expected());
		}
	}
	pub fn skip_block(&mut self, block: Block) {
		let Some(close_off) = block.try_open(self) else { return };
		if let Some(close_off) = close_off {
			self.off += close_off as usize;
			return;
		}
		let mut depth = 1;
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
			return V::default();
		}
		self.try_delim_list(open, close, item).unwrap()
	}
	pub fn try_delim_list<T, V: VecLike<T> + Default>(
		&mut self, open: impl Matcher<Tokens<'src>>, close: impl Matcher<Tokens<'src>>,
		mut item: impl FnMut(&mut Self) -> Option<T>,
	) -> Option<V> {
		let mut items = V::default();
		if !self.try_symbol(open) {
			return None;
		}
		while !self.try_symbol(&close) {
			if self.is_end() {
				panic!();
			}
			let skip = self.try_symbol(unit_comment);
			if let Some(item) = item(self) {
				if !skip {
					items.push(item)
				};
				if !(self.try_symbol(comma) || self.test(&close)) {
					self.expected(comma.expected())
				}
			} else {
				self.skip_until(|cur| cur.try_symbol(comma) || cur.test(&close));
			}
		}
		Some(items)
	}
	pub fn eat_delim<T>(
		&mut self, open: impl Matcher<Tokens<'src>>, close: impl Matcher<Tokens<'src>>,
		mut item: impl FnMut(&mut Self) -> Option<T>,
	) -> Option<T> {
		self.eat(open).ok()?;
		let item = item(self);
		if self.eat(&close).is_err() {
			self.skip_until(|cur| cur.try_symbol(&close));
		}
		item
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

fn resolve_str_normal<'src>(
	cur: &mut Cursor<'src, '_>, span: Span, unterminated: bool,
) -> Str<'src> {
	let mut buf = None;
	let end = span.end() - if unterminated { 0 } else { 1 };
	let content_span = Span::new(span.start() + 1, end);
	let mut cur = StrCursor::new(cur.src, Some(content_span), cur.errors);
	resolve_escapes(&mut cur, &mut buf);
	match buf {
		Some(buf) => Str { val: Cow::Owned(buf), span },
		None => Str { val: Cow::Borrowed(cur.src.source_of(content_span)), span },
	}
}

fn resolve_str_multiline<'src>(
	cur: &mut Cursor<'src, '_>, span: Span, unterminated: bool,
) -> Str<'src> {
	let raw = cur.src.source_of(span);
	let body = &raw[3..raw.len() - if unterminated { 0 } else { 3 }];
	let trimmed = body.trim_end_matches([' ', '\t']);
	let indent = &body[trimmed.len()..];
	let trimmed =
		trimmed.trim_start_matches([' ', '\t']).trim_prefix("\n").trim_suffix("\n");

	let mut buf = Some(String::new());
	for (ind, line) in trimmed.split('\n').enumerate() {
		if ind != 0 {
			buf.as_mut().unwrap().push('\n')
		}
		let line = line.strip_prefix(indent).unwrap_or(line);
		let range = cur.src.str.substr_range(line).unwrap();
		let range = Some(Span::new(range.start, range.end));
		let mut cur = StrCursor::new(cur.src, range, cur.errors);
		resolve_escapes(&mut cur, &mut buf);
	}

	Str { val: Cow::Owned(buf.unwrap()), span }
}

fn parse_unicode_escape(cur: &mut StrCursor, start: usize) -> Result<char, ()> {
	cur.eat('{')?;
	let (dg_count, has_dash) = parse_digits(cur, hex);
	let code_end = cur.off;
	_ = cur.eat('}');
	let span = Span::new(start, cur.off);
	if !(1..=6).contains(&dg_count) {
		cur.error("unicode escape should contain 1..6 hex digits".into(), span);
		return Err(());
	}
	let code = cur.src.source_of(Span::new(start + 3, code_end));
	let code = if has_dash { &code.replace('_', "") } else { code };
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
		let start = cur.off - 1;
		let buf = buf.get_or_insert_default();
		buf.push_str(&cur.src.str[last_ind..cur.off - 1]);

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
