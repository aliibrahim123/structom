/// error produced by the parsing functions
#[derive(Debug, Clone, PartialEq)]
pub enum ParserError {
	/// errors related to syntax (unexpected tokens, invalid literials...).
	SyntaxError(String),
	/// errors related to types (undefined types, undefined fields, ...).
	TypeError(String),
}

pub fn end_of_input(_len: usize) -> ParserError {
	ParserError::SyntaxError("end of input".to_string())
}

pub fn unexpected_token<T: ToString>(token: T, ind: usize) -> ParserError {
	//panic!();
	let token = token.to_string();
	if token == "end_of_file" {
		return end_of_input(ind);
	}
	ParserError::SyntaxError(format!("unexpected token `{token}` at {ind}"))
}
