use std::{
	borrow::Cow,
	fmt::Display,
	ops::{Add, AddAssign},
	str::FromStr,
};

use num_bigint::BigInt;
use num_traits::Num;

use crate::{
	errors::{ParseError, err},
	parser::utils::{StrExt, end_of_input, unexpected_token, while_matching},
};

/// line:column position of a token, unicode aware
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Pos {
	pub line: u32,
	pub col: u32,
}

impl Display for Pos {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}:{}", self.line, self.col)
	}
}

impl Pos {
	pub(crate) fn new(line: u32, col: u32) -> Self {
		Self { line, col }
	}
}

impl PartialEq<(u32, u32)> for Pos {
	fn eq(&self, other: &(u32, u32)) -> bool {
		self.line == other.0 && self.col == other.1
	}
	fn ne(&self, other: &(u32, u32)) -> bool {
		self.line != other.0 || self.col != other.1
	}
}

impl Add<u32> for Pos {
	type Output = Self;
	fn add(self, rhs: u32) -> Self::Output {
		Self { line: self.line, col: self.col + rhs }
	}
}
impl AddAssign<u32> for Pos {
	fn add_assign(&mut self, rhs: u32) {
		self.col += rhs
	}
}
impl AddAssign<usize> for Pos {
	fn add_assign(&mut self, rhs: usize) {
		self.col += rhs as u32
	}
}
impl Add<usize> for Pos {
	type Output = Self;
	fn add(self, rhs: usize) -> Self::Output {
		Self { line: self.line, col: self.col + rhs as u32 }
	}
}

/// a lexical unit of the source, with its position
#[derive(Debug)]
pub enum Token<'s> {
	Ident(&'s str, Pos),
	Str(String, Pos),

	Uint(u64, Pos),
	Int(i64, Pos),
	BigInt(BigInt, Pos),
	Float(f64, Pos),

	Symbol(char, Pos),

	/// end of file
	EOF(Pos),
}

impl Token<'_> {
	pub fn pos(&self) -> Pos {
		match self {
			Token::Ident(_, ind) => *ind,
			Token::Str(_, ind) => *ind,
			Token::Uint(_, ind) => *ind,
			Token::Int(_, ind) => *ind,
			Token::BigInt(_, ind) => *ind,
			Token::Float(_, ind) => *ind,
			Token::Symbol(_, ind) => *ind,
			Token::EOF(ind) => *ind,
		}
	}
}

impl Display for Token<'_> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Token::Ident(ident, _) => write!(f, "{ident}"),
			Token::Str(str, _) => write!(f, "\"{str}\""),
			Token::Uint(nb, _) => write!(f, "{nb}"),
			Token::Int(nb, _) => write!(f, "{nb}"),
			Token::BigInt(nb, _) => write!(f, "{nb}"),
			Token::Float(nb, _) => write!(f, "{nb}"),
			Token::Symbol(symbol, _) => write!(f, "{symbol}"),
			Token::EOF(_) => write!(f, "end_of_file"),
		}
	}
}

fn strip_dashes_in_nb<'a>(
	nb: &'a str, start_pos: Pos, file: &str,
) -> Result<Cow<'a, str>, ParseError> {
	let mut cur_ind = 0;
	let mut has_dash = false;
	while let Some(ind) = nb.find_after('_', cur_ind) {
		has_dash = true;

		// ensure prev is digit
		if matches!(nb.char_at(ind - 1), None | Some('_')) {
			return unexpected_token('_', start_pos + ind, file);
		}
		// ensure next is digit
		if matches!(nb.char_at(ind + 1), None | Some('_')) {
			return unexpected_token('_', start_pos + ind, file);
		}

		// skip dash
		cur_ind = ind + 1;
	}
	Ok(if has_dash { Cow::from(nb.replace('_', "")) } else { Cow::from(nb) })
}

/// update the position in content that is find end and skip
///
/// like str and comments
fn update_pos_in_raw(pos: &mut Pos, source: &str) {
	let mut last_ind = 0;
	// count lines
	while let Some(ind) = source.find_after('\n', last_ind) {
		pos.line += 1;
		pos.col = 1;
		last_ind = ind + 1;
	}
	// count chars after last line
	pos.col += source[last_ind..].chars().count() as u32;
}

