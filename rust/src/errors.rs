#[derive(Debug)]
pub enum Error {
	SyntaxError(String),
	TypeError(String),
}

pub fn end_of_input(_len: usize) -> Error {
	Error::SyntaxError("end of input".to_string())
}

pub fn unexpected_token<T: ToString>(token: T, ind: usize) -> Error {
	//panic!();
	let token = token.to_string();
	if token == "end_of_file" {
		return end_of_input(ind);
	}
	Error::SyntaxError(format!("unexpected token `{token}` at {ind}"))
}
