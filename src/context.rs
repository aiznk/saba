use crate::error::{make_error, err_runtime, Error};
use crate::objects::{Object, HeaderType, Table};
use crate::consts::{NIL};
use std::fs::File;
use std::path::{Path, PathBuf};
use csv::{Reader, StringRecord};
use std::collections::HashMap;

#[derive(Debug)]
pub struct Context {
	pub root_dir_path: PathBuf,
	pub using_db_name: String,
	pub current_table_name: String,
	pub selected_csv_columns: Vec<String>,
	pub vars: HashMap<String, Box<Object>>,
	pub counter_selected: usize,
	pub distinct_map: HashMap<String, bool>,
	pub cache_distinct_objs: Option<Vec<Object>>,
	pub tables: HashMap<String, Box<Table>>,
	pub do_read_record: bool,
	pub joined_headers: StringRecord,
	pub joined_header_types: Vec<HeaderType>,
	pub joined_header_idents: Vec<String>,
	pub joined_record: StringRecord,
	pub join_matched: bool,
	pub joins_enable: bool,
	pub wait_left_scan: bool,
	pub scanned_record_is_empty: bool,
	pub id_counter: usize,
	pub selected_header_idents: Vec<String>,
	pub join_matched_counter: usize,
	pub finished_scan_table_names: Vec<String>,

	// if cli mode, set true. that print projected columns
	pub is_cli: bool,

	// if filter matched/unmatched store record
	pub matched_record: StringRecord,
	pub unmatched_record: StringRecord,

	pub test_get_records: Option<Vec<StringRecord>>,
	pub test_selected_records: Option<Vec<StringRecord>>,
	pub limit_counter: i128,

	// functions
	pub count_counter: usize,
	pub sum_value: f64,
	pub avg_sum_value: f64,
	pub avg_counter: usize,
	pub min_value: f64,
	pub max_value: f64,
}

impl Context {
	pub fn new() -> Self {
		Self {
			root_dir_path: PathBuf::new(),
			using_db_name: String::new(),
			current_table_name: String::new(),
			selected_csv_columns: vec![],
			vars: HashMap::new(),
			counter_selected: 0,
			distinct_map: HashMap::new(),
			cache_distinct_objs: None,
			tables: HashMap::new(),
			do_read_record: false,
			joined_headers: StringRecord::new(),
			joined_header_types: vec![],
			joined_header_idents: vec![],
			joined_record: StringRecord::new(),
			join_matched: false,
			joins_enable: false,
			wait_left_scan: false,
			scanned_record_is_empty: false,
			id_counter: 1,
			selected_header_idents: vec![],
			join_matched_counter: 0,
			finished_scan_table_names: vec![],
			is_cli: false,
			matched_record: StringRecord::new(),
			unmatched_record: StringRecord::new(),
			test_get_records: None,
			test_selected_records: None,
			limit_counter: 0,
			count_counter: 0,
			sum_value: 0.0,
			avg_sum_value: 0.0,
			avg_counter: 0,
			min_value: f64::MAX,
			max_value: 0.0,
		}
	}

	pub fn clear(&mut self) {
		self.current_table_name.clear();
		self.selected_csv_columns.clear();
		self.vars.clear();
		self.counter_selected = 0;
		self.distinct_map.clear();
		self.cache_distinct_objs = None;
		self.tables.clear();
		self.do_read_record = false;
		self.joined_headers.clear();
		self.joined_header_types.clear();
		self.joined_header_idents.clear();
		self.joined_record.clear();
		self.join_matched = false;
		self.joins_enable = false;
		self.wait_left_scan = false;
		self.scanned_record_is_empty = false;
		self.id_counter = 1;
		self.selected_header_idents.clear();
		self.join_matched_counter = 0;
		self.finished_scan_table_names.clear();
		self.matched_record.clear();
		self.unmatched_record.clear();
		if let Some(test_get_records) = self.test_get_records.as_mut() {
			test_get_records.clear();
		}
		if let Some(test_selected_records) = self.test_selected_records.as_mut() {
			test_selected_records.clear();
		}
		self.limit_counter = 0;
		self.count_counter = 0;
		self.sum_value = 0.0;
		self.avg_sum_value = 0.0;
		self.avg_counter = 0;
		self.min_value = f64::MAX;
		self.max_value = 0.0;
	}

