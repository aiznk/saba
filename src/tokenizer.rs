use crate::error::{Error, make_error, err_runtime};

#[derive(Debug, PartialEq)]
pub enum TokenKind {
	Nil,
	Ident, // id
	String, // "str"
	Int, // 1
	Float, // 1.23
	Create, // CREATE
	PrimaryKey, // PRIMARY_KEY
	AutoIncrement, // AUTO_INCREMENT
	Database, // DATABASE
	Databases, // DATABASES
	Use, // USE
	Show, // SHOW
	Drop, // DROP
	Table, // TABLE
	Tables, // TABLES
	I64,
	F64,
	Char, // CHAR
	Get, // GET
	Set, // SET
	Add, // ADD
	Del, // DEL
	Of, // OF
	Where, // WHERE
	Eq, // ==
	NotEq, // !=
	And, // AND
	Or, // OR
	All, // ALL
	Assign, // =
	Semicolon, // ; 
	Colon, // :
	Comma, // ,
	LParen, // (
	RParen, // )
	LBracket, // [
	RBracket, // ]
}

#[derive(Debug)]
pub struct Token {
	pub kind: TokenKind,
	pub text: Option<String>,
	pub i64_value: Option<i64>,
	pub f64_value: Option<f64>,
}

impl Token {
	pub fn new() -> Self {
		Self {
			kind: TokenKind::Nil,
			text: None,
			i64_value: None,
			f64_value: None,
		}
	}

	pub fn from(kind: TokenKind, text: Option<String>) -> Self {
		Self {
			kind,
			text,
			i64_value: None,
			f64_value: None,
		}
	}

	pub fn from_int(n: i64) -> Self {
		Self {
			kind: TokenKind::Int,
			text: None,
			i64_value: Some(n),
			f64_value: None,
		}
	}

	pub fn from_float(n: f64) -> Self {
		Self {
			kind: TokenKind::Float,
			text: None,
			i64_value: None,
			f64_value: Some(n),
		}
	}
}

pub struct TokenStream {
	pub tokens: Vec<Token>,
	pub index: usize,
}

impl TokenStream {
	pub fn new(tokens: Vec<Token>) -> Self {
		Self {
			tokens,
			index: 0,
		}
	}

	pub fn is_end(&self) -> bool {
		self.index >= self.tokens.len()
	}

	pub fn prev(&mut self) {
		if self.index > 0 {
			self.index -= 1;
		}
	}

	pub fn next(&mut self) {
		self.index += 1;
	}

	pub fn cur(&self) -> Result<&Token, Error> {
		if self.index >= self.tokens.len() {
			return err_runtime!("index out of range (cur)");
		}
		Ok(&self.tokens[self.index]) 
	}

	pub fn get(&mut self) -> Result<&Token, Error> {
		if self.index >= self.tokens.len() {
			return err_runtime!("index out of range (get)");
		}
		let token = &self.tokens[self.index];
		self.index += 1;
		Ok(token)
	}
}

fn is_ident_head(c: char) -> bool {
	c.is_alphabetic() || c == '_'
}

fn is_ident_char(c: char) -> bool {
	c.is_alphanumeric() || c == '_'
}

fn read_string(i: &mut usize, chars: &Vec<char>) -> Token {
	let mut text = String::new();

	*i += 1; // "

	while *i < (*chars).len() {
		let c = (*chars)[*i];
		if c == '"' {
			break;
		} else {
			text.push(c);
		}
		*i += 1;
	}

	Token::from(TokenKind::String, Some(text))
}

fn read_number(i: &mut usize, chars: &Vec<char>) -> Token {
	let mut text = String::new();
	let mut float = false;

	while *i < (*chars).len() {
		let c = (*chars)[*i];
		if c.is_numeric() {
			text.push(c);
		} else if c == '.' {
			text.push(c);
			float = true;
		} else {
			*i -= 1;
			break;
		}
		*i += 1;
	}

	if float {
		let n = text.parse::<f64>().unwrap();
		Token::from_float(n)
	} else {
		let n = text.parse::<i64>().unwrap();
		Token::from_int(n)
	}
}

