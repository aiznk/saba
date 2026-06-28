use std::fs::File;
use std::path::{Path, PathBuf};
use crate::error::{make_error, err_runtime, Error};
use csv::{Reader, StringRecord};

#[derive(Debug)]
pub struct Context {
	pub root_dir_path: String,
	pub using_db_name: String,
	pub table_csv_reader: Option<Reader<File>>,
	pub csv_header: StringRecord,
	pub csv_header_idents: Vec<String>,
	pub csv_record: StringRecord,
	pub selected_csv_columns: Vec<String>,
}

impl Context {
	pub fn new() -> Self {
		Self {
			root_dir_path: String::new(),
			using_db_name: String::new(),
			table_csv_reader: None,
			csv_header: StringRecord::new(),
			csv_header_idents: vec![],
			csv_record: StringRecord::new(),
			selected_csv_columns: vec![],
		}
	}

	pub fn gen_table_file_path(&self, table_name: &str) -> Result<PathBuf, Error> {
		if self.root_dir_path.len() == 0 ||
		   self.using_db_name.len() == 0 {
		   	return err_runtime!("invalid state in gen table file path");
		}

		let path = Path::new(&self.root_dir_path).join(&self.using_db_name).join(table_name.to_lowercase() + ".csv");

		Ok(path)
	}
}
