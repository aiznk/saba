#[derive(Debug, Clone)]
pub struct Context {
	pub root_dir_path: String,
	pub using_db_name: String,
}

impl Context {
	pub fn new() -> Self {
		Self {
			root_dir_path: String::new(),
			using_db_name: String::new(),
		}
	}
}