fn read_ident(i: &mut usize, chars: &Vec<char>) -> Token {
	let mut text = String::new();

	while *i < (*chars).len() {
		let c = (*chars)[*i];
		if is_ident_char(c) {
			text.push(c);
		} else {
			*i -= 1;
			break;
		}
		*i += 1;
	}

	Token::from(TokenKind::Ident, Some(text))
}

pub fn show_tokens(tokens: &Vec<Token>) {
	for (i, tok) in (*tokens).iter().enumerate() {
		println!("{}: {:?}", i, tok);
	}
}

pub fn tokenize(string: String) -> Result<Vec<Token>, Error> {
	let chars: Vec<char> = string.chars().collect();
	let mut i: usize = 0;
	let mut ret: Vec<Token> = vec![];

	while i < chars.len() {
		let mut c1: char = '?';
		let mut c2: char = '?';
		let mut c3: char = '?';
		let mut c4: char = '?';
		let mut c5: char = '?';
		let mut c6: char = '?';
		let mut c7: char = '?';
		let mut c8: char = '?';
		let mut c9: char = '?';
		let mut c10: char = '?';
		let mut c11: char = '?';
		let mut c12: char = '?';
		let mut c13: char = '?';
		let mut c14: char = '?';

		c1 = chars[i].to_ascii_lowercase();
		if i+1 < chars.len() {
			c2 = chars[i+1].to_ascii_lowercase();
		}
		if i+2 < chars.len() {
			c3 = chars[i+2].to_ascii_lowercase();
		}
		if i+3 < chars.len() {
			c4 = chars[i+3].to_ascii_lowercase();
		}
		if i+4 < chars.len() {
			c5 = chars[i+4].to_ascii_lowercase();
		}
		if i+5 < chars.len() {
			c6 = chars[i+5].to_ascii_lowercase();
		}
		if i+6 < chars.len() {
			c7 = chars[i+6].to_ascii_lowercase();
		}
		if i+7 < chars.len() {
			c8 = chars[i+7].to_ascii_lowercase();
		}
		if i+8 < chars.len() {
			c9 = chars[i+8].to_ascii_lowercase();
		}
		if i+9 < chars.len() {
			c10 = chars[i+9].to_ascii_lowercase();
		}
		if i+10 < chars.len() {
			c11 = chars[i+10].to_ascii_lowercase();
		}
		if i+11 < chars.len() {
			c12 = chars[i+11].to_ascii_lowercase();
		}
		if i+12 < chars.len() {
			c13 = chars[i+12].to_ascii_lowercase();
		}
		if i+13 < chars.len() {
			c14 = chars[i+13].to_ascii_lowercase();
		}

		// println!("{} {} {}", c1, c2, c3);

		if c1 == 'g' && c2 == 'e' && c3 == 't' {
			ret.push(Token::from(TokenKind::Get, None));
			i += 2;
		} else if c1 == 's' && c2 == 'e' && c3 == 't' {
			ret.push(Token::from(TokenKind::Set, None));
			i += 2;
		} else if c1 == 'a' && c2 == 'd' && c3 == 'd' {
			ret.push(Token::from(TokenKind::Add, None));
			i += 2;
		} else if c1 == 'd' && c2 == 'e' && c3 == 'l' {
			ret.push(Token::from(TokenKind::Del, None));
			i += 2;
		} else if c1 == 'a' && c2 == 'l' && c3 == 'l' {
			ret.push(Token::from(TokenKind::All, None));
			i += 2;
		} else if c1 == 'a' && c2 == 'n' && c3 == 'd' {
			ret.push(Token::from(TokenKind::And, None));
			i += 2;
		} else if c1 == 'i' && c2 == '6' && c3 == '4' {
			ret.push(Token::from(TokenKind::I64, None));
			i += 2;
		} else if c1 == 'f' && c2 == '6' && c3 == '4' {
			ret.push(Token::from(TokenKind::F64, None));
			i += 2;
		} else if c1 == 'u' && c2 == 's' && c3 == 'e' {
			ret.push(Token::from(TokenKind::Use, None));
			i += 2;
		} else if c1 == 'd' && c2 == 'r' && c3 == 'o' && c4 == 'p' {
			ret.push(Token::from(TokenKind::Drop, None));
			i += 3;
		} else if c1 == 's' && c2 == 'h' && c3 == 'o' && c4 == 'w' {
			ret.push(Token::from(TokenKind::Show, None));
			i += 3;
		} else if c1 == 'c' && c2 == 'h' && c3 == 'a' && c4 == 'r' {
			ret.push(Token::from(TokenKind::Char, None));
			i += 3;
		} else if c1 == 'o' && c2 == 'r' {
			ret.push(Token::from(TokenKind::Or, None));
			i += 1;
		} else if c1 == 'o' && c2 == 'f' {
			ret.push(Token::from(TokenKind::Of, None));
			i += 1;
		} else if c1 == 'w' && c2 == 'h' && c3 == 'e' && c4 == 'r' && c5 == 'e' {
			ret.push(Token::from(TokenKind::Where, None));
			i += 4;
		} else if c1 == 't' && c2 == 'a' && c3 == 'b' && c4 == 'l' && c5 == 'e' && c6 == 's' {
			ret.push(Token::from(TokenKind::Tables, None));
			i += 5;
		} else if c1 == 't' && c2 == 'a' && c3 == 'b' && c4 == 'l' && c5 == 'e' {
			ret.push(Token::from(TokenKind::Table, None));
			i += 4;
		} else if c1 == 'c' && c2 == 'r' && c3 == 'e' && c4 == 'a' && c5 == 't' && c6 == 'e' {
			ret.push(Token::from(TokenKind::Create, None));
			i += 5;
		} else if c1 == 'd' && c2 == 'a' && c3 == 't' && c4 == 'a' && c5 == 'b' && c6 == 'a' && c7 == 's' && c8 == 'e' && c9 == 's' {
			ret.push(Token::from(TokenKind::Databases, None));
			i += 8;
		} else if c1 == 'd' && c2 == 'a' && c3 == 't' && c4 == 'a' && c5 == 'b' && c6 == 'a' && c7 == 's' && c8 == 'e' {
			ret.push(Token::from(TokenKind::Database, None));
			i += 7;
		} else if c1 == 'p' && c2 == 'r' && c3 == 'i' && c4 == 'm' && c5 == 'a' && c6 == 'r' && c7 == 'y' && c8 == '_' && c9 == 'k' && c10 == 'e' && c11 == 'y' {
			// primary_key
			ret.push(Token::from(TokenKind::PrimaryKey, None));
			i += 10;
		} else if c1 == 'a' && c2 == 'u' && c3 == 't' && c4 == 'o' && c5 == '_' && c6 == 'i' && c7 == 'n' && c8 == 'c' && c9 == 'r' && c10 == 'e' && c11 == 'm' && c12 == 'e' && c13 == 'n' && c14 == 't' {
			// auto_increment
			ret.push(Token::from(TokenKind::PrimaryKey, None));
			i += 10;
		} else if c1 == '=' && c2 == '=' {
			ret.push(Token::from(TokenKind::Eq, None));
			i += 1;
		} else if c1 == '!' && c2 == '=' {
			ret.push(Token::from(TokenKind::NotEq, None));
			i += 1;
		} else if c1 == '=' {
			ret.push(Token::from(TokenKind::Assign, None));
		} else if c1 == ';' {
			ret.push(Token::from(TokenKind::Semicolon, None));
		} else if c1 == ':' {
			ret.push(Token::from(TokenKind::Colon, None));
		} else if c1 == ',' {
			ret.push(Token::from(TokenKind::Comma, None));
		} else if c1 == '(' {
			ret.push(Token::from(TokenKind::LParen, None));
		} else if c1 == ')' {
			ret.push(Token::from(TokenKind::RParen, None));
		} else if c1 == '[' {
			ret.push(Token::from(TokenKind::LBracket, None));
		} else if c1 == ']' {
			ret.push(Token::from(TokenKind::RBracket, None));
		} else if c1 == '"' {
			let tok = read_string(&mut i, &chars);
			ret.push(tok);
		} else if c1.is_numeric() {
			let tok = read_number(&mut i, &chars);
			ret.push(tok);
		} else if is_ident_head(c1) {
			let tok = read_ident(&mut i, &chars);
			ret.push(tok);
		}

		i += 1;
	}

	// show_tokens(&ret);
	Ok(ret)
}