fn parse_str(source: &str, pos: &mut Pos, file: &str) -> Result<String, ParseError> {
	let mut res = String::new();
	let mut last_ind = 0;

	let invalid_seq =
		|seq, pos: &Pos| err!(format!("invalid escape sequence \"{seq}\""), *pos, file);

	// resolve escape codes
	while let Some(ind) = source.find_after('\\', last_ind) {
		let before = &source[last_ind..ind];
		update_pos_in_raw(pos, before);
		res.push_str(before);

		pos.col += 1; // move to the code
		let (resolved, escape_len) = match source.char_at(ind + 1).unwrap() {
			'0' => ('\0', 2),
			'n' => ('\n', 2),
			'r' => ('\r', 2),
			't' => ('\t', 2),
			'"' => ('"', 2),
			'\\' => ('\\', 2),
			// \xhh
			'x' => {
				let Some(code_hex) = source.get(ind + 2..ind + 4) else {
					return invalid_seq(&source[ind..], pos);
				};
				let Ok(code) = u8::from_str_radix(code_hex, 16) else {
					return invalid_seq(&source[ind..], pos);
				};
				(code as char, 4)
			}
			// \u{..}
			'u' => {
				// extract
				if source.get(ind + 2..ind + 3) != Some("{") {
					return invalid_seq(&source[ind..ind + 1], pos);
				}
				let Some(end_ind) = source.find_after('}', ind + 3) else {
					return invalid_seq(&source[ind..], pos);
				};
				let code_raw = &source[ind + "\\u{".len()..end_ind];
				let code_hex = strip_dashes_in_nb(code_raw, *pos + 3u32, file)?;

				// resolve
				let Ok(code) = u32::from_str_radix(&code_hex, 16) else {
					return invalid_seq(&source[ind..end_ind + 1], pos);
				};
				let Some(code) = char::from_u32(code) else {
					return invalid_seq(&source[ind..end_ind + 1], pos);
				};
				(code, code_raw.len() + "\\u{}".len())
			}
			_ => return invalid_seq(&source[ind..ind + 2], pos),
		};

		res.push(resolved);
		// skip escaped code
		*pos += escape_len;
		last_ind = ind + escape_len;
	}

	let rest = &source[last_ind..];
	update_pos_in_raw(pos, rest);
	res.push_str(rest);

	return Ok(res);
}

fn parse_float<'a>(
	source: &'a str, mut ind: usize, pos: Pos, file: &str,
) -> Result<(Token<'a>, usize, Pos), ParseError> {
	let start_ind = ind;
	matches!(source.char_at(ind), Some('-' | '+')).then(|| ind += 1);

	// spec allow fractional only floats (.5)
	let decimal_end = while_matching(source, ind, |c| matches!(c, '0'..='9' | '_'));
	ind = decimal_end;

	// fractional part
	if source.char_at(ind) == Some('.') {
		ind = while_matching(source, ind + 1, |c| matches!(c, '0'..='9' | '_'));
		if ind == decimal_end + 1 {
			return unexpected_token('.', pos + (decimal_end - start_ind) as u32, file);
		}
	}

	// exponent
	if let Some('e' | 'E') = source.char_at(ind) {
		let e_ind = ind;
		ind += 1;

		matches!(source.char_at(ind), Some('-' | '+')).then(|| ind += 1);
		let exp_start = ind;

		ind = while_matching(source, ind, |c| matches!(c, '0'..='9' | '_'));
		if ind == exp_start {
			return unexpected_token('e', pos + (e_ind - start_ind) as u32, file);
		}
	}

	let nb_source = strip_dashes_in_nb(&source[start_ind..ind], pos, file)?;

	// structom floats are like rust ones
	let Ok(value) = nb_source.parse::<f64>() else {
		return err!(format!("invalid float ({nb_source})"), pos, file);
	};
	let new_pos = pos + (ind - start_ind) as u32;
	Ok((Token::Float(value, pos), ind, new_pos))
}

fn parse_int<'a>(
	source: &'a str, mut ind: usize, pos: Pos, file: &str,
) -> Result<(Token<'a>, usize, Pos), ParseError> {
	let start_ind = ind;
	// match literial
	let sign_char = source.char_at(ind);
	let has_sign = matches!(sign_char, Some('-' | '+'));
	let neg = sign_char == Some('-');
	has_sign.then(|| ind += 1);

	let base = match source.get(ind..ind + 2) {
		Some("0b") => 2,
		Some("0x") => 16,
		None => return end_of_input(file),
		_ => 10,
	};
	(base != 10).then(|| ind += 2);

	let dg_start = ind;
	let end_ind = match base {
		2 => while_matching(source, ind, |c| matches!(c, '0' | '1' | '_')),
		10 => while_matching(source, ind, |c| matches!(c, '0'..='9' | '_')),
		16 => while_matching(source, ind, |c| matches!(c, '0'..='9' | 'a'..='f' | 'A'..='F' | '_')),
		_ => unreachable!(),
	};
	if dg_start == end_ind {
		return unexpected_token(&source.char_at(ind).unwrap(), pos, file);
	}
	let nb_source = strip_dashes_in_nb(&source[dg_start..end_ind], pos, file)?;
	ind = end_ind;

	// float path
	if matches!(source.char_at(ind), Some('.' | 'e' | 'E')) {
		return parse_float(source, start_ind, pos, file);
	}

	let suffix_end =
		while_matching(source, ind, |c| matches!(c, 'a'..='z' | 'A'..='Z' | '0' ..= '9'));
	let suffix = &source[end_ind..suffix_end];
	ind = suffix_end;
	let new_pos = pos + (ind - start_ind) as u32;

	// bigint path
	if suffix == "bint" {
		let Ok(value) = BigInt::from_str_radix(&nb_source, base) else {
			return err!(format!("invalid bigint ({nb_source})"), pos, file);
		};

		return Ok((Token::BigInt(value, pos), ind, new_pos));
	}

	if suffix != "" {
		return err!(format!("invalid suffix \"{suffix}\""), pos, file);
	}

	// int path
	if has_sign {
		// parse without sign
		let Ok(value) = i64::from_str_radix(&nb_source, base) else {
			return err!(format!("integer ({nb_source}) out of range"), pos, file);
		};

		return Ok((Token::Int(value * if neg { -1 } else { 1 }, pos), ind, new_pos));
	}
	// uint path
	else {
		let Ok(value) = u64::from_str_radix(&nb_source, base) else {
			return err!(format!("unsigned integer ({nb_source}) out of range"), pos, file);
		};

		return Ok((Token::Uint(value, pos), ind, new_pos));
	}
}

