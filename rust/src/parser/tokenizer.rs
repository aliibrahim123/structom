use std::fmt::Display;

use crate::{
	errors::{ParserError, end_of_input, unexpected_token},
	parser::utils::{get_char, while_matching},
};

/// a well defined part of the source
///
/// has a value and a start index
#[derive(Debug)]
pub enum Token<'s> {
	Identifier(&'s str, usize),
	Str(String, usize),

	Uint(u64, usize),
	Int(i64, usize),
	BigInt((), usize),
	Float(f64, usize),

	Symbol(char, usize),

	EOF(usize),
}

impl Token<'_> {
	pub fn ind(&self) -> usize {
		match self {
			Token::Identifier(_, ind) => *ind,
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
			Token::Identifier(ident, _) => write!(f, "{}", ident),
			Token::Str(str, _) => write!(f, "\"{}\"", str),
			Token::Uint(nb, _) => write!(f, "nb({})", nb),
			Token::Int(nb, _) => write!(f, "nb({})", nb),
			Token::BigInt(_, _) => write!(f, "nb({})", 0),
			Token::Float(nb, _) => write!(f, "nb({})", nb),
			Token::Symbol(symbol, _) => write!(f, "{}", symbol),
			Token::EOF(_) => write!(f, "end_of_file"),
		}
	}
}

enum NbValueType {
	Int,
	Uint,
	BigInt,
}

/// remove dashes "_" in number
fn parse_dashes_in_nb(nb: &str, start_ind: usize) -> Result<String, ParserError> {
	// loop through dashes
	let mut cur_ind = 0;
	if nb.starts_with("_") {
		return Err(unexpected_token('_', start_ind));
	};
	while let Some(ind) = nb[cur_ind..].find('_') {
		cur_ind += ind;

		// if prev not digit or start of input
		if get_char(nb, cur_ind - 1).is_none_or(|c| !matches!(c, '0'..='9')) {
			return Err(unexpected_token('_', start_ind + cur_ind));
		}
		// if next not digit or end of input
		if get_char(nb, cur_ind + 1).is_none_or(|c| !matches!(c, '0'..='9')) {
			return Err(unexpected_token('_', start_ind + cur_ind));
		}
		cur_ind += 1;
	}
	Ok(nb.replace('_', ""))
}

/// resolve all escape sequences in a string
fn parse_escape_sequences(source: &str, start_ind: usize) -> Result<String, ParserError> {
	let mut res = String::new();
	let mut last_ind = 0;

	let invalid_seq = |seq: &str, ind| {
		ParserError::SyntaxError(format!("invalid escape sequence `{seq}` at {}", start_ind + ind))
	};

	// loop through escape sequences
	while let Some(i) = source[last_ind..].find('\\') {
		let ind = last_ind + i;
		// push before escape
		res.push_str(&source[last_ind..ind]);

		let (resolved, escape_len) = match get_char(source, ind + 1).unwrap() {
			'0' => ('\0', 2),
			'n' => ('\n', 2),
			'r' => ('\r', 2),
			't' => ('\t', 2),
			'"' => ('"', 2),
			'\\' => ('\\', 2),
			// \xhh
			'x' => {
				// extract code
				let code_str =
					source.get(ind + 2..ind + 4).ok_or_else(|| invalid_seq(&source[ind..], ind))?;

				// hex -> char
				let code = u8::from_str_radix(code_str, 16)
					.map_err(|_| invalid_seq(&source[ind..ind + 4], ind))? as char;

				(code, 4)
			}
			'u' => {
				// extract code
				let end_ind = source[ind + 3..]
					.find('}')
					.map(|i| ind + 3 + i)
					.ok_or_else(|| invalid_seq(&source[ind..], ind))?;

				let code_str = &source[ind + 3..end_ind];

				// code to char
				let code_u32 =
					u32::from_str_radix(&parse_dashes_in_nb(code_str, ind + start_ind)?, 16)
						.map_err(|_| invalid_seq(&source[ind..end_ind + 1], ind))?;
				let code = char::from_u32(code_u32)
					.ok_or_else(|| invalid_seq(&source[ind..end_ind + 1], ind))?;

				(code, code_str.len() + 4)
			}
			_ => return Err(invalid_seq(&source[ind..ind + 2], ind)),
		};

		res.push(resolved);
		last_ind = ind + escape_len;
	}

	res.push_str(&source[last_ind..]);

	return Ok(res);
}

