#[derive(Debug)]
pub enum Fail {
	Tokenize(String),
	Parse(String),
	Exec(String),
}

impl std::fmt::Display for Fail {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			Fail::Tokenize(s) => write!(f, "failed to tokenize. {}", s),
			Fail::Parse(s) => write!(f, "failed to parse. {}", s),
			Fail::Exec(s) => write!(f, "failed to execute. {}", s),
		}
	}
}

impl std::error::Error for Fail {}

macro_rules! fail_tokenize {
	($msg:expr) => {
		Fail::Tokenize($msg.to_string())
	}
}

macro_rules! fail_parse {
	($msg:expr) => {
		Fail::Parse($msg.to_string())
	}
}

macro_rules! fail_exec {
	($msg:expr) => {
		Fail::Exec($msg.to_string())
	}
}