/// simplify the source into sequence of tokens
pub fn tokenize<'a>(source: &'a str, file: &str) -> Result<Vec<Token<'a>>, ParseError> {
	let mut tokens = Vec::<Token>::new();
	let mut ind: usize = 0;
	let mut pos = Pos::new(1, 1);

	macro_rules! inc {
		($n:expr) => {{
			ind += $n as usize;
			pos.col += $n as u32;
		}};
	}

	while let Some(cur_char) = source.char_at(ind) {
		match cur_char {
			' ' | '\t' | '\r' => inc!(1),
			'\n' => {
				pos.line += 1;
				pos.col = 1;
			}

			'.' => {
				// case fractionless float
				if matches!(source.char_at(ind + 1), Some('0'..='9')) {
					let token;
					(token, ind, pos) = parse_float(source, ind, pos, file)?;
					tokens.push(token);
				} else {
					tokens.push(Token::Symbol('.', pos));
					inc!(1);
				}
			}
			// one char tokens
			',' | ':' | '(' | ')' | '[' | ']' | '{' | '}' | '<' | '>' | '?' | '@' => {
				tokens.push(Token::Symbol(cur_char, pos));
				inc!(1);
			}

			// identifiers
			'a'..='z' | 'A'..='Z' | '_' => {
				let ident_matcher = |c: char| matches!(c, 'a'..='z' | 'A'..='Z' | '_' | '0'..='9');
				let end_ind = while_matching(source, ind, ident_matcher);

				tokens.push(Token::Ident(&source[ind..end_ind], pos));
				pos += end_ind - ind;
				ind = end_ind;
			}

			// strings
			'"' => {
				let start_pos = pos;
				let mut end_index = ind;
				// locate end quote, handling escaped quotes
				loop {
					let Some(ind) = source.find_after('"', end_index + 1) else {
						return end_of_input(file);
					};
					end_index = ind;
					if source.char_at(ind - 1) != Some('\\') {
						break;
					}
				}
				let str = parse_str(&source[ind + 1..end_index], &mut pos, file)?;
				tokens.push(Token::Str(str, start_pos));
				ind = end_index;
				inc!(1); // skip end quote
			}

			// numbers
			'0'..='9' | '-' | '+' => {
				// case + or - identifier like -inf
				if matches!(cur_char, '-' | '+')
					&& !matches!(source.char_at(ind + 1), Some('0'..='9'))
				{
					tokens.push(match cur_char {
						'-' => Token::Symbol('-', pos),
						'+' => Token::Symbol('+', pos),
						_ => unreachable!(),
					});
					inc!(1);
				}

				let token;
				(token, ind, pos) = parse_int(source, ind, pos, file)?;
				tokens.push(token);
			}

			// comments
			'/' => {
				match source.char_at(ind + 1) {
					// single line
					Some('/') => {
						// move to the end of the line or input
						ind = source.find_after('\n', ind + 2).unwrap_or(source.len());
					}
					// multi line
					Some('*') => {
						let Some(end) = source.find_str_after("*/", ind + 2) else {
							return end_of_input(file);
						};
						update_pos_in_raw(&mut pos, &source[ind..end + 2]);
						ind = end + 2;
					}
					Some(char) => return unexpected_token(char, pos, file),
					None => return end_of_input(file),
				}
			}

			_ => return unexpected_token(cur_char, pos, file),
		}
	}
	// make life easier
	tokens.push(Token::EOF(pos));
	Ok(tokens)
}
