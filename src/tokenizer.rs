use crate::error::{Error, make_error, err_runtime};

#[derive(Clone, Debug, PartialEq)]
pub enum TokenKind {
	Nil,
	Ident, // id
	String, // "str"
	Create, // CREATE
	Rename, // RENAME
	To, // TO
	Order, // ORDER
	By, // BY
	Desc, // DESC
	Asc, // ASC
	PrimaryKey, // PRIMARY_KEY
	AutoIncrement, // AUTO_INCREMENT
	Database, // DATABASE
	Databases, // DATABASES
	Default, // DEFAULT
	Alter, // ALTER
	Use, // USE
	Show, // SHOW
	Limit, // LIMIT
	Drop, // DROP
	Table, // TABLE
	Tables, // TABLES
	Int, // value of int
	Float, // value of float
	TypeI64, // type of int
	TypeF64, // type of float
	Bool, // BOOL
	True, // true, TRUE
	False, // false, FALSE
	Column, // COLUMN
	Char, // CHAR
	If, // IF
	Type, // TYPE
	Exists, // EXISTS
	Not, // NOT
	Get, // GET
	Set, // SET
	Add, // ADD
	Del, // DEL
	Of, // OF
	Where, // WHERE
	Values, // VALUES
	AddOp, // +
	SubOp, // -
	MulOp, // *
	DivOp, // /
	ModOp, // %
	Lt, // <
	LtEq, // <=
	Gt, // >
	GtEq, // >=
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
	#[allow(dead_code)]
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

	#[allow(dead_code)]
	pub fn next(&mut self) {
		self.index += 1;
	}

