use crate::fail::Fail;

enum TokenKind {
	Nil,
	Ident,
	String,
	Int,
	Float,
	Get,
	Set,
	Add,
	Del,
	Of,
	Where,
}

pub struct Token {
	kind: TokenKind,
	text: Option<String>,
}

impl Token {
	pub fn new() -> Self {
		Self {
			kind: TokenKind::Nil,
			text: None,
		}
	}

	pub fn from(kind: TokenKind, text: Option<String>) -> Self {
		Self {
			kind,
			text,
		}
	}
}

fn read_ident(i: &mut usize, chars: &Vec<char>) -> Token {
	todo!();
	Token::new()
}

pub fn tokenize(string: String) -> Result<Vec<Token>, Fail> {
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
			ret.push(Token::from(TokenKind::Add, None));
			i += 2;
		} else if c1 == 'o' && c2 == 'f' {
			ret.push(Token::from(TokenKind::Add, None));
			i += 1;
		} else if c1 == 'w' && c2 == 'h' && c3 == 'e' && c4 == 'r' && c5 == 'e' {
			ret.push(Token::from(TokenKind::Add, None));
			i += 4;
		} else {
			let tok = read_ident(&mut i, &chars);
			ret.push(tok);
		}

		i += 1;
	}

	Ok(ret)
}