/// parse float literal
fn parse_float(source: &str, mut ind: usize) -> Result<(Token<'_>, usize), ParserError> {
	let start_ind = ind;
	// sign
	if let Some('-' | '+') = get_char(source, ind) {
		ind += 1;
	}

	// decimal part
	let decimal_end = while_matching(source, ind, |c| matches!(c, '0'..='9' | '_'));
	ind = decimal_end;

	//fractional part
	if get_char(source, ind) == Some('.') {
		ind = while_matching(source, ind + 1, |c| matches!(c, '0'..='9' | '_'));
		if ind == decimal_end + 1 {
			return Err(unexpected_token('.', ind));
		}
	}

	// exponent
	if let Some('e' | 'E') = get_char(source, ind) {
		let e_ind = ind;
		ind += 1;
		//sign
		if let Some('-' | '+') = get_char(source, ind) {
			ind += 1;
		}
		let exp_start = ind;
		// digits
		ind = while_matching(source, ind, |c| matches!(c, '0'..='9' | '_'));
		if ind == exp_start {
			return Err(unexpected_token('e', e_ind));
		}
	}

	// remove dashes
	let nb_source = parse_dashes_in_nb(&source[start_ind..ind], start_ind)?;

	// create token
	let value = nb_source.parse::<f64>();
	if value.is_err() {
		return Err(ParserError::SyntaxError(format!("invalid number {nb_source} at {start_ind}")));
	}
	Ok((Token::Float(value.unwrap(), start_ind), ind))
}

/// parse integer literals
fn parse_nb(source: &str, mut ind: usize) -> Result<(Token<'_>, usize), ParserError> {
	let sign_ind = ind;

	// sign
	let (has_sign, neg) = match get_char(source, ind) {
		Some('-') => (true, true),
		Some('+') => (true, false),
		_ => (false, false),
	};
	if has_sign {
		// skip sign
		ind += 1
	};

	// base
	let base = match get_char(source, ind) {
		Some('0') => match get_char(source, ind + 1) {
			Some('x') => 16,
			Some('b') => 2,
			_ => 10,
		},
		Some(_) => 10,
		None => return Err(end_of_input(source.len())),
	};
	if base != 10 {
		// skip 0x or 0b
		ind += 2
	}

	// digits
	let start_ind = ind;
	let end_ind = match base {
		2 => while_matching(source, ind + 2, |c| matches!(c, '0' | '1' | '_')),
		10 => while_matching(source, ind, |c| matches!(c, '0'..='9' | '_')),
		16 => while_matching(source, ind, |c| matches!(c, '0'..='9' | 'a'..='f' | 'A'..='F' | '_')),
		_ => unreachable!(),
	};
	let nb_source = parse_dashes_in_nb(&source[start_ind..end_ind], start_ind)?;
	if nb_source.len() == 0 {
		return Err(end_of_input(source.len()));
	}
	ind = end_ind;

	// case float
	if let Some('.' | 'e' | 'E') = get_char(source, ind) {
		return parse_float(source, sign_ind);
	}

	// suffix
	let suffix_end =
		while_matching(source, ind, |c| matches!(c, 'a'..='z' | 'A'..='Z' | '0' ..= '9'));
	let suffix = &source[end_ind..suffix_end];
	ind = suffix_end;

	// value type
	let value_type = match suffix {
		"bint" => NbValueType::BigInt,
		"" => match has_sign {
			true => NbValueType::Int,
			false => NbValueType::Uint,
		},
		_ => {
			return Err(ParserError::SyntaxError(format!(
				"unsupported suffix \"{suffix}\" at {sign_ind}",
			)));
		}
	};

	//create token
	match value_type {
		NbValueType::Uint => {
			// disallow negative sign
			if neg {
				return Err(ParserError::SyntaxError(format!(
					"negative sign in unsigned number at {sign_ind}",
				)));
			};

			// parse
			let value = u64::from_str_radix(&nb_source, base);
			if value.is_err() {
				return Err(ParserError::SyntaxError(format!(
					"unsigned number ({nb_source}) out of range at {sign_ind}",
				)));
			}
			Ok((Token::Uint(value.unwrap(), sign_ind), ind))
		}
		NbValueType::Int => {
			// parse
			let value = i64::from_str_radix(&nb_source, base);
			if value.is_err() {
				return Err(ParserError::SyntaxError(format!(
					"signed number ({nb_source}) out of range at {sign_ind}",
				)));
			}
			Ok((Token::Int(value.unwrap() * if neg { -1 } else { 1 }, sign_ind), ind))
		}
		NbValueType::BigInt => {
			// todo

			Ok((Token::BigInt((), sign_ind), ind))
		}
	}
}

