#[derive(Debug, Clone)]
pub enum ObjectKind {
	Nil,
	Bool,
	I64,
	F64,
	String,
	Ident,
}

#[derive(Debug, Clone)]
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

	pub fn to_string(&self) -> String {
		match self.kind {
			ObjectKind::Nil => { String::from("nil") }
			ObjectKind::Bool => { format!("{}", self.bool_value) }
			ObjectKind::I64 => { format!("{}", self.i64_value) }
			ObjectKind::F64 => { format!("{}", self.f64_value) }
			ObjectKind::String => { self.string.clone() }
			ObjectKind::Ident => { self.ident.clone() }
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
			kind: ObjectKind::Bool,
			bool_value: false,
			i64_value: n,
			f64_value: 0.0,
			string: String::new(),
			ident: String::new(),
		}		
	}

	pub fn from_f64(n: f64) -> Self {
		Self {
			kind: ObjectKind::Bool,
			bool_value: false,
			i64_value: 0,
			f64_value: n,
			string: String::new(),
			ident: String::new(),
		}		
	}

	pub fn from_string(s: String) -> Self {
		Self {
			kind: ObjectKind::Bool,
			bool_value: false,
			i64_value: 0,
			f64_value: 0.0,
			string: s,
			ident: String::new(),
		}		
	}
}

