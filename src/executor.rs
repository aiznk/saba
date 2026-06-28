use crate::error::{Error, make_error, err_exec};
use crate::planner;
use crate::context::{Context};
use std::path::{Path};
use std::fs;
use std::io::{Write};

pub fn exec(context: &mut Context, node: &planner::PlansNode) -> Result<(), Error> {
	for plan in node.plans.iter() {
		exec_plan(context, &plan)?
	}
	Ok(())
}

pub fn exec_plan(context: &mut Context, node: &planner::PlanNode) -> Result<(), Error> {
	if !node.use_db.is_none() {
		if let Some(use_db) = &node.use_db {
			exec_use_db(context, &use_db)?;
		}
	} else if !node.project.is_none() {
		if let Some(project) = &node.project {
			exec_project(context, &project)?;
		}
	} else if !node.dir_create.is_none() {
		if let Some(dir_create) = &node.dir_create {
			exec_dir_create(context, &dir_create)?;
		}
	} else if !node.csv_file_create.is_none() {
		if let Some(csv_file_create) = &node.csv_file_create {
			exec_csv_file_create(context, &csv_file_create)?;
		}
	}
	Ok(())
}

pub fn exec_use_db(context: &mut Context, node: &planner::UseDatabaseNode) -> Result<(), Error> {
	context.using_db_name = node.db_name.clone();
	Ok(())
}

pub fn exec_project(context: &mut Context, node: &planner::ProjectNode) -> Result<(), Error> {
	Ok(())
}

pub fn exec_dir_create(context: &mut Context, node: &planner::DirCreateNode) -> Result<(), Error> {
	let path = Path::new(&context.root_dir_path);
	let path = path.join(&node.dir_name);
	if !path.exists() {
		match fs::create_dir(path) {
			Ok(_) => {},
			Err(e) => return err_exec!("failed to create directory. {}", e),
		}
	}
	Ok(())
}

pub fn exec_csv_file_create(context: &mut Context, node: &planner::CsvFileCreateNode) -> Result<(), Error> {
	let table_name = node.table_name.to_lowercase() + ".csv";
	let path = Path::new(&context.root_dir_path);
	let path = path.join(&context.using_db_name);
	let path = path.join(table_name);

	if !path.exists() {
		let header = &node.csv_head_row;

		let mut file = match fs::File::create(path) {
			Ok(v) => v,
			Err(e) => return err_exec!("failed to create CSV file. {}", e),
		};
		match file.write_all(header.as_bytes()) {
			Ok(_) => {},
			Err(e) => return err_exec!("failed to write CSV file. {}", e),
		}
	} else {
		return err_exec!("{} table already exists", node.table_name);
	}

	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::tokenizer::{Token, TokenStream, tokenize};
	use crate::parser::{QueryNode, parse};
	use crate::planner::{PlansNode, planning};

	fn remove_file<P: AsRef<Path>>(path: P) {
		if path.as_ref().exists() {
			fs::remove_file(path).unwrap();
		}
	}

	fn do_exec(context: &mut Context, query: &str) {
		let tests_dir = Path::new("test_env");
		if !tests_dir.exists() {
			fs::create_dir(tests_dir).unwrap();
		}
		context.root_dir_path = String::from("test_env");
		let tokens: Vec<Token> = tokenize(query.to_string()).unwrap();
		let mut tok_strm = TokenStream::new(tokens);
		let node: QueryNode = parse(&mut tok_strm).unwrap();
		let node: PlansNode = planning(&node).unwrap();
		exec(context, &node).unwrap();
	}

	#[test]
	fn test_use_db() {
		let mut context = Context::new();
		do_exec(&mut context, "USE hige");
		assert!(context.using_db_name == "hige");
	}

	#[test]
	fn test_dir_create() {
		let path = Path::new("test_env").join("mydb");
		if path.exists() {
			fs::remove_dir_all(&path).unwrap();
		}
		let mut context = Context::new();
		do_exec(&mut context, "CREATE DATABASE mydb");
		assert!(path.exists());
	}

	#[test]
	fn test_csv_file_create() {
		let path = Path::new("test_env").join("mydb").join("mytable.csv");
		remove_file(&path);
		let mut context = Context::new();
		do_exec(&mut context, "CREATE DATABASE mydb");
		do_exec(&mut context, "USE mydb");
		do_exec(&mut context, "CREATE TABLE MyTable (id: I64, weight: F64)");
		assert!(path.exists());
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: I64,weight: F64\n");
	}
}
