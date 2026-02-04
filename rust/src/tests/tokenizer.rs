use std::str::FromStr;

use num_bigint::BigInt;

use crate::parser::tokenizer::{Pos, Token, tokenize};

mod token {
	use num_bigint::BigInt;

	use crate::parser::tokenizer::{Pos, Token};

	pub fn symbol<'a>(c: char) -> Token<'a> {
		Token::Symbol(c, Pos::new(1, 1))
	}
	pub fn ident<'a>(s: &'a str) -> Token<'a> {
		Token::Ident(s, Pos::new(1, 1))
	}
	pub fn str(s: &str) -> Token<'_> {
		Token::Str(s.to_string(), Pos::new(1, 1))
	}
	pub fn uint<'a>(n: u64) -> Token<'a> {
		Token::Uint(n, Pos::new(1, 1))
	}
	pub fn int<'a>(n: i64) -> Token<'a> {
		Token::Int(n, Pos::new(1, 1))
	}
	pub fn bigint<'a>(n: BigInt) -> Token<'a> {
		Token::BigInt(n, Pos::new(1, 1))
	}
	pub fn float<'a>(n: f64) -> Token<'a> {
		Token::Float(n, Pos::new(1, 1))
	}
}

fn test(source: &str, tokens: &[Token]) {
	let res = tokenize(source, "test").unwrap();

	let eq = res[0..res.len() - 1].iter().enumerate().all(|(i, t)| match (t, tokens.get(i)) {
		(Token::Ident(a, _), Some(Token::Ident(b, _))) => a == b,
		(Token::Str(a, _), Some(Token::Str(b, _))) => a == b,
		(Token::Uint(a, _), Some(Token::Uint(b, _))) => a == b,
		(Token::Int(a, _), Some(Token::Int(b, _))) => a == b,
		(Token::BigInt(a, _), Some(Token::BigInt(b, _))) => a == b,
		(Token::Float(a, _), Some(Token::Float(b, _))) => a == b,
		(Token::Symbol(a, _), Some(Token::Symbol(b, _))) => a == b,
		_ => false,
	});

	assert!(eq, "expected {tokens:#?}, got {res:#?}");
}
#[test]
fn core() {
	assert_eq!(tokenize("", ""), Ok(vec![Token::EOF(Pos::new(1, 1))]));

	test("  \t\t\n\n\r\r", &[]);
	test("// a comment \n // a comment", &[]);
	test(" /* a comment\n */ ", &[]);
	assert!(tokenize("/* unnclosed ", "").is_err());

	test(",:?@.", &",:?@.".chars().map(token::symbol).collect::<Vec<_>>());
	test("()[]{}<>", &"()[]{}<>".chars().map(token::symbol).collect::<Vec<_>>());
	assert!(tokenize("~", "").is_err());
}

#[test]
fn ident() {
	use token::ident;
	test("abcd", &[ident("abcd")]);
	test("EFGH", &[ident("EFGH")]);
	test("a_a", &[ident("a_a")]);
	test("n123", &[ident("n123")]);
	test("abcdEFG123_", &[ident("abcdEFG123_")]);
}

#[test]
fn str() {
	use token::str;
	test(r#"  ""  "#, &[str("")]);
	test(r#""abc""#, &[str("abc")]);
	test(r#""\0\n\r\t\\""#, &[str("\0\n\r\t\\")]);
	test(r#""\x61\x62\x63""#, &[str("abc")]);
	test(r#""\u{1_F6_00}""#, &[str("😀")]);
	test(r#""a \"str\" inside \"str\"""#, &[str("a \"str\" inside \"str\"")]);

	assert!(tokenize(r#""unclosed"#, "").is_err());
	assert!(tokenize(r#""\a""#, "").is_err());
	assert!(tokenize(r#""\xGG""#, "").is_err());
	assert!(tokenize(r#""\u{1""#, "").is_err());
	assert!(tokenize(r#""\u{ 123 }""#, "").is_err());
}

#[test]
fn nb() {
	use token::*;
	test("0123456789", &[uint(123456789)]);
	test("-123 +123", &[int(-123), int(123)]);
	test("123_456", &[uint(123456)]);
	test("0b10101 0xFe0", &[uint(0b10101), uint(0xFe0)]);
	test(
		"1234567890123456789012345678901234567890bint",
		&[bigint(BigInt::from_str("1234567890123456789012345678901234567890").unwrap())],
	);

	assert!(tokenize("0x", "").is_err());
	assert!(tokenize("0_", "").is_err());
	assert!(tokenize("1__0", "").is_err());
	assert!(tokenize("1234567890123456789012345678901234567890", "").is_err());

	test("1.0", &[float(1.0)]);
	test("-1.0", &[float(-1.0)]);
	test(".1", &[float(0.1)]);
	test("1.0e1", &[float(10.0)]);
	test("1E-1", &[float(0.1)]);
	test("1_0.0_1e0_1", &[float(100.1)]);

	assert!(tokenize("1.", "").is_err());
	assert!(tokenize("1e ", "").is_err());
	assert!(tokenize("1e-", "").is_err());
	assert!(tokenize("1.E", "").is_err());
}

#[test]
fn complex() {
	use token::*;
	#[rustfmt::skip]
	let expected = &[
		symbol('{'),
        ident("a"), symbol(':'), uint(1), symbol(','),
        ident("b"), symbol(':'), symbol('['), float(1.0), symbol(','), str("a"), symbol(']'), symbol(','),
        ident("c"), symbol(':'), bigint(BigInt::from(2)), symbol(','),
        ident("d"), symbol(':'), symbol('@'), ident("base"), symbol('('), str("16"), symbol(')'), str("\x01\x02\x03"),
        symbol('}')
	];
	let source =
		r#"{ a: 1, b: [1.0, "a"], c: 2bint, /* a comment */ d: @base("16") "\x01\x02\x03" }"#;
	test(source, expected);
}

#[test]
fn pos() {
	let source =
		r#"{ a: 1, b: [1.0, "a"], c: 2bint, /* a comment */ d: @base("16") "\x01\x02\x03" }"#;
	assert_eq!(tokenize(source, "").unwrap()[25].pos(), Pos::new(1, 80));

	let source = r#"123
1234 1234
123 here"#;
	assert_eq!(tokenize(source, "").unwrap()[4].pos(), Pos::new(3, 5));

	let source = r#""123
123 😀 1
123 😀 123" here"#;
	assert_eq!(tokenize(source, "").unwrap()[1].pos(), Pos::new(3, 12))
}