	pub fn join_table_header_idents(&self) -> Vec<String> {
		let mut ret: Vec<String> = vec![];

		if self.tables.len() == 0 {
			return ret;
		} else if self.tables.len() >= 2 {
			for table in self.tables.values() {
				for ident in table.header_idents.iter() {
					let ident = format!("{}.{}", table.name, ident);
					ret.push(ident);
				}
			}
		} else {
			for table in self.tables.values() {
				ret = table.header_idents.clone();
			}			
		}

		ret
	}

	pub fn join_table_records(&self) -> (StringRecord, bool) {
		let mut ret = StringRecord::new();
		let mut v: Vec<(usize, StringRecord)> = vec![];

		for table in self.tables.values() {
			let id = table.id;
			let r = table.scanned_record.clone();
			if r.len() == 0 {
				return (ret, false);
			}
			v.push((id, r));
		}

		v.sort_by(|a, b| a.0.cmp(&b.0));

		for (_id, record) in v.iter() {
			for field in record.iter() {
				ret.push_field(field);
			}
		}

		(ret, true)
	}

	pub fn replace_scanned_record_to_nil_record(&mut self, table_name: &String) -> Result<(), Error> {
		if let Some(table) = self.tables.get_mut(table_name) {
			let len = table.headers.len();
			table.scanned_record.clear();
			for _ in 0..len {
				table.scanned_record.push_field(NIL);
			}
			Ok(())
		} else {
			return err_runtime!("not found table '{}' on gen table nil record", table_name);
		}
	}

	pub fn joins_enable_unmatched(&self) -> bool {
		self.joins_enable && !self.join_matched
	}

	pub fn print_record(&self, record: &StringRecord) {
		let mut s = String::new();

		for field in record.iter() {
			s.push_str(field);
			s.push(',');
		}
		s.pop();

		println!("{}", s);
	}

	pub fn print_tables_scanned_records(&self) {
		for table in self.tables.values() {
			print!("{}: ", table.name);
			self.print_record(&table.scanned_record);
		}		
	}

	pub fn clear_tables_scanned_records(&mut self) -> Result<(), Error> {
		for table in self.tables.values_mut() {
			table.scanned_record.clear();
		}
		Ok(())	
	}

	pub fn get_current_table_scanned_record(&self) -> Result<StringRecord, Error> {
		if let Some(table) = self.tables.get(self.current_table_name.as_str()) {
			Ok(table.scanned_record.clone())
		} else {
			err_runtime!("not found table '{}' in get table scanned record", self.current_table_name)
		}
	}

	pub fn set_table_scanned_record(&mut self, table_name: &str, scanned_record: StringRecord) -> Result<(), Error> {
		if let Some(table) = self.tables.get_mut(table_name) {
			table.scanned_record = scanned_record;
		} else {
			return err_runtime!("not found table '{}' in get table scanned record", table_name);
		}
		Ok(())
	}	

	pub fn get_table_scanned_record(&self, table_name: &str) -> Result<StringRecord, Error> {
		if let Some(table) = self.tables.get(table_name) {
			Ok(table.scanned_record.clone())
		} else {
			err_runtime!("not found table '{}' in get table scanned record", table_name)
		}
	}

	pub fn get_current_table_headers(&mut self) -> Result<StringRecord, Error> {
		if let Some(table) = self.tables.get(self.current_table_name.as_str()) {
			Ok(table.headers.clone())
		} else {
			err_runtime!("not found table '{}' in get current table headers", self.current_table_name)
		}
	}

	pub fn get_table_headers(&self, table_name: &str) -> Result<StringRecord, Error> {
		if let Some(table) = self.tables.get(table_name) {
			Ok(table.headers.clone())
		} else {
			err_runtime!("not found table '{}'", table_name)
		}
	}

