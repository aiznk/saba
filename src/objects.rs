use crate::error::{Error, make_error, err_parse, err_runtime};

#[derive(Debug, Clone, PartialEq)]
pub struct HeaderType {
	pub ident: String,
	pub is_i64: bool,
	pub is_f64: bool,
	pub is_bool: bool,
	pub is_char: bool,
	pub char_size: usize,
	pub is_primary_key: bool,
	pub is_auto_increment: bool,
	pub is_default: bool,
	pub default_value: Option<Object>,
}

impl HeaderType {
	pub fn new() -> Self {
		Self {
			ident: String::new(),
			is_i64: false,
			is_f64: false,
			is_bool: false,
			is_char: false,
			char_size: 0,
			is_primary_key: false,
			is_auto_increment: false,			
			is_default: false,
			default_value: None,
		}
	}

	pub fn to_string(&self) -> String {
		let mut s = String::new();
		
		s.push_str(self.ident.as_str());
		s.push_str(": ");

		if self.is_i64 {
			s.push_str("I64");
		} else if self.is_f64 {
			s.push_str("F64");
		} else if self.is_bool {
			s.push_str("BOOL");
		} else if self.is_char {
			s.push_str("CHAR[");
			s.push_str(self.char_size.to_string().as_str());
			s.push_str("]");
		}
		if self.is_primary_key {
			s.push_str(" PRIMARY_KEY");
		}
		if self.is_auto_increment {
			s.push_str(" AUTO_INCREMENT");
		}
		if self.is_default {
			s.push_str(" DEFAULT");
			if let Some(default_value) = &self.default_value {
				s.push(' ');
				s.push_str(default_value.to_column_string().as_str());
			}
		}

		s
	}

	pub fn to_default_value_string(&self) -> Result<String, Error> {
		if self.is_default {
			if let Some(default_value) = &self.default_value {
				return Ok(default_value.to_string());
			} else {
				return err_runtime!("missing default value");
			}
		} else {
			if self.is_i64 {
				return Ok(String::from("0"));
			} else if self.is_f64 {
				return Ok(String::from("0.0"));
			} else if self.is_bool {
				return Ok(String::from("false"));
			} else if self.is_char {
				return Ok(String::from(""));
			}
			return err_runtime!("invalid state: header type");
		}
	}

	pub fn parse_str(&self, s: &str) -> Result<Object, Error> {
		if self.is_i64 {
			let n = match s.parse::<i64>() {
				Ok(v) => v,
				Err(e) => return err_parse!("failed to parse as i64. {}", e),
			};
			return Ok(Object::from_i64(n));
		} else if self.is_f64 {
			let n = match s.parse::<f64>() {
				Ok(v) => v,
				Err(e) => return err_parse!("failed to parse as f64. {}", e),
			};
			return Ok(Object::from_f64(n));
		} else if self.is_bool {
			let n = match s.parse::<bool>() {
				Ok(v) => v,
				Err(e) => return err_parse!("failed to parse as bool. {}", e),
			};
			return Ok(Object::from_bool(n));
		} else if self.is_char {
			if s.len() > self.char_size {
				return err_parse!("{} size is over char size of {}", s.len(), self.char_size);
			}
			return Ok(Object::from_string(s.to_string()));
		}

		return err_parse!("failed to parse str as type");
	}
}

#[derive(Debug, Clone, PartialEq)]
pub enum ObjectKind {
	Nil,
	Bool,
	I64,
	F64,
	String,
	Ident,
	Star,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Object {
	pub kind: ObjectKind,
	pub bool_value: bool,
	pub i64_value: i64,
	pub f64_value: f64,
	pub string: String,
	pub ident: String,
}

impl Object {
	pub fn new() -> Self {
		Self {
			kind: ObjectKind::Nil,
			bool_value: false,
			i64_value: 0,
			f64_value: 0.0,
			string: String::new(),
			ident: String::new(),
		}
	}

	pub fn to_column_string(&self) -> String {
		match self.kind {
			ObjectKind::String => { format!("\"{}\"", self.string) }
			_ => { self.to_string() }
		}
	}

	pub fn to_string(&self) -> String {
		match self.kind {
			ObjectKind::Nil => { String::from("nil") }
			ObjectKind::Bool => { format!("{}", self.bool_value) }
			ObjectKind::I64 => { format!("{}", self.i64_value) }
			ObjectKind::F64 => { format!("{}", self.f64_value) }
			ObjectKind::String => { self.string.clone() }
			ObjectKind::Ident => { self.ident.clone() }
			ObjectKind::Star => { String::from("*") }
		}
	}

	#[allow(dead_code)]
	pub fn from_nil() -> Self {
		let mut o = Object::new();
		o.kind = ObjectKind::Nil;
		o
	}

	pub fn from_ident(ident: &str) -> Self {
		Self {
			kind: ObjectKind::Ident,
			bool_value: false,
			i64_value: 0,
			f64_value: 0.0,
			string: String::new(),
			ident: String::from(ident),
		}		
	}

	pub fn from_star() -> Self {
		Self {
			kind: ObjectKind::Star,
			bool_value: false,
			i64_value: 0,
			f64_value: 0.0,
			string: String::new(),
			ident: String::new(),
		}		
	}

	pub fn from_bool(b: bool) -> Self {
		Self {
			kind: ObjectKind::Bool,
			bool_value: b,
			i64_value: 0,
			f64_value: 0.0,
			string: String::new(),
			ident: String::new(),
		}		
	}

	pub fn from_i64(n: i64) -> Self {
		Self {
			kind: ObjectKind::I64,
			bool_value: false,
			i64_value: n,
			f64_value: 0.0,
			string: String::new(),
			ident: String::new(),
		}		
	}

	pub fn from_f64(n: f64) -> Self {
		Self {
			kind: ObjectKind::F64,
			bool_value: false,
			i64_value: 0,
			f64_value: n,
			string: String::new(),
			ident: String::new(),
		}		
	}

	pub fn from_string(s: String) -> Self {
		Self {
			kind: ObjectKind::String,
			bool_value: false,
			i64_value: 0,
			f64_value: 0.0,
			string: s,
			ident: String::new(),
		}		
	}
}

