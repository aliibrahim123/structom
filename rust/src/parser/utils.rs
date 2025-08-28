use crate::{
	Error,
	errors::{end_of_input, unexpected_token},
	parser::tokenizer::Token,
};

pub fn get_char(str: &str, ind: usize) -> Option<char> {
	str.as_bytes().get(ind).map(|b| *b as char)
}
pub fn is_hex(char: char) -> bool {
	matches!(char, '0'..='9' | 'a'..='f' | 'A'..='F')
}

pub fn while_matching(source: &str, ind: usize, pred: fn(char) -> bool) -> usize {
	source
		.get(ind..)
		.unwrap_or("")
		.find(|c| !pred(c))
		.map(|i| ind + i)
		.unwrap_or(source.len())
}

pub fn all_matching(source: &str, pred: fn(char) -> bool) -> bool {
	source.bytes().all(|c| pred(c as char))
}

pub fn consume_ident<'a>(tokens: &'a [Token], ind: &mut usize) -> Result<&'a str, Error> {
	match tokens.get(*ind) {
		Some(Token::Identifier(ident, _)) => (Ok(*ident), *ind += 1).0,
		Some(Token::EOF(_)) | None => Err(end_of_input(tokens[*ind].ind())),
		Some(token) => Err(unexpected_token(token, token.ind())),
	}
}
pub fn consume_str<'a>(tokens: &'a [Token], ind: &mut usize) -> Result<&'a str, Error> {
	match tokens.get(*ind) {
		Some(Token::Str(str, _)) => (Ok(&str[..]), *ind += 1).0,
		Some(Token::EOF(_)) | None => Err(end_of_input(tokens[*ind].ind())),
		Some(token) => Err(unexpected_token(token, token.ind())),
	}
}
pub fn consume_symbol(token: char, tokens: &[Token], ind: &mut usize) -> Result<bool, Error> {
	match tokens.get(*ind) {
		Some(Token::Symbol(sym, _)) if *sym == token => (Ok(true), *ind += 1).0,
		Some(Token::EOF(_)) | None => Err(end_of_input(tokens[*ind].ind())),
		Some(token) => Err(unexpected_token(token, token.ind())),
	}
}
pub fn consume_int(tokens: &[Token], ind: &mut usize) -> Result<i64, Error> {
	match tokens.get(*ind) {
		Some(Token::Uint(nb, _)) => (Ok(*nb as i64), *ind += 1).0,
		Some(Token::Int(nb, _)) => (Ok(*nb), *ind += 1).0,
		Some(Token::EOF(_)) | None => Err(end_of_input(tokens[*ind].ind())),
		Some(token) => Err(unexpected_token(token, token.ind())),
	}
}
pub fn consume_uint(tokens: &[Token], ind: &mut usize) -> Result<u64, Error> {
	match tokens.get(*ind) {
		Some(Token::Uint(nb, _)) => (Ok(*nb), *ind += 1).0,
		Some(Token::Int(nb, _)) if (*nb >= 0) => (Ok(*nb as u64), *ind += 1).0,
		Some(Token::EOF(_)) | None => Err(end_of_input(tokens[*ind].ind())),
		Some(token) => Err(unexpected_token(token, token.ind())),
	}
}
pub fn consume_bigint<'a>(tokens: &'a [Token], ind: &mut usize) -> Result<&'a [u8], Error> {
	match tokens.get(*ind) {
		Some(Token::BigInt(nb, _)) => (Ok(&nb[..]), *ind += 1).0,
		Some(Token::EOF(_)) | None => Err(end_of_input(tokens[*ind].ind())),
		Some(token) => Err(unexpected_token(token, token.ind())),
	}
}
pub fn consume_float(tokens: &[Token], ind: &mut usize) -> Result<f64, Error> {
	match tokens.get(*ind) {
		Some(Token::Float(nb, _)) => (Ok(*nb), *ind += 1).0,
		Some(Token::Int(nb, _)) => (Ok(*nb as f64), *ind += 1).0,
		Some(Token::Uint(nb, _)) => (Ok(*nb as f64), *ind += 1).0,
		Some(Token::EOF(_)) | None => Err(end_of_input(tokens[*ind].ind())),
		Some(token) => Err(unexpected_token(token, token.ind())),
	}
}

// handle commas
pub fn struct_like_start(
	tokens: &[Token], ind: &mut usize, watched_comma: &mut bool, end_delimiter: char,
) -> Result<bool, Error> {
	// break on end
	if let Some(Token::Symbol(c, _)) = tokens.get(*ind)
		&& *c == end_delimiter
	{
		*ind += 1;
		return Ok(true);
	}

	// no comma
	if *watched_comma == false {
		return Err(unexpected_token(&tokens[*ind], tokens[*ind].ind()));
	}
	*watched_comma = false;

	Ok(false)
}
pub fn struct_like_end(tokens: &[Token], ind: &mut usize, watched_comma: &mut bool) {
	// allow no comma at end
	if let Some(Token::Symbol(',', _)) = tokens.get(*ind) {
		*ind += 1;
		*watched_comma = true;
	}
}