/// simplify the source into sequence of tokens
pub fn tokenize(source: &str) -> Result<Vec<Token<'_>>, ParserError> {
	let mut tokens = Vec::<Token>::new();
	let mut ind: usize = 0;

	let next = |char, ind| match source[ind..].find(char) {
		Some(i) => Ok(ind + i),
		_ => Err(end_of_input(source.len())),
	};

	while source.len() > ind {
		let cur_char = get_char(source, ind).unwrap();

		match cur_char {
			' ' | '\t' | '\n' | '\r' => {}

			'.' => {
				// case .digit, float
				if let Some('0'..='9') = get_char(source, ind + 1) {
					let (token, _ind) = parse_float(source, ind)?;
					tokens.push(token);
					ind = _ind;
					continue;
				} else {
					tokens.push(Token::Symbol('.', ind));
				}
			}
			// one char tokens
			',' => tokens.push(Token::Symbol(',', ind)),
			':' => tokens.push(Token::Symbol(':', ind)),
			'(' => tokens.push(Token::Symbol('(', ind)),
			')' => tokens.push(Token::Symbol(')', ind)),
			'[' => tokens.push(Token::Symbol('[', ind)),
			']' => tokens.push(Token::Symbol(']', ind)),
			'{' => tokens.push(Token::Symbol('{', ind)),
			'}' => tokens.push(Token::Symbol('}', ind)),
			'<' => tokens.push(Token::Symbol('<', ind)),
			'>' => tokens.push(Token::Symbol('>', ind)),
			'?' => tokens.push(Token::Symbol('?', ind)),
			'@' => tokens.push(Token::Symbol('@', ind)),

			// identifiers
			'a'..='z' | 'A'..='Z' | '_' => {
				let end_ind = while_matching(
					source,
					ind,
					|c| matches!(c, 'a'..='z' | 'A'..='Z' | '_' | '-' |'0'..='9'),
				);

				tokens.push(Token::Identifier(&source[ind..end_ind], ind));
				ind = end_ind;
				continue;
			}

			// strings
			'"' => {
				let mut end_index = ind;
				// get end quote, handling escaped quotes
				loop {
					end_index = next("\"", end_index + 1)?;
					if get_char(source, end_index - 1).unwrap() != '\\' {
						break;
					}
				}

				let str = parse_escape_sequences(&source[ind + 1..end_index], ind)?;
				tokens.push(Token::Str(str, ind));
				ind = end_index + 1;
				continue;
			}

			// numbers
			'0'..='9' | '-' | '+' => {
				// case [+-] identifier like -inf
				if (cur_char == '-' || cur_char == '+')
					&& get_char(source, ind + 1).is_some_and(|c| !matches!(c, '0'..='9'))
				{
					tokens.push(match cur_char {
						'-' => Token::Symbol('-', ind),
						'+' => Token::Symbol('+', ind),
						_ => unreachable!(),
					});
					ind += 1;
					continue;
				}

				// else number
				let (token, _ind) = parse_nb(source, ind)?;
				tokens.push(token);
				ind = _ind;
				continue;
			}

			// comments
			'/' => {
				let next_char = get_char(source, ind + 1);
				match next_char {
					// single line
					Some('/') => ind = source[ind..].find("\n").unwrap_or(source.len()) + 1,
					// multi line
					Some('*') => ind = next("*/", ind)? + 2,
					Some(char) => return Err(unexpected_token(char, ind)),
					None => return Err(end_of_input(source.len())),
				}
				continue;
			}

			_ => {
				return Err(unexpected_token(cur_char, ind));
			}
		}
		// inc ind for one char tokens
		ind += 1;
	}
	tokens.push(Token::EOF(source.len()));

	Ok(tokens)
}
