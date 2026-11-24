use core::fmt::Display;

use alloc_crate::{borrow::Cow, fmt::Write, format, string::String, vec::Vec};
use gramex::{
	MatchAble, Matcher, Mode,
	result::{Expected, MatchError, MatchResult},
	str::hex,
};

use crate::{
	parse::{
		ParseError, ParseOptions, Span,
		str_cursor::{Cursor as StrCursor, Ident},
		tokenize::parse_digits,
		tokens::{
			Block, IntBase, Sign, Source, StrKind, Token, TokenKind, TokenTag, Tokens,
		},
	},
	utils::{IntFromStr, VecLike},
};
pub use TokenTag::*;
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
		let span = self.eat(Ident).ok()?.span;
		Some(Ident { val: self.src.source_of(span), span })
	}
	pub fn try_ident(&mut self) -> Option<Ident<'src>> {
		let span = self.try_eat(Ident)?.span;
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
	pub fn try_int<T: IntFromStr + Display>(&mut self) -> Option<(T, Span)> {
		self.try_eat(Int)
			.and_then(|token| parse_int(token, self.src.source_of(token.span)))
	}
	pub fn int<T: IntFromStr + Display>(&mut self, kind: &str) -> Option<(T, Span)> {
		self.eat(Int).ok().and_then(|token| {
			match parse_int(token, self.src.source_of(token.span)) {
				Some(res) => Some(res),
				None => {
					let raw = self.src.source_of(token.span);
					let (min, max) = T::BOUNDS;
					let msg = format!(
						"number `{raw}` is out of range for {kind}, valid range is `{min}`..`{max}`"
					);
					self.error(msg, token.span);
					None
				}
			}
		})
	}
	pub fn comma_unless_end(&mut self) {
		if !self.try_symbol(Comma) && !self.peek().is_some_and(|t| t.is_block_closer()) {
			self.expected(Comma.expected());
		}
	}
	pub fn skip_block(&mut self, block: Block) -> bool {
		let (open, close) = block.delim_tags();
		let Some(close_off) = self.try_eat(open).map(Token::get_close_off) else {
			return false;
		};
		if let Some(close_off) = close_off {
			self.off += close_off as usize;
			return true;
		}
		let mut depth = 1;
		while depth > 0 {
			if let Some(closed_off) = self.try_eat(open).map(Token::get_close_off) {
				match closed_off {
					Some(closed_off) => self.off += closed_off as usize,
					None => depth += 1,
				}
			} else if self.try_symbol(close) {
				depth -= 1;
			} else if self.is_end() {
				panic!();
			} else {
				self.skip();
			}
		}
		true
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
	pub fn delim_list<T, V: VecLike<T> + Default>(
		&mut self, block: Block, item: impl FnMut(&mut Self, bool) -> Option<T>,
	) -> V {
		let (open, _) = block.delim_tags();
		if !self.test(open) {
			self.expected(open.expected());
			return V::default();
		}
		self.try_delim_list(block, item).unwrap()
	}
	pub fn try_delim_list<T, V: VecLike<T> + Default>(
		&mut self, block: Block, mut item: impl FnMut(&mut Self, bool) -> Option<T>,
	) -> Option<V> {
		let (open, close) = block.delim_tags();
		let mut items = V::default();
		if !self.try_symbol(open) {
			return None;
		}
		while !self.try_symbol(close) {
			if self.is_end() {
				panic!();
			}
			let skip = self.try_symbol(UnitComment);
			if let Some(item) = item(self, skip) {
				if !skip {
					items.push(item)
				};
				if !(self.try_symbol(Comma) || self.test(close)) {
					self.expected(Comma.expected())
				}
			} else {
				self.skip_until(|cur| cur.try_symbol(Comma) || cur.test(close));
			}
		}
		Some(items)
	}
	pub fn try_delim<T>(
		&mut self, block: Block, mut item: impl FnMut(&mut Self) -> Option<T>,
	) -> Option<T> {
		let (open, close) = block.delim_tags();
		self.try_eat(open)?;
		let item = item(self);
		if self.eat(close).is_err() {
			self.skip_until(|cur| cur.try_symbol(close));
		}
		item
	}
	pub fn delim<T>(
		&mut self, block: Block, mut item: impl FnMut(&mut Self) -> Option<T>,
	) -> Option<T> {
		let (open, close) = block.delim_tags();
		self.eat(open).ok()?;
		let item = item(self);
		if self.eat(close).is_err() {
			self.skip_until(|cur| cur.try_symbol(close));
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

impl<'slice> Matcher<Tokens<'slice>> for TokenTag {
	type Capture<'src>
		= Token
	where
		Tokens<'slice>: 'src;
	fn do_match<'src, M: Mode>(
		&self, matched: &'src Tokens<'slice>, off: &mut usize,
	) -> MatchResult<Self::Capture<'src>, M> {
		if let Some(&token) = matched.0.get(*off).filter(|v| v.kind.tag() == *self) {
			*off += 1;
			Ok(M::wrap_success(token))
		} else {
			M::err(|| MatchError::expected(self.expected(), *off == matched.len(), *off))
		}
	}
	fn expected(&self) -> Expected {
		Expected::from(match self {
			TokenTag::Ident => "an identifier",
			TokenTag::StrTk { .. } => "a string",
			TokenTag::Bytes { .. } => "a byte string",
			TokenTag::Int { .. } => "an integer",
			TokenTag::Float { .. } => "a float",

			TokenTag::KwFalse => "`false`",
			TokenTag::KwTrue => "`true`",
			TokenTag::KwStruct => "`struct`",
			TokenTag::KwUnion => "`union`",
			TokenTag::KwNone => "`none`",

			TokenTag::BraceOpen { .. } => "`{`",
			TokenTag::BracketOpen { .. } => "`[`",
			TokenTag::ParenOpen { .. } => "`(`",
			TokenTag::AngleOpen { .. } => "`<`",

			TokenTag::BraceClose => "`}`",
			TokenTag::BracketClose => "`]`",
			TokenTag::ParenClose => "`)`",
			TokenTag::AngleClose => "`>`",

			TokenTag::Comma => "`,`",
			TokenTag::Colon => "`:`",
			TokenTag::Dot => "`.`",
			TokenTag::Question => "`?`",
			TokenTag::Equal => "`=`",
			TokenTag::Hash => "`#`",
			TokenTag::At => "`@`",
			TokenTag::Bang => "`!`",

			TokenTag::UnitComment => "`/-`",
			TokenTag::Dummy => "",
		})
	}
}

fn parse_int<T: IntFromStr + Display>(token: Token, raw: &str) -> Option<(T, Span)> {
	let Token { kind: TokenKind::Int { base, has_dash, sign }, span } = token else {
		unreachable!()
	};
	let radix = match base {
		IntBase::Dec => 10,
		IntBase::Hex => 16,
		IntBase::Bin => 2,
	};

	let dg_start =
		if sign != Sign::None { 1 } else { 0 } + if base != IntBase::Dec { 2 } else { 0 };
	let digits = &raw[dg_start..];
	let digits = if has_dash { &digits.replace('_', "") } else { digits };
	T::from_str(digits, radix, sign == Sign::Neg).map(|nb| (nb, span))
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
