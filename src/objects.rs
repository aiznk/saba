use crate::error::{Error, make_error, err_parse, err_runtime};

#[derive(Debug, Clone, PartialEq)]
pub struct HeaderType {
	pub ident: String,
	pub is_int: bool,
	pub is_float: bool,
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
			is_int: false,
			is_float: false,
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

		if self.is_int {
			s.push_str("INT");
		} else if self.is_float {
			s.push_str("FLOAT");
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
			if self.is_int {
				return Ok(String::from("0"));
			} else if self.is_float {
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
		if self.is_int {
			let n = match s.parse::<i128>() {
				Ok(v) => v,
				Err(e) => return err_parse!("failed to parse as i64. {}", e),
			};
			return Ok(Object::from_int(n));
		} else if self.is_float {
			if self.is_auto_increment {
				return err_parse!("cannot auto increment f64");
			}
			let n = match s.parse::<f64>() {
				Ok(v) => v,
				Err(e) => return err_parse!("failed to parse as f64. {}", e),
			};
			return Ok(Object::from_float(n));
		} else if self.is_bool {
			if self.is_auto_increment {
				return err_parse!("cannot auto increment bool");
			}
			let n = match s.parse::<bool>() {
				Ok(v) => v,
				Err(e) => return err_parse!("failed to parse as bool. {}", e),
			};
			return Ok(Object::from_bool(n));
		} else if self.is_char {
			if self.is_auto_increment {
				return err_parse!("cannot auto increment char[]");
			}
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
	Int,
	Float,
	String,
	Ident,
	Star,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Object {
	pub kind: ObjectKind,
	pub bool_value: bool,
	pub int_value: i128,
	pub float_value: f64,
	pub string: String,
	pub ident: String,
}

impl Object {
	pub fn new() -> Self {
		Self {
			kind: ObjectKind::Nil,
			bool_value: false,
			int_value: 0,
			float_value: 0.0,
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
			ObjectKind::Int => { format!("{}", self.int_value) }
			ObjectKind::Float => { format!("{}", self.float_value) }
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
			int_value: 0,
			float_value: 0.0,
			string: String::new(),
			ident: String::from(ident),
		}		
	}

	pub fn from_star() -> Self {
		Self {
			kind: ObjectKind::Star,
			bool_value: false,
			int_value: 0,
			float_value: 0.0,
			string: String::new(),
			ident: String::new(),
		}		
	}

	pub fn from_bool(b: bool) -> Self {
		Self {
			kind: ObjectKind::Bool,
			bool_value: b,
			int_value: 0,
			float_value: 0.0,
			string: String::new(),
			ident: String::new(),
		}		
	}

	pub fn from_int(n: i128) -> Self {
		Self {
			kind: ObjectKind::Int,
			bool_value: false,
			int_value: n,
			float_value: 0.0,
			string: String::new(),
			ident: String::new(),
		}		
	}

	pub fn from_float(n: f64) -> Self {
		Self {
			kind: ObjectKind::Float,
			bool_value: false,
			int_value: 0,
			float_value: n,
			string: String::new(),
			ident: String::new(),
		}		
	}

	pub fn from_string(s: String) -> Self {
		Self {
			kind: ObjectKind::String,
			bool_value: false,
			int_value: 0,
			float_value: 0.0,
			string: s,
			ident: String::new(),
		}		
	}
}

