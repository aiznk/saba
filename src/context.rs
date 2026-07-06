use crate::error::{make_error, err_runtime, Error};
use crate::objects::{Object, HeaderType};
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
	pub scan_record: StringRecord,
	pub selected_csv_columns: Vec<String>,
	pub vars: HashMap<String, Box<Object>>,
	pub counter_selected: usize,

	// if cli mode, set true. that print projected columns
	pub is_cli: bool,

	// if filter matched/unmatched store record
	pub matched_record: StringRecord,
	pub unmatched_record: StringRecord,

	pub test_get_records: Option<Vec<StringRecord>>,
	pub limit_counter: i64,
	pub filtered: bool,
	pub matched: bool,
}

impl Context {
	pub fn new() -> Self {
		Self {
			root_dir_path: PathBuf::new(),
			using_db_name: String::new(),
			table_csv_reader: None,
			csv_header: StringRecord::new(),
			csv_header_idents: vec![],
			scan_record: StringRecord::new(),
			selected_csv_columns: vec![],
			vars: HashMap::new(),
			counter_selected: 0,
			is_cli: false,
			matched_record: StringRecord::new(),
			unmatched_record: StringRecord::new(),
			test_get_records: None,
			limit_counter: 0,
			filtered: false,
			matched: false,
		}
	}

	pub fn clear(&mut self) {
		self.table_csv_reader = None;
		self.csv_header.clear();
		self.csv_header_idents.clear();
		self.scan_record.clear();
		self.selected_csv_columns.clear();
		self.vars.clear();
		self.counter_selected = 0;
		self.matched_record.clear();
		self.unmatched_record.clear();
		if let Some(test_get_records) = self.test_get_records.as_mut() {
			test_get_records.clear();
		}
		self.limit_counter = 0;
		self.filtered = false;
		self.matched = false;
	}

	pub fn gen_db_dir_path(&self, db_name: &str) -> Result<PathBuf, Error> {
		if self.root_dir_path.as_os_str().is_empty() ||
		   db_name.len() == 0 {
		   	return err_runtime!("invalid state in gen db dir path");
		}

		let path = Path::new(&self.root_dir_path).join(db_name);

		Ok(path)		
	}

	pub fn gen_using_db_tables_path(&self) -> Result<PathBuf, Error> {
		if self.root_dir_path.as_os_str().is_empty() ||
		   self.using_db_name.len() == 0 {
		   	return err_runtime!("invalid state in gen using db dir path");
		}

		let path = Path::new(&self.root_dir_path).join(&self.using_db_name).join("tables");

		Ok(path)		
	}

	pub fn gen_id_file_path(&self, table_name: &str, typ: &HeaderType) -> Result<PathBuf, Error> {
		if self.root_dir_path.as_os_str().is_empty() ||
		   self.using_db_name.len() == 0 {
		   	return err_runtime!("invalid state in gen using db dir path");
		}

		let path = Path::new(&self.root_dir_path)
			.join(&self.using_db_name)
			.join("id")
			.join(format!("{}__{}.txt", table_name, typ.ident));

		Ok(path)		
	}

	#[allow(dead_code)]
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

		let path = Path::new(&self.root_dir_path)
			.join(&self.using_db_name)
			.join("tables")
			.join(table_name.to_lowercase() + ".csv");

		Ok(path)
	}

	pub fn gen_tmp_table_file_path(&self, table_name: &str) -> Result<PathBuf, Error> {
		if self.root_dir_path.as_os_str().is_empty() ||
		   self.using_db_name.len() == 0 {
		   	return err_runtime!("invalid state in gen table file path");
		}

		let path = Path::new(&self.root_dir_path)
			.join(&self.using_db_name)
			.join("tables")
			.join(table_name.to_lowercase() + ".tmp.csv");

		Ok(path)
	}
}
