enum ParseError {
	UnexpectedToken { token: char, ind: usize },
	EndOfInput,
}
