use core::{
	cell::OnceCell,
	fmt::Display,
	mem::{replace, swap},
};

use alloc_crate::{fmt::Write, format, string::String, vec::Vec};
use gramex::{
	Matcher,
	cursor::Cursor as _,
	result::{Expected, MatchErrorKind},
	str::{bin, dec, hex},
};

use crate::{parse::error::Error, utils::StrExt};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Source<'a> {
	pub str: &'a str,
	pub path: &'a str,
	pub line_poses: OnceCell<Vec<usize>>,
}
impl Source<'_> {
	pub fn new<'a>(str: &'a str, path: &'a str) -> Source<'a> {
		Source { str, path, line_poses: OnceCell::new() }
	}
	pub fn source_of(&self, span: Span) -> &str {
		&self.str[span.start()..span.end()]
	}
	fn init_line_poses(&self) -> Vec<usize> {
		self.str.match_indices('\n').map(|(ind, _)| ind).collect()
	}
	pub fn pos_of(&self, ind: usize) -> (usize, usize) {
		let Source { str: src, line_poses, .. } = self;
		let line_poses = line_poses.get_or_init(|| self.init_line_poses());
		let nl_before = line_poses.partition_point(|&p| p < ind);
		if nl_before == 0 {
			(1, src[..ind].chars().count() + 1)
		} else {
			(nl_before + 1, src[line_poses[nl_before - 1]..ind].chars().count())
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Hash)]
pub struct Span {
	#[cfg(not(feature = "large-span"))]
	pub(crate) start: u32,
	#[cfg(not(feature = "large-span"))]
	pub(crate) end: u32,
	#[cfg(feature = "large-span")]
	pub(crate) start: u64,
	#[cfg(feature = "large-span")]
	pub(crate) end: u64,
}

