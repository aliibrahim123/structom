use crate::{
	ParseError,
	errors::err,
	parser::tokenizer::{Pos, Token},
};

pub trait StrExt {
	/// get char at byte index
	fn char_at(&self, ind: usize) -> Option<char>;

	/// find the index of the first char matching the pattern after index
	fn find_after(&self, pat: char, ind: usize) -> Option<usize>;

	/// find the index of the first char matching the pattern after index
	fn find_str_after(&self, pat: &str, ind: usize) -> Option<usize>;
}

impl StrExt for str {
	fn char_at(&self, ind: usize) -> Option<char> {
		self.as_bytes().get(ind).map(|b| *b as char)
	}
	fn find_after(&self, pat: char, ind: usize) -> Option<usize> {
		self[ind..].find(pat).map(|i| ind + i)
	}
	fn find_str_after(&self, pat: &str, ind: usize) -> Option<usize> {
		self[ind..].find(pat).map(|i| ind + i)
	}
}

pub fn is_hex(char: char) -> bool {
	matches!(char, '0'..='9' | 'a'..='f' | 'A'..='F')
}

/// count the times a string appears as a prefix
pub fn count_prefix(source: &str, search: &str) -> usize {
	let mut count = 0;
	let mut ind = 0;
	while source.get(ind..ind + search.len()) == Some(search) {
		count += 1;
		ind += search.len();
	}
	count
}

/// remove a suffix n times
pub fn remove_n_suffix<'a>(source: &'a str, search: &str, count: usize) -> &'a str {
	let mut ind = source.len();
	for _ in 0..count {
		match source[..ind].rfind(search) {
			Some(i) => ind = i,
			None => return "",
		}
	}
	&source[..ind]
}

/// return the index after the last char matching the predicate after index
pub fn while_matching(source: &str, ind: usize, pred: fn(char) -> bool) -> usize {
	match source.get(ind..).unwrap_or("").find(|c| !pred(c)) {
		Some(i) => ind + i,
		_ => source.len(),
	}
}

/// whether all chars match the predicate
pub fn all_matching(source: &str, pred: fn(char) -> bool) -> bool {
	source.bytes().all(|c| pred(c as char))
}

/// safely consume an identifier
pub fn consume_ident<'a>(
	tokens: &'a [Token], ind: &mut usize, file: &str,
) -> Result<&'a str, ParseError> {
	match tokens.get(*ind) {
		Some(Token::Ident(ident, _)) => (Ok(*ident), *ind += 1).0,
		Some(Token::EOF(_)) | None => end_of_input(file),
		Some(token) => unexpected_token(token, token.pos(), file),
	}
}
/// safely consume a string
pub fn consume_str<'a>(
	tokens: &'a [Token], ind: &mut usize, file: &str,
) -> Result<&'a str, ParseError> {
	match tokens.get(*ind) {
		Some(Token::Str(str, _)) => (Ok(&str[..]), *ind += 1).0,
		Some(Token::EOF(_)) | None => end_of_input(file),
		Some(token) => unexpected_token(token, token.pos(), file),
	}
}
/// safely consume a symbol
pub fn consume_symbol(
	token: char, tokens: &[Token], ind: &mut usize, file: &str,
) -> Result<bool, ParseError> {
	match tokens.get(*ind) {
		Some(Token::Symbol(sym, _)) if *sym == token => (Ok(true), *ind += 1).0,
		Some(Token::EOF(_)) | None => end_of_input(file),
		Some(token) => unexpected_token(token, token.pos(), file),
	}
}
/// safely consume an uint
pub fn consume_uint(tokens: &[Token], ind: &mut usize, file: &str) -> Result<u64, ParseError> {
	match tokens.get(*ind) {
		Some(Token::Uint(nb, _)) => (Ok(*nb), *ind += 1).0,
		Some(Token::Int(nb, _)) if (*nb >= 0) => (Ok(*nb as u64), *ind += 1).0,
		Some(Token::EOF(_)) | None => end_of_input(file),
		Some(token) => unexpected_token(token, token.pos(), file),
	}
}

/// handle struct like syntax commons, must be at start of field loop
pub fn struct_like_start(
	tokens: &[Token], ind: &mut usize, watched_comma: &mut bool, end_delimiter: char, file: &str,
) -> Result<bool, ParseError> {
	// break on end
	if let Some(Token::Symbol(c, _)) = tokens.get(*ind)
		&& *c == end_delimiter
	{
		*ind += 1;
		return Ok(true);
	}

	// no comma
	if *watched_comma == false {
		return unexpected_token(&tokens[*ind], tokens[*ind].pos(), file);
	}
	*watched_comma = false;

	Ok(false)
}
// handle struct like syntax commons, must be at end of field loop
pub fn struct_like_end(tokens: &[Token], ind: &mut usize, watched_comma: &mut bool) {
	// allow no comma at end
	if let Some(Token::Symbol(',', _)) = tokens.get(*ind) {
		*ind += 1;
		*watched_comma = true;
	}
}

pub fn unexpected_token<T>(token: impl ToString, pos: Pos, file: &str) -> Result<T, ParseError> {
	err!(format!("unexpected token \"{}\"", token.to_string()), pos, file)
}
pub fn end_of_input<T>(file: &str) -> Result<T, ParseError> {
	err!("end of input".to_string(), file)
}