	#[allow(dead_code)]
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

#[allow(dead_code)]
pub fn show_tokens(tokens: &Vec<Token>) {
	for (i, tok) in (*tokens).iter().enumerate() {
		println!("{}: {:?}", i, tok);
	}
}

pub fn tokenize(string: String) -> Result<Vec<Token>, Error> {
	let mut s = string.clone();
	s = s.replace("\n", " ");
	s = s.replace("\t", " ");
	s.push(' ');
	let chars: Vec<char> = s.chars().collect();
	let mut i: usize = 0;
	let mut ret: Vec<Token> = vec![];

	while i < chars.len() {
		let c1: char;
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
		let mut c15: char = '?';
		let mut c16: char = '?';

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
		if i+14 < chars.len() {
			c15 = chars[i+14].to_ascii_lowercase();
		}
		if i+15 < chars.len() {
			c16 = chars[i+15].to_ascii_lowercase();
		}

		// println!("{} {} {}", c1, c2, c3);

		if c1 == 'g' && c2 == 'e' && c3 == 't' && c4 == ' ' {
			ret.push(Token::from(TokenKind::Get, None));
			i += 2;
		} else if c1 == 's' && c2 == 'e' && c3 == 't' && c4 == ' ' {
			ret.push(Token::from(TokenKind::Set, None));
			i += 2;
		} else if c1 == 'a' && c2 == 'd' && c3 == 'd' && c4 == ' ' {
			ret.push(Token::from(TokenKind::Add, None));
			i += 2;
		} else if c1 == 'd' && c2 == 'e' && c3 == 'l' && c4 == ' ' {
			ret.push(Token::from(TokenKind::Del, None));
			i += 2;
		} else if c1 == ' ' && c2 == 'a' && c3 == 'l' && c4 == 'l' && c5 == ' ' {
			ret.push(Token::from(TokenKind::All, None));
			i += 3;
		} else if c1 == ' ' && c2 == 'a' && c3 == 'n' && c4 == 'd' && c5 == ' ' {
			ret.push(Token::from(TokenKind::And, None));
			i += 3;
		} else if c1 == ' ' && c2 == 'a' && c3 == 's' && c4 == 'c' && c5 == ' ' {
			ret.push(Token::from(TokenKind::Asc, None));
			i += 3;
		} else if (c1 == ' ' || c1 == '(' || c1 == ':') && c2 == 'i' && c3 == '6' && c4 == '4' && (c5 == ' ' || c5 == ',' || c5 == ')' || c5 == ';') {
			ret.push(Token::from(TokenKind::TypeI64, None));
			i += 3;
		} else if (c1 == ' ' || c1 == '(' || c1 == ':') && c2 == 'f' && c3 == '6' && c4 == '4' && (c5 == ' ' || c5 == ',' || c5 == ')' || c5 == ';') {
			ret.push(Token::from(TokenKind::TypeF64, None));
			i += 3;
		} else if c1 == 'u' && c2 == 's' && c3 == 'e' && c4 == ' ' {
			ret.push(Token::from(TokenKind::Use, None));
			i += 3;
		} else if c1 == ' ' && c2 == 'n' && c3 == 'o' && c4 == 't' && c5 == ' ' {
			ret.push(Token::from(TokenKind::Not, None));
			i += 3;
		} else if c1 == ' ' && c2 == 'v' && c3 == 'a' && c4 == 'l' && c5 == 'u' && c6 == 'e' && c7 == 's' && c8 == ' ' {
			ret.push(Token::from(TokenKind::Values, None));
			i += 6;
		} else if c1 == ' ' && c2 == 't' && c3 == 'y' && c4 == 'p' && c5 == 'e' && c6 == ' ' {
			ret.push(Token::from(TokenKind::Type, None));
			i += 4;
		} else if c1 == 'd' && c2 == 'e' && c3 == 's' && c4 == 'c' && c5 == ' ' {
			ret.push(Token::from(TokenKind::Desc, None));
			i += 3;
		} else if c1 == 'd' && c2 == 'r' && c3 == 'o' && c4 == 'p' && c5 == ' ' {
			ret.push(Token::from(TokenKind::Drop, None));
			i += 3;
		} else if c1 == 's' && c2 == 'h' && c3 == 'o' && c4 == 'w' && c5 == ' ' {
			ret.push(Token::from(TokenKind::Show, None));
			i += 3;
		} else if (c1 == ' ' || c1 == ':' || c1 == '(') && c2 == 'c' && c3 == 'h' && c4 == 'a' && c5 == 'r' && (c6 == ' ' || c6 == '[') {
			ret.push(Token::from(TokenKind::Char, None));
			i += 4;
		} else if (c1 == ' ' || c1 == ':' || c1 == '(') && c2 == 'b' && c3 == 'o' && c4 == 'o' && c5 == 'l' && (c6 == ' ' || c6 == ';' || c6 == ')' || c6 == ',') {
			ret.push(Token::from(TokenKind::Bool, None));
			i += 4;
		} else if (c1 == ' ' || c1 == '=') && c2 == 't' && c3 == 'r' && c4 == 'u' && c5 == 'e' && (c6 == ' ' || c6 == ';' || c6 == ')' || c6 == ',') {
			ret.push(Token::from(TokenKind::True, None));
			i += 4;
		} else if (c1 == ' ' || c1 == '=') && c2 == 'f' && c3 == 'a' && c4 == 'l' && c5 == 's' && c6 == 'e' && (c7 == ' ' || c7 == ';' || c7 == ',' || c7 == ')') {
			ret.push(Token::from(TokenKind::False, None));
			i += 5;
		} else if c1 == ' ' && c2 == 'o' && c3 == 'r' && c4 == ' ' {
			ret.push(Token::from(TokenKind::Or, None));
			i += 2;
		} else if c1 == ' ' && c2 == 'o' && c3 == 'f' && c4 == ' ' {
			ret.push(Token::from(TokenKind::Of, None));
			i += 2;
		} else if c1 == ' ' && c2 == 'b' && c3 == 'y' && c4 == ' ' {
			ret.push(Token::from(TokenKind::By, None));
			i += 2;
		} else if c1 == ' ' && c2 == 'o' && c3 == 'r' && c4 == 'd' && c5 == 'e' && c6 == 'r' && c7 == ' ' {
			ret.push(Token::from(TokenKind::Order, None));
			i += 5;
		} else if c1 == ' ' && c2 == 'w' && c3 == 'h' && c4 == 'e' && c5 == 'r' && c6 == 'e' && c7 == ' ' {
			ret.push(Token::from(TokenKind::Where, None));
			i += 5;
		} else if c1 == 'a' && c2 == 'l' && c3 == 't' && c4 == 'e' && c5 == 'r' && c6 == ' ' {
			ret.push(Token::from(TokenKind::Alter, None));
			i += 4;
		} else if c1 == ' ' && c2 == 'l' && c3 == 'i' && c4 == 'm' && c5 == 'i' && c6 == 't' && c7 == ' ' {
			ret.push(Token::from(TokenKind::Limit, None));
			i += 5;
		} else if c1 == ' ' && c2 == 't' && c3 == 'a' && c4 == 'b' && c5 == 'l' && c6 == 'e' && c7 == 's' && (c8 == ' ' || c8 == ';') {
			ret.push(Token::from(TokenKind::Tables, None));
			i += 6;
		} else if c1 == ' ' && c2 == 'e' && c3 == 'x' && c4 == 'i' && c5 == 's' && c6 == 't' && c7 == 's' && c8 == ' ' {
			ret.push(Token::from(TokenKind::Exists, None));
			i += 6;
		} else if c1 == ' ' && c2 == 't' && c3 == 'a' && c4 == 'b' && c5 == 'l' && c6 == 'e' && c7 == ' ' {
			ret.push(Token::from(TokenKind::Table, None));
			i += 5;
		} else if c1 == ' ' && c2 == 'c' && c3 == 'o' && c4 == 'l' && c5 == 'u' && c6 == 'm' && c7 == 'n' && c8 == ' ' {
			ret.push(Token::from(TokenKind::Column, None));
			i += 6;
		} else if c1 == 'c' && c2 == 'r' && c3 == 'e' && c4 == 'a' && c5 == 't' && c6 == 'e' && c7 == ' ' {
			ret.push(Token::from(TokenKind::Create, None));
			i += 5;
		} else if c1 == ' ' && c2 == 'r' && c3 == 'e' && c4 == 'n' && c5 == 'a' && c6 == 'm' && c7 == 'e' && c8 == ' ' {
			ret.push(Token::from(TokenKind::Rename, None));
			i += 6;
		} else if c1 == ' ' && c2 == 'd' && c3 == 'a' && c4 == 't' && c5 == 'a' && c6 == 'b' && c7 == 'a' && c8 == 's' && c9 == 'e' && c10 == 's' && (c11 == ' ' || c11 == ';') {
			ret.push(Token::from(TokenKind::Databases, None));
			i += 9;
		} else if c1 == ' ' && c2 == 'd' && c3 == 'e' && c4 == 'f' && c5 == 'a' && c6 == 'u' && c7 == 'l' && c8 == 't' && c9 == ' ' {
			ret.push(Token::from(TokenKind::Default, None));
			i += 7;
		} else if c1 == ' ' && c2 == 'd' && c3 == 'a' && c4 == 't' && c5 == 'a' && c6 == 'b' && c7 == 'a' && c8 == 's' && c9 == 'e' && c10 == ' ' {
			ret.push(Token::from(TokenKind::Database, None));
			i += 8;
		} else if (c1 == ' ' || c1 == '(' || c1 == ',' || c1 == ':') && c2 == 'p' && c3 == 'r' && c4 == 'i' && c5 == 'm' && c6 == 'a' && c7 == 'r' && c8 == 'y' && c9 == '_' && c10 == 'k' && c11 == 'e' && c12 == 'y' && (c13 == ' ' || c13 == ',' || c13 == ')' || c13 == ';') {
			// primary_key
			ret.push(Token::from(TokenKind::PrimaryKey, None));
			i += 11;
		} else if (c1 == ' ' || c1 == '(' || c1 == ',' || c1 == ':') && c2 == 'a' && c3 == 'u' && c4 == 't' && c5 == 'o' && c6 == '_' && c7 == 'i' && c8 == 'n' && c9 == 'c' && c10 == 'r' && c11 == 'e' && c12 == 'm' && c13 == 'e' && c14 == 'n' && c15 == 't' && (c16 == ' ' || c16 == ',' || c16 == ')' || c16 == ';') {
			// auto_increment
			ret.push(Token::from(TokenKind::AutoIncrement, None));
			i += 14;
		} else if c1 == ' ' && c2 == 't' && c3 == 'o' && c4 == ' ' {
			ret.push(Token::from(TokenKind::To, None));
			i += 2;
		} else if c1 == ' ' && c2 == 'i' && c3 == 'f' && c4 == ' ' {
			ret.push(Token::from(TokenKind::If, None));
			i += 2;
		} else if c1 == '=' && c2 == '=' {
			ret.push(Token::from(TokenKind::Eq, None));
			i += 1;
		} else if c1 == '!' && c2 == '=' {
			ret.push(Token::from(TokenKind::NotEq, None));
			i += 1;
		} else if c1 == '<' && c2 == '=' {
			ret.push(Token::from(TokenKind::LtEq, None));
			i += 1;
		} else if c1 == '>' && c2 == '=' {
			ret.push(Token::from(TokenKind::GtEq, None));
			i += 1;
		} else if c1 == '<' {
			ret.push(Token::from(TokenKind::Lt, None));
		} else if c1 == '>' {
			ret.push(Token::from(TokenKind::Gt, None));
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
		} else if c1 == '+' {
			ret.push(Token::from(TokenKind::AddOp, None));
		} else if c1 == '-' {
			ret.push(Token::from(TokenKind::SubOp, None));
		} else if c1 == '*' {
			ret.push(Token::from(TokenKind::MulOp, None));
		} else if c1 == '/' {
			ret.push(Token::from(TokenKind::DivOp, None));
		} else if c1 == '%' {
			ret.push(Token::from(TokenKind::ModOp, None));
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
