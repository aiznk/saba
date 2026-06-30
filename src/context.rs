use crate::error::{make_error, err_runtime, Error};
use crate::objects::{Object};
use std::fs::File;
use std::path::{Path, PathBuf};
use csv::{Reader, StringRecord};
use std::collections::HashMap;

#[derive(Debug)]
pub struct Context {
	pub root_dir_path: PathBuf,
	pub using_db_name: String,
	pub table_csv_reader: Option<Reader<File>>,
	pub csv_header: StringRecord,
	pub csv_header_idents: Vec<String>,
	pub csv_record: StringRecord,
	pub selected_csv_columns: Vec<String>,
	pub vars: HashMap<String, Box<Object>>,
	pub counter_selected: usize,

	// if cli mode, set true. that print projected columns
	pub is_cli: bool,

	// if enable sequentil mode on exec_project, set true
	pub is_sequential: bool,

	// if filter matched/unmatched store record
	pub matched_csv_record: StringRecord,
	pub unmatched_csv_record: StringRecord,

	pub test_get_records: Option<Vec<StringRecord>>,
}

impl Context {
	pub fn new() -> Self {
		Self {
			root_dir_path: PathBuf::new(),
			using_db_name: String::new(),
			table_csv_reader: None,
			csv_header: StringRecord::new(),
			csv_header_idents: vec![],
			csv_record: StringRecord::new(),
			selected_csv_columns: vec![],
			vars: HashMap::new(),
			counter_selected: 0,
			is_cli: false,
			is_sequential: false,
			matched_csv_record: StringRecord::new(),
			unmatched_csv_record: StringRecord::new(),
			test_get_records: None,
		}
	}

	pub fn gen_db_dir_path(&self, db_name: &str) -> Result<PathBuf, Error> {
		if self.root_dir_path.as_os_str().is_empty() ||
		   db_name.len() == 0 {
		   	return err_runtime!("invalid state in gen db dir path");
		}

		let path = Path::new(&self.root_dir_path).join(db_name);

		Ok(path)		
	}

	pub fn gen_using_db_dir_path(&self) -> Result<PathBuf, Error> {
		if self.root_dir_path.as_os_str().is_empty() ||
		   self.using_db_name.len() == 0 {
		   	return err_runtime!("invalid state in gen using db dir path");
		}

		let path = Path::new(&self.root_dir_path).join(&self.using_db_name);

		Ok(path)		
	}

	pub fn gen_table_file_path(&self, table_name: &str) -> Result<PathBuf, Error> {
		if self.root_dir_path.as_os_str().is_empty() ||
		   self.using_db_name.len() == 0 {
		   	return err_runtime!("invalid state in gen table file path");
		}

		let path = Path::new(&self.root_dir_path).join(&self.using_db_name).join(table_name.to_lowercase() + ".csv");

		Ok(path)
	}

	pub fn gen_tmp_table_file_path(&self, table_name: &str) -> Result<PathBuf, Error> {
		if self.root_dir_path.as_os_str().is_empty() ||
		   self.using_db_name.len() == 0 {
		   	return err_runtime!("invalid state in gen table file path");
		}

		let path = Path::new(&self.root_dir_path).join(&self.using_db_name).join(table_name.to_lowercase() + ".tmp.csv");

		Ok(path)
	}
}
