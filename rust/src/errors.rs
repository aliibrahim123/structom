#[derive(Debug)]
pub enum Error {
	SyntaxError {
		disc: &'static str,
		token: Option<String>,
		ind: usize,
	},
}

pub(crate) fn end_of_input(source: &str) -> Error {
	Error::SyntaxError {
		disc: "end of input",
		token: None,
		ind: source.len(),
	}
}

pub(crate) fn unexpected_token(token: char, ind: usize) -> Error {
	Error::SyntaxError {
		disc: "unexpected token",
		token: Some(token.to_string()),
		ind,
	}
}
