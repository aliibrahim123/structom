use core::mem::replace;

use alloc_crate::{format, vec::Vec};
use gramex::{
	Matcher,
	str::{bin, dec, hex},
};

use crate::{
	parse::{
		error::ParseError,
		str_cursor::{self, *},
		tokens::{
			Block, FloatKind, IntBase, Sign, Source, Span, StrKind, Token, TokenKind::*,
		},
	},
	utils::StrExt,
};

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
				let body = &cur.slice[last_stop..cur.off() - 2];
				depth = depth + body.matches("/*").count() - 1;
			} else {
				cur.error(
					"unterminated block comment".into(),
					Span::new(start, cur.slice.len()),
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

	let Some((_, open_ind)) = block_stack.pop() else { unreachable!() };
	let close_off = tokens.len() - open_ind;
	tokens[open_ind].set_close_off(close_off);
	tokens.push(Token { kind: block.end(), span });
}

fn skip_underscores(cur: &mut Cursor, dg: impl Matcher<str> + Copy) {
	while cur.test('_') {
		cur.expected(dg.expected());
		cur.skip();
	}
}

pub fn parse_digits(cur: &mut Cursor, dg: impl Matcher<str> + Copy) -> (u32, bool) {
	let mut dg_count = if cur.eat(dg).is_ok() { 1 } else { 0 };
	let mut has_dash = cur.try_char('_');
	skip_underscores(cur, dg);
	while cur.try_eat(dg).is_some() {
		dg_count += 1;
		has_dash |= cur.try_char('_');
		skip_underscores(cur, dg);
	}
	(dg_count, has_dash)
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

	if has_dg == 0 {
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
	} else if cur.try_ident().is_some_and(|ident| ident == "inf") {
		let kind = if char == '+' { FloatKind::PosInf } else { FloatKind::NegInf };
		tokens.push(Token {
			kind: Float { kind, has_dash: false },
			span: Span::new(start, cur.off()),
		})
	} else {
		cur.off = start + 1;
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
			cur.error("unterminated string".into(), Span::new(start, cur.slice.len()));
			break;
		}
		let pre_offsets = (last_quote..cur.off() - quotes.len()).rev();
		let slashes =
			pre_offsets.take_while(|&i| cur.slice.char_at(i) == Some('\\')).count();
		if slashes % 2 == 0 {
			break;
		}
	}

	tokens.push(Token { kind: StrTk { kind }, span: Span::new(start, cur.off()) });
}

fn parse_ident(cur: &mut Cursor, tokens: &mut Vec<Token>) {
	cur.off -= 1;
	let str_cursor::Ident { val: ident, span } = cur.try_ident().unwrap();
	match ident {
		"true" => tokens.push(Token { kind: KwTrue, span }),
		"false" => tokens.push(Token { kind: KwFalse, span }),
		"struct" => tokens.push(Token { kind: KwStruct, span }),
		"union" => tokens.push(Token { kind: KwUnion, span }),
		"none" => tokens.push(Token { kind: KwNone, span }),
		"nan" => tokens
			.push(Token { kind: Float { kind: FloatKind::Nan, has_dash: false }, span }),
		"inf" => tokens.push(Token {
			kind: Float { kind: FloatKind::PosInf, has_dash: false },
			span,
		}),
		_ => tokens.push(Token { kind: Ident, span }),
	}
}

pub fn tokenize(
	src: &mut Source, errors: &mut Vec<ParseError>,
) -> Result<Vec<Token>, ()> {
	if cfg!(not(feature = "large-span")) && src.str.len() >= u32::MAX as usize {
		let msg = "file too large, turn on `large-span` feature".into();
		errors.push(ParseError::new(msg, Span::EMPTY, src));
		return Err(());
	}

	let mut tokens = Vec::new();
	let cur = &mut Cursor::new(src, None, errors);
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

			'a'..='z' | 'A'..='Z' | '_' => parse_ident(cur, &mut tokens),

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
				tokens.push(Token { kind: ParenOpen { close_off: None }, span });
			}
			'[' => {
				block_stack.push((Block::Bracket, tokens.len()));
				tokens.push(Token { kind: BracketOpen { close_off: None }, span });
			}
			'{' => {
				block_stack.push((Block::Brace, tokens.len()));
				tokens.push(Token { kind: BraceOpen { close_off: None }, span });
			}
			'<' => {
				block_stack.push((Block::Angle, tokens.len()));
				tokens.push(Token { kind: AngleOpen { close_off: None }, span });
			}
			')' => close_delim(Block::Paren, cur),
			']' => close_delim(Block::Bracket, cur),
			'}' => close_delim(Block::Brace, cur),
			'>' => close_delim(Block::Angle, cur),

			'/' => parse_comment(cur, &mut tokens, start),

			_ => cur.error(format!("invalid token `{char}`"), span),
		}
	}

	for (block, open_ind) in block_stack.iter().rev() {
		let close_off = tokens.len() - open_ind;
		let open = &mut tokens[*open_ind];
		open.set_close_off(close_off);
		cur.error(format!("unclosed `{}`", block.delims().0), open.span);
		tokens.push(Token { kind: block.end(), span: Span::EMPTY });
	}
	if filter_dummy {
		tokens.retain(|token| *token != Token::DUMMY)
	}
	Ok(tokens)
}