impl Span {
	pub const EMPTY: Span = Span { start: 0, end: 0 };
	pub(crate) fn new(start: usize, end: usize) -> Self {
		Self { start: start as _, end: end as _ }
	}
	pub fn start(&self) -> usize {
		self.start as usize
	}
	pub fn end(&self) -> usize {
		self.end as usize
	}
	pub fn is_point(&self) -> bool {
		self.start + 1 == self.end
	}
	pub fn point(pos: usize) -> Self {
		Self::new(pos, pos + 1)
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntBase {
	Dec,
	Bin,
	Hex,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sign {
	None,
	Pos,
	Neg,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FloatKind {
	Normal,
	PosInf,
	NegInf,
	Nan,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StrKind {
	Normal,
	Multiline,
	Bytes,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Block {
	Brace,
	Bracket,
	Paren,
	Angle,
}
impl Block {
	fn delims(&self) -> (char, char) {
		match self {
			Block::Brace => ('{', '}'),
			Block::Bracket => ('[', ']'),
			Block::Paren => ('(', ')'),
			Block::Angle => ('<', '>'),
		}
	}
	fn end(&self) -> TokenKind {
		match self {
			Block::Brace => BraceClose,
			Block::Bracket => BracketClose,
			Block::Paren => ParenClose,
			Block::Angle => AngleClose,
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
	Ident,
	Str { kind: StrKind },
	Int { sign: Sign, base: IntBase, has_dash: bool },
	Float { kind: FloatKind, has_dash: bool },

	True,
	False,
	Struct,
	Union,
	None,

	BraceOpen,
	BracketOpen,
	ParenOpen,
	AngleOpen,

	BraceClose,
	BracketClose,
	ParenClose,
	AngleClose,

	Comma,
	Colon,
	Dot,
	Question,
	Equal,
	Hash,
	Alt,
	Bang,

	UnitComment,

	Dummy,
	Eof,
}
use TokenKind::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Token {
	pub kind: TokenKind,
	pub span: Span,
}
impl Token {
	const DUMMY: Token = Token { kind: Dummy, span: Span::EMPTY };
}

#[derive(Debug)]
struct Cursor<'src, 'err> {
	src: &'src Source<'src>,
	off: usize,
	errors: &'err mut Vec<Error>,
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
		let (msg, span) = match err.kind {
			MatchErrorKind::MisMatch(expected) => {
				let char = self.src.str.char_at(err.off).unwrap();
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
		self.errors.push(Error::new(msg, span, self.src));
	}
}
impl<'a> Cursor<'a, '_> {
	fn try_char(&mut self, char: char) -> bool {
		self.try_eat(char).is_some()
	}
	fn test_next(&self, offset: usize, matcher: impl Matcher<str>) -> bool {
		matcher.test(self.src.str, &mut (self.off + offset))
	}
	fn try_until(&mut self, char: char) -> bool {
		let res = self.src.str[self.off..].find(char);
		self.off = res.map_or(self.src.str.len(), |i| self.off + i + char.len_utf8());
		res.is_some()
	}
	fn try_until_str(&mut self, chars: &str) -> bool {
		let res = self.src.str[self.off..].find(chars);
		self.off = res.map_or(self.src.str.len(), |i| self.off + i + chars.len());
		res.is_some()
	}
	fn try_eat_ident(&mut self) -> Option<&'a str> {
		let len = self.src.str[self.off..]
			.find(|c: char| !matches!(c, 'a'..='z' | 'A'..='Z' | '0'..='9' | '_'))
			.map_or(self.src.str.len(), |i| self.off + i);
		(len > 0).then(|| &self.src.str[self.off..len])
	}
	fn expected(&mut self, expected: impl Into<Expected>) {
		_ = <Self as gramex::cursor::Cursor>::expected::<()>(self, expected);
	}
	fn error(&mut self, msg: String, span: Span) {
		self.errors.push(Error::new(msg, span, self.src));
	}
}

fn parse_comment(cur: &mut Cursor, tokens: &mut Vec<Token>, start: usize) {
	if cur.try_char('-') {
		tokens.push(Token { kind: UnitComment, span: Span::new(start, cur.off()) });
	} else if cur.try_char('/') {
		cur.try_until('\n');
	} else if cur.try_char('*') {
		let mut depth = 1;
		while depth > 0 {
			let last_stop = cur.off();
			if cur.try_until_str("*/") {
				let body = &cur.src.str[last_stop..cur.off() - 2];
				depth = depth + body.matches("/*").count() - 1;
			} else {
				cur.error(
					"unterminated block comment".into(),
					Span::new(start, cur.src.str.len()),
				);
				break;
			}
		}
	} else {
		cur.expected(["`/`", "`*`", "`-`"]);
	}
}

fn close_delim(
	block: Block, pos: usize, block_stack: &mut Vec<(Block, usize)>,
	tokens: &mut Vec<Token>, filter_dummy: &mut bool, cur: &mut Cursor,
) {
	let span = Span::point(pos);
	let Some(unclosed) = (0..=2).find(|n| {
		block_stack.len() > *n && block_stack[block_stack.len() - *n - 1].0 == block
	}) else {
		let (open, close) = block.delims();
		cur.error(format!("unexpected `{close}`, no matching `{open}`"), span);
		return;
	};

	*filter_dummy |= unclosed > 0;
	for _ in 1..=unclosed {
		let (block, ind) = block_stack.pop().unwrap();
		let span = replace(&mut tokens[ind], Token::DUMMY).span;
		cur.error(format!("unclosed `{}`", block.delims().0), span);
	}

	block_stack.pop();
	tokens.push(Token { kind: block.end(), span });
}

fn skip_underscores(cur: &mut Cursor, dg: impl Matcher<str> + Copy) {
	while cur.test('_') {
		cur.expected(dg.expected());
		cur.skip();
	}
}

fn parse_digits(cur: &mut Cursor, dg: impl Matcher<str> + Copy) -> (bool, bool) {
	let mut has_dg = cur.eat(dg).is_ok();
	let mut has_dash = cur.try_char('_');
	skip_underscores(cur, dg);
	while cur.try_eat(dg).is_some() {
		has_dg = true;
		has_dash |= cur.try_char('_');
		skip_underscores(cur, dg);
	}
	(has_dg, has_dash)
}
fn parse_int(cur: &mut Cursor, tokens: &mut Vec<Token>, sign: Sign, start: usize) {
	if sign == Sign::None {
		cur.off -= 1;
	}
	let (base, has_dg, has_dash) = if cur.try_eat("0x").is_some() {
		if cur.try_char('"') {
			return parse_str(cur, StrKind::Bytes, tokens, start);
		}
		let (has_dg, has_dash) = parse_digits(cur, hex);
		(IntBase::Hex, has_dg, has_dash)
	} else if cur.try_eat("0b").is_some() {
		let (has_dg, has_dash) = parse_digits(cur, bin);
		(IntBase::Bin, has_dg, has_dash)
	} else {
		let (has_dg, has_dash) = parse_digits(cur, dec);
		(IntBase::Dec, has_dg, has_dash)
	};

	if !has_dg {
		return;
	}

	if base == IntBase::Dec && (cur.test('.') || cur.test('e') || cur.test('E')) {
		return parse_float(cur, tokens, has_dash, start);
	}

	tokens.push(Token {
		kind: Int { base, sign, has_dash },
		span: Span::new(start, cur.off()),
	});
}

fn parse_float(
	cur: &mut Cursor, tokens: &mut Vec<Token>, mut has_dash: bool, start: usize,
) {
	if cur.try_char('.') {
		has_dash |= parse_digits(cur, dec).1;
	}
	if cur.try_char('e') || cur.try_char('E') {
		_ = cur.try_char('+') || cur.try_char('-');
		has_dash |= parse_digits(cur, dec).1;
	}

	tokens.push(Token {
		kind: Float { kind: FloatKind::Normal, has_dash },
		span: Span::new(start, cur.off()),
	});
}

fn handle_sign(cur: &mut Cursor, char: char, tokens: &mut Vec<Token>, start: usize) {
	if cur.test(dec) && !cur.test("0x\"") {
		let sign = if char == '+' { Sign::Pos } else { Sign::Neg };
		parse_int(cur, tokens, sign, start);
	} else if cur.test('.') && cur.test_next(1, dec) {
		parse_float(cur, tokens, false, start);
	} else if cur.try_eat_ident() == Some("inf") {
		cur.off += 3;
		let kind = if char == '+' { FloatKind::PosInf } else { FloatKind::NegInf };
		tokens.push(Token {
			kind: Float { kind, has_dash: false },
			span: Span::new(start, cur.off()),
		})
	} else {
		cur.error(
			format!("expected one of `.`, `inf` or a decimal digit after `{char}`",),
			Span::point(start),
		);
	}
}

fn parse_str(cur: &mut Cursor, kind: StrKind, tokens: &mut Vec<Token>, start: usize) {
	let quotes = match kind {
		StrKind::Multiline => "\"\"\"",
		StrKind::Normal | StrKind::Bytes => "\"",
	};
	loop {
		let last_quote = cur.off;
		if !cur.try_until_str(quotes) {
			cur.error("unterminated string".into(), Span::new(start, cur.src.str.len()));
			break;
		}
		let pre_offsets = (last_quote..cur.off() - quotes.len()).rev();
		let slashes =
			pre_offsets.take_while(|&i| cur.src.str.char_at(i) == Some('\\')).count();
		if slashes % 2 == 0 {
			break;
		}
	}

	tokens.push(Token { kind: Str { kind }, span: Span::new(start, cur.off()) });
}

pub fn tokenize(source: &mut Source, errors: &mut Vec<Error>) -> Result<Vec<Token>, ()> {
	if cfg!(not(feature = "large-span")) && source.str.len() >= u32::MAX as usize {
		let msg = "file too large, turn on `large-span` feature".into();
		errors.push(Error::new(msg, Span::EMPTY, source));
		return Err(());
	}

	let mut tokens = Vec::new();
	let cur = &mut Cursor { src: source, off: 0, errors };
	let mut filter_dummy = false;
	let block_stack = &mut Vec::new();

	cur.try_eat('\u{FEFF}');

	while let Some(char) = cur.try_eat_token() {
		let start = cur.off() - char.len_utf8();
		let span = Span::new(start, cur.off);
		let mut close_delim = |block, cur| {
			close_delim(block, start, block_stack, &mut tokens, &mut filter_dummy, cur)
		};
		match char {
			' ' | '\t' | '\r' | '\n' => {}

			',' => tokens.push(Token { kind: Comma, span }),
			':' => tokens.push(Token { kind: Colon, span }),
			'.' if !cur.test(dec) => tokens.push(Token { kind: Dot, span }),
			'?' => tokens.push(Token { kind: Question, span }),
			'=' => tokens.push(Token { kind: Equal, span }),
			'#' => tokens.push(Token { kind: Hash, span }),
			'@' => tokens.push(Token { kind: Alt, span }),
			'!' => tokens.push(Token { kind: Bang, span }),

			'a'..='z' | 'A'..='Z' | '_' => {
				cur.off -= 1;
				let ident = cur.try_eat_ident().unwrap();
				cur.off += ident.len();
				let span = Span::new(start, cur.off());
				match ident {
					"true" => tokens.push(Token { kind: True, span }),
					"false" => tokens.push(Token { kind: False, span }),
					"struct" => tokens.push(Token { kind: Struct, span }),
					"union" => tokens.push(Token { kind: Union, span }),
					"none" => tokens.push(Token { kind: None, span }),
					"nan" => tokens.push(Token {
						kind: Float { kind: FloatKind::Nan, has_dash: false },
						span,
					}),
					"inf" => tokens.push(Token {
						kind: Float { kind: FloatKind::PosInf, has_dash: false },
						span,
					}),
					_ => tokens.push(Token { kind: Ident, span }),
				}
			}

			'+' | '-' => handle_sign(cur, char, &mut tokens, start),
			'0'..='9' => parse_int(cur, &mut tokens, Sign::None, start),
			'.' => {
				cur.off -= 1;
				parse_float(cur, &mut tokens, false, start);
			}

			'"' => match cur.try_eat("\"\"").is_some() {
				true => parse_str(cur, StrKind::Multiline, &mut tokens, start),
				false => parse_str(cur, StrKind::Normal, &mut tokens, start),
			},

			'(' => {
				block_stack.push((Block::Paren, tokens.len()));
				tokens.push(Token { kind: ParenOpen, span });
			}
			'[' => {
				block_stack.push((Block::Bracket, tokens.len()));
				tokens.push(Token { kind: BracketOpen, span });
			}
			'{' => {
				block_stack.push((Block::Brace, tokens.len()));
				tokens.push(Token { kind: BraceOpen, span });
			}
			'<' => {
				block_stack.push((Block::Angle, tokens.len()));
				tokens.push(Token { kind: AngleOpen, span });
			}
			')' => close_delim(Block::Paren, cur),
			']' => close_delim(Block::Bracket, cur),
			'}' => close_delim(Block::Brace, cur),
			'>' => close_delim(Block::Angle, cur),

			'/' => parse_comment(cur, &mut tokens, start),

			_ => cur.error(format!("unexpected token `{char}`"), span),
		}
	}

	for (block, ind) in block_stack.iter().rev() {
		cur.error(format!("unclosed `{}`", block.delims().0), tokens[*ind].span);
		tokens.push(Token { kind: block.end(), span: Span::EMPTY });
	}
	if filter_dummy {
		tokens.retain(|token| *token != Token::DUMMY)
	}
	tokens.push(Token { kind: Eof, span: Span::EMPTY });
	Ok(tokens)
}
