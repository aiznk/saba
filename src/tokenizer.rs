use crate::error::{Error, err_runtime};

#[derive(Debug, PartialEq)]
pub enum TokenKind {
	Nil,
	Ident, // id
	String, // "str"
	Int, // 1
	Float, // 1.23
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
}

#[derive(Debug)]
pub struct Token {
	pub kind: TokenKind,
	pub text: Option<String>,
	pub int_value: Option<i64>,
	pub float_value: Option<f64>,
}

impl Token {
	pub fn new() -> Self {
		Self {
			kind: TokenKind::Nil,
			text: None,
			int_value: None,
			float_value: None,
		}
	}

	pub fn from(kind: TokenKind, text: Option<String>) -> Self {
		Self {
			kind,
			text,
			int_value: None,
			float_value: None,
		}
	}

	pub fn from_int(n: i64) -> Self {
		Self {
			kind: TokenKind::Int,
			text: None,
			int_value: Some(n),
			float_value: None,
		}
	}

	pub fn from_float(n: f64) -> Self {
		Self {
			kind: TokenKind::Float,
			text: None,
			int_value: None,
			float_value: Some(n),
		}
	}
}

pub struct TokenStream {
	tokens: Vec<Token>,
	index: usize,
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
			return err_runtime!("index out of range");
		}
		Ok(&self.tokens[self.index]) 
	}

	pub fn get(&mut self) -> Result<&Token, Error> {
		if self.index >= self.tokens.len() {
			return err_runtime!("index out of range");
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
		} else if c1 == 'o' && c2 == 'r' {
			ret.push(Token::from(TokenKind::Or, None));
			i += 1;
		} else if c1 == 'o' && c2 == 'f' {
			ret.push(Token::from(TokenKind::Of, None));
			i += 1;
		} else if c1 == 'w' && c2 == 'h' && c3 == 'e' && c4 == 'r' && c5 == 'e' {
			ret.push(Token::from(TokenKind::Where, None));
			i += 4;
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

	show_tokens(&ret);
	Ok(ret)
}