	pub fn get_table_header_idents(&self, table_name: &str) -> Result<Vec<String>, Error> {
		if let Some(table) = self.tables.get(table_name) {
			Ok(table.header_idents.clone())
		} else {
			err_runtime!("not found table '{}' (2)", table_name)
		}
	}

	pub fn get_table_header_types(&self, table_name: &str) -> Result<Vec<HeaderType>, Error> {
		if let Some(table) = self.tables.get(table_name) {
			Ok(table.header_types.clone())
		} else {
			err_runtime!("not found table '{}' (3)", table_name)
		}
	}

	pub fn set_table_header_types(&mut self, table_name: &str, header_types: &Vec<HeaderType>) -> Result<(), Error> {
		if let Some(table) = self.tables.get_mut(table_name) {
			table.header_types = header_types.clone();
			Ok(())
		} else {
			err_runtime!("not found table '{}' (4)", table_name)
		}
	}

	pub fn set_table_header_idents(&mut self, table_name: &str, header_idents: &Vec<String>) -> Result<(), Error> {
		if let Some(table) = self.tables.get_mut(table_name) {
			table.header_idents = header_idents.clone();
			Ok(())
		} else {
			err_runtime!("not found table '{} (5)'", table_name)
		}
	}

	pub fn gen_db_dir_path(&self) -> Result<PathBuf, Error> {
		let path = Path::new(&self.root_dir_path)
			.join("db");

		Ok(path)		
	}

	pub fn gen_using_db_dir_path(&self, db_name: &str) -> Result<PathBuf, Error> {
		if self.root_dir_path.as_os_str().is_empty() ||
		   db_name.len() == 0 {
		   	return err_runtime!("invalid state in gen db dir path");
		}

		let path = Path::new(&self.root_dir_path)
			.join("db")
			.join(db_name);

		Ok(path)		
	}

	pub fn gen_using_db_tables_path(&self) -> Result<PathBuf, Error> {
		if self.root_dir_path.as_os_str().is_empty() ||
		   self.using_db_name.len() == 0 {
		   	return err_runtime!("invalid state in gen using db dir path");
		}

		let path = Path::new(&self.root_dir_path)
			.join("db")
			.join(&self.using_db_name)
			.join("table");
		if path.as_os_str().to_string_lossy().contains("..") {
			return err_runtime!("directory traversal error");
		}

		Ok(path)		
	}

	pub fn gen_id_file_path(&self, table_name: &str, typ: &HeaderType) -> Result<PathBuf, Error> {
		if self.root_dir_path.as_os_str().is_empty() ||
		   self.using_db_name.len() == 0 {
		   	return err_runtime!("invalid state in gen using db dir path");
		}

		let path = Path::new(&self.root_dir_path)
			.join("db")
			.join(&self.using_db_name)
			.join("id")
			.join(format!("{}__{}.txt", table_name, typ.ident));
		if path.as_os_str().to_string_lossy().contains("..") {
			return err_runtime!("directory traversal error");
		}

		Ok(path)		
	}

	pub fn gen_table_file_path(&self, table_name: &str) -> Result<PathBuf, Error> {
		if self.root_dir_path.as_os_str().is_empty() ||
		   self.using_db_name.len() == 0 {
		   	return err_runtime!("invalid state in gen table file path");
		}

		let path = Path::new(&self.root_dir_path)
			.join("db")
			.join(&self.using_db_name)
			.join("table")
			.join(table_name.to_lowercase() + ".csv");
		if path.as_os_str().to_string_lossy().contains("..") {
			return err_runtime!("directory traversal error");
		}

		Ok(path)
	}

	pub fn gen_tmp_table_file_path(&self, table_name: &str) -> Result<PathBuf, Error> {
		if self.root_dir_path.as_os_str().is_empty() ||
		   self.using_db_name.len() == 0 {
		   	return err_runtime!("invalid state in gen table file path");
		}

		let path = Path::new(&self.root_dir_path)
			.join("db")
			.join(&self.using_db_name)
			.join("table")
			.join(table_name.to_lowercase() + ".tmp.csv");
		if path.as_os_str().to_string_lossy().contains("..") {
			return err_runtime!("directory traversal error");
		}

		Ok(path)
	}
}
