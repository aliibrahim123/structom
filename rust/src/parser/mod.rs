mod tokenizer;
mod utils;

use tokenizer::Token;

pub fn parse(source: &str) -> Vec<Token> {
	tokenizer::tokenize(source).unwrap()
}
