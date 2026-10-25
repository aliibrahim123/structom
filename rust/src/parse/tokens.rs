use core::cell::OnceCell;

use alloc_crate::vec::Vec;
use gramex::{
	Matcher, derive::derive_enum_matcher, derive_slice_matchable, result::Expected,
};

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

#[derive_enum_matcher(for Tokens<'slice>, field = .kind, expected = tk_kind_to_expected)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
	Ident,
	StrTk { kind: StrKind },
	Int { sign: Sign, base: IntBase, has_dash: bool },
	Float { kind: FloatKind, has_dash: bool },

	KwTrue,
	KwFalse,
	KwStruct,
	KwUnion,
	KwNone,

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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Token {
	pub kind: TokenKind,
	pub span: Span,
}
impl Token {
	pub const DUMMY: Token = Token { kind: TokenKind::Dummy, span: Span::EMPTY };
	pub fn is_block_opener(&self) -> Option<Block> {
		match self.kind {
			TokenKind::ParenOpen => Some(Block::Paren),
			TokenKind::BraceOpen => Some(Block::Brace),
			TokenKind::BracketOpen => Some(Block::Bracket),
			TokenKind::AngleOpen => Some(Block::Angle),
			_ => None,
		}
	}
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Tokens<'a>(&'a [Token]);
derive_slice_matchable!(Tokens(&[Token]), matchers: true, expected: Debug);

fn tk_kind_to_expected(kind: &'static str) -> Expected {
	Expected::A(
		match kind {
			"Ident" => "an identifier",
			"StrTk" => "a string",
			"Int" => "an integer",
			"Float" => "a float",

			"KwFalse" => "`false`",
			"KwTrue" => "`true`",
			"KwStruct" => "`struct`",
			"KwUnion" => "`union`",
			"KwNone" => "`none`",

			"BraceOpen" => "`{`",
			"BracketOpen" => "`[`",
			"ParenOpen" => "`(`",
			"AngleOpen" => "`<`",

			"BraceClose" => "`}`",
			"BracketClose" => "`]`",
			"ParenClose" => "`)`",
			"AngleClose" => "`>`",

			"Comma" => "`,`",
			"Colon" => "`:`",
			"Dot" => "`.`",
			"Question" => "`?`",
			"Equal" => "`=`",
			"Hash" => "`#`",
			"Alt" => "`@`",
			"Bang" => "`!`",

			"UnitComment" => "`/-`",
			_ => unreachable!(),
		}
		.into(),
	)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Block {
	Brace,
	Bracket,
	Paren,
	Angle,
}
impl Block {
	pub fn delims(&self) -> (char, char) {
		match self {
			Block::Brace => ('{', '}'),
			Block::Bracket => ('[', ']'),
			Block::Paren => ('(', ')'),
			Block::Angle => ('<', '>'),
		}
	}
	pub fn end(&self) -> TokenKind {
		match self {
			Block::Brace => TokenKind::BraceClose,
			Block::Bracket => TokenKind::BracketClose,
			Block::Paren => TokenKind::ParenClose,
			Block::Angle => TokenKind::AngleClose,
		}
	}
}
