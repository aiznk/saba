use crate::error::Error::Exec;
use crate::error::{Error, make_error, err_exec, err_parse, err_planning};
use crate::parser::{self, ParenIdentsNode};
use crate::planner::*;
use crate::tokenizer::{TokenKind};
use crate::context::{Context};
use crate::objects::{Object, ObjectKind, HeaderType, Table};
use std::path::{Path, PathBuf};
use std::fs;
use std::fs::{OpenOptions};
use std::io::{Write};
use csv::{Reader, Writer, StringRecord};
use regex::Regex;

pub fn exec(context: &mut Context, node: &mut PlansNode) -> Result<(), Error> {
	for plan in node.plans.iter_mut() {
		context.clear();
		exec_plan(context, plan)?
	}
	Ok(())
}

pub fn exec_plan(context: &mut Context, node: &mut PlanNode) -> Result<(), Error> {
	if let Some(use_db) = &node.use_db {
		exec_use_db(context, &use_db)?;
	} else if let Some(desc_table) = &node.desc_table {
		exec_desc_table(context, &desc_table)?;
	} else if let Some(sort) = node.sort.as_mut() {
		exec_sort(context, sort)?;
	} else if let Some(project) = node.project.as_mut() {
		while exec_project(context, project)? {
		}
	} else if let Some(aggregate) = node.aggregate.as_mut() {
		exec_aggregate(context, aggregate)?;
	} else if let Some(database_create) = &node.database_create {
		exec_database_create(context, &database_create)?;
	} else if let Some(dir_list) = &node.dir_list {
		exec_dir_list(context, &dir_list)?;
	} else if let Some(dir_delete_all) = &node.dir_delete_all {
		exec_dir_delete_all(context, &dir_delete_all)?;
	} else if let Some(csv_file_append) = &node.csv_file_append {
		exec_csv_file_append(context, &csv_file_append)?;
	} else if let Some(csv_file_create) = &node.csv_file_create {
		exec_csv_file_create(context, &csv_file_create)?;
	} else if let Some(csv_file_delete) = &node.csv_file_delete {
		exec_csv_file_delete(context, &csv_file_delete)?;
	} else if let Some(csv_file_rewrite) = node.csv_file_rewrite.as_mut() {
		exec_csv_file_rewrite(context, csv_file_rewrite)?;
	} else if let Some(csv_file_rename) = &node.csv_file_rename {
		exec_csv_file_rename(context, &csv_file_rename)?;
	} else {
		return err_exec!("invalid state: exec plan");
	}

	Ok(())
}

fn string_record_to_vev_string(record: &StringRecord) -> Vec<String> {
	let mut v: Vec<String> = vec![];

	for col in record.iter() {
		v.push(col.to_string());
	}

	v
}

fn exec_sort(context: &mut Context, sort: &mut SortNode) -> Result<(), Error> {
	let mut obj = Object::new();

	context.current_table_name = sort.table_name.clone();
	assert!(context.current_table_name.len() > 0);

	if let Some(expr) = &sort.expr {
		obj = exec_expr(context, expr)?;
		if obj.kind != ObjectKind::Ident {
			return err_exec!("not ident: order by");
		}
	}

	if let Some(project) = sort.project.as_mut() {
		let mut records: Vec<StringRecord> = vec![];
		let mut limit_value = None;
		let is_cli = context.is_cli;
		context.is_cli = false;

		if let Some(limit) = &project.limit {
			limit_value = gen_limit_value(context, limit)?;
		}

		let mut project = (*project).clone();
		project.limit = None;

		while exec_project(context, &mut project)? {
			if context.skip {
				continue;
			}
			if context.filtered {
				if context.matched {
					records.push(context.matched_record.clone());
				}
			} else {
				records.push(context.get_current_table_scanned_record()?);
			}
		}

		context.is_cli = is_cli;

		let index = context.get_table_header_idents(&sort.table_name)?.iter().position(|s| *s == obj.ident);
		if index.is_none() {
			return err_exec!("not found '{}' ident in sort", obj.ident);
		}
		let index = index.unwrap();

		if sort.is_asc {
			records.sort_by(|a, b| {
				a[index].cmp(&b[index])
			});
		} else {
			records.sort_by(|a, b| {
				b[index].cmp(&a[index])
			});
		}

		if let Some(limit_value) = limit_value {
			records.truncate(limit_value as usize);
		}

		if !sort.all && records.len() > 0 {
			let first = records.first().unwrap().clone();
			records.clear();
			records.push(first.clone());
			context.selected_csv_columns = string_record_to_vev_string(&first);
		}

		if let Some(test_get_records) = context.test_get_records.as_mut() {
			*test_get_records = records.clone();
		}

		if context.is_cli {
			for rec in records.iter() {
				print_string_record(&rec)?;
			}
		}
	}

	Ok(())
}

fn exec_desc_table(context: &mut Context, node: &DescTableNode) -> Result<(), Error> {
	if let Some(table_name) = &node.table_name {
		let headers = read_table_headers(context, &table_name)?;
		for header in headers.iter() {
			println!("{}", header);
		}
	}
	Ok(())
}

pub fn gen_default_record(headers: &StringRecord) -> Result<Vec<String>, Error> {
	let mut v: Vec<String> = vec![];
	let types = parse_csv_headers_as_types(headers)?;

	for i in 0..types.len() {
		let typ = &types[i];
		v.push(typ.to_default_value_string()?);
	}

	Ok(v)
}

fn csv_headers_to_idents(headers: &StringRecord) -> Vec<String> {
	let mut header_idents: Vec<String> = vec![];
	for header in headers.iter() {
		if let Some((left, _right)) = header.split_once(":") {
			header_idents.push(left.trim().to_string());
		}
	}
	header_idents
}

pub fn find_header_position(headers: &StringRecord, col_name: &str) -> Result<Option<usize>, Error> {
	let header_idents = csv_headers_to_idents(headers);

	if let Some(index) = header_idents.iter().position(|s| *s == col_name) {
		return Ok(Some(index));
	}

	Ok(None)
}

fn open_append_writer(path: &PathBuf) -> Result<Writer<fs::File>, Error> {
	let file = match OpenOptions::new()
        .append(true)
        .create(true)
        .open(path) {
    	Ok(v) => v,
    	Err(e) => return err_exec!("failed to open file on append: {}", e),
    };
    let writer = Writer::from_writer(file);
    Ok(writer)
}

fn rewrite_append_record_by_vars(context: &mut Context, node: &CsvFileAppendNode, headers: &StringRecord, row: &mut Vec<String>) -> Result<(), Error> {
	if let Some(expr_list) = &node.expr_list {
		let objs = exec_expr_list(context, &expr_list)?;
		for obj in objs.iter() {
			let key = obj.to_string();
			if let Some(o) = context.vars.get(key.as_str()) {
				if let Some(index) = find_header_position(&headers, key.as_str())? {
					row[index] = o.to_string();
				} else {
					return err_exec!("invalid column: {} in append record", key);
				}
			} else {
				return err_exec!("failed to get value of vars");
			}
		}
	}

	Ok(())
}

fn next_id(path: impl AsRef<Path>) -> std::io::Result<u64> {
    let path = path.as_ref();

    let id = match fs::read_to_string(path) {
        Ok(s) => s.trim().parse::<u64>().unwrap_or(1),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => 1,
        Err(e) => return Err(e),
    };

    fs::write(path, (id + 1).to_string())?;

    // 元のIDを返す
    Ok(id)
}

fn gen_auto_increment_id(context: &mut Context, table_name: &String, typ: &HeaderType) -> Result<u64, Error> {
	let path = context.gen_id_file_path(table_name, typ)?;
	let id = match next_id(&path) {
		Ok(v) => v,
		Err(e) => return err_exec!("failed to generate auto increment id. {}", e),
	};
	Ok(id)
}

fn set_auto_increment_ids(context: &mut Context, table_name: &String, headers: &StringRecord, row: &mut Vec<String>) -> Result<(), Error> {
	let types = parse_csv_headers_as_types(headers)?;

	for (i, typ) in types.iter().enumerate() {
		if !typ.is_auto_increment {
			continue;
		}
		if !typ.is_int {
			return err_exec!("cannot auto increment. id must be int");
		}
		let id = gen_auto_increment_id(context, table_name, &typ)?;
		row[i] = id.to_string();
	}
	
	Ok(())	
}

fn parse_csv_headers_as_types(headers: &StringRecord) -> Result<Vec<HeaderType>, Error> {
	let mut v: Vec<HeaderType> = vec![];

	for header in headers.iter() {
		if let Some((left, right)) = header.split_once(":") {
			let mut typ = HeaderType::new();
			typ.ident = left.trim().to_string();
			let stype = right.to_lowercase();

			if stype.contains("int") {
				typ.is_int = true;
			} else if stype.contains("float") {
				typ.is_float = true;
			} else if stype.contains("bool") {
				typ.is_bool = true;
			} else if stype.contains("char") {
				typ.is_char = true;
				let re = Regex::new(r"char\s*\[\s*(\d+)\s*\]").unwrap();
				if let Some(caps) = re.captures(stype.as_str()) {
					let size: usize = match caps[1].parse() {
						Ok(v) => v,
						Err(e) => return err_exec!("failed to parse CHAR[n]. {}", e),
					};
					typ.char_size = size;
				} else {
					return err_exec!("failed to capture stype for char");
				}
			}
			if stype.contains("auto_increment") {
				typ.is_auto_increment = true;
			}
			if stype.contains("primary_key") {
				typ.is_primary_key = true;
			}
			if stype.contains("default") {
				typ.is_default = true;
				let re = Regex::new(
						    r#"default\s+(?:(?P<float>-?\d+\.\d+)|(?P<int>-?\d+)|"(?P<string>[^"]*)"|(?P<bool>true|false))"#
							).unwrap();
				if let Some(cap) = re.captures(stype.as_str()) {
				    if let Some(v) = cap.name("int") {
				    	typ.default_value = Some(Object::from_int(v.as_str().parse::<i128>().unwrap()));
				    } else if let Some(v) = cap.name("float") {
				    	typ.default_value = Some(Object::from_float(v.as_str().parse::<f64>().unwrap()));
				    } else if let Some(v) = cap.name("string") {
				    	typ.default_value = Some(Object::from_string(v.as_str().to_string()));
				    } else if let Some(v) = cap.name("bool") {
				    	typ.default_value = Some(Object::from_bool(v.as_str().parse::<bool>().unwrap()));
				    } else {
				    	return err_exec!("invalid default value");
				    }
				} else {
					return err_exec!("failed to capture stype for default");
				}
			}

			v.push(typ);
		} else {
			return err_exec!("failed to split header. maybe this is invalid header");
		}
	}

	Ok(v)
}

fn check_invalid_append_record(record: &Vec<String>, headers: &StringRecord) -> Result<(), Error> {
	let types = parse_csv_headers_as_types(headers)?;
	
	if types.len() != record.len() {
		return err_exec!("invalid record length");
	}

	for i in 0..record.len() {
		let typ = &types[i];
		let col = &record[i];
		match typ.parse_str(col) {
			Ok(_) => {},
			Err(e) => return err_exec!("failed to parse column by type. invalid column found. {}", e),
		}
	}

	Ok(())
}

pub fn exec_csv_file_append(context: &mut Context, node: &CsvFileAppendNode) -> Result<(), Error> {
	let path = context.gen_table_file_path(&node.table_name)?;
    let mut writer = open_append_writer(&path)?;
    let headers = read_table_headers(context, &node.table_name)?;

    if node.expr_list.is_some() {
		let mut row: Vec<String> = gen_default_record(&headers)?;
	    rewrite_append_record_by_vars(context, node, &headers, &mut row)?;
	    set_auto_increment_ids(context, &node.table_name, &headers, &mut row)?;
	    check_invalid_append_record(&row, &headers)?;

		match writer.write_record(&row) {
			Ok(_) => {},
			Err(e) => return err_exec!("failed to write CSV row {}", e),
		}

	} else if node.paren_values_list.len() > 0 {
		let mut ident_objs: Option<Vec<Object>> = None;
		let mut value_objs_list = vec![];
		let header_idents = parse_header_idents(&headers)?;

		if let Some(paren_idents) = &node.paren_idents {
			ident_objs = Some(exec_paren_idents(context, paren_idents)?);
		}	

		for paren_values in node.paren_values_list.iter() {
			let value_objs = exec_paren_values(context, paren_values)?;
			value_objs_list.push(value_objs);
		}

		if let Some(ident_objs) = ident_objs {
			for value_objs in value_objs_list.iter() {
				if value_objs.len() != ident_objs.len() {
					return err_exec!("not same length values list");
				}
			}

			let mut indices: Vec<usize> = vec![];

			for ident_obj in ident_objs.iter() {
				let ident = ident_obj.to_string();
				let index = header_idents.iter().position(|s| *s == ident);
				if index.is_none() {
					return err_exec!("not found ident '{}' in add stmt", ident);
				}
				let index = index.unwrap();
				indices.push(index);
			}

			for value_objs in value_objs_list.iter() {
				let mut row: Vec<String> = gen_default_record(&headers)?;

				for (i, index) in indices.iter().enumerate() {
					let value = value_objs[i].to_string();
					row[*index] = value;
				}

			    set_auto_increment_ids(context, &node.table_name, &headers, &mut row)?;
			    check_invalid_append_record(&row, &headers)?;

				match writer.write_record(&row) {
					Ok(_) => {},
					Err(e) => return err_exec!("failed to write CSV row (2). {}", e),
				}
			}
		} else {
			for value_objs in value_objs_list.iter() {
				let mut row: Vec<String> = gen_default_record(&headers)?;
				if value_objs.len() > row.len() {
					return err_exec!("does not match length: add stmt");
				}

				for (i, value_obj) in value_objs.iter().enumerate() {
					let value = value_obj.to_string();
					row[i] = value;
				}

			    set_auto_increment_ids(context, &node.table_name, &headers, &mut row)?;
			    check_invalid_append_record(&row, &headers)?;

				match writer.write_record(&row) {
					Ok(_) => {},
					Err(e) => return err_exec!("failed to write CSV row (3). {}", e),
				}
			}
		}
	} else {
		let mut row: Vec<String> = gen_default_record(&headers)?;
	    set_auto_increment_ids(context, &node.table_name, &headers, &mut row)?;
	    check_invalid_append_record(&row, &headers)?;

		match writer.write_record(&row) {
			Ok(_) => {},
			Err(e) => return err_exec!("failed to write CSV row {}", e),
		}
	}
	Ok(())
}

pub fn exec_use_db(context: &mut Context, node: &UseDatabaseNode) -> Result<(), Error> {
	if node.db_name.contains("..") {
		return err_exec!("directory traversal error");
	}
	context.using_db_name = node.db_name.clone();
	let path = context.gen_db_dir_path(&context.using_db_name)?;
	if !path.exists() {
		return err_exec!("{} does not exists database", context.using_db_name);
	}
	Ok(())
}

fn print_string_record(record: &StringRecord) -> Result<(), Error> {
	if record.len() == 0 {
		return Ok(());
	}

	let mut s = String::new();

	for col in record.iter() {
		s.push_str(col);
		s.push_str(",");
	}
	s.pop();
	println!("{}", s);	
	Ok(())
}

pub fn print_selected_columns(context: &mut Context) -> Result<(), Error> {
	if context.selected_csv_columns.len() == 0 {
		return Ok(());
	}

	let mut s = String::new();

	for col in context.selected_csv_columns.iter() {
		s.push_str(col);
		s.push_str(",");
	}
	s.pop();
	println!("{}", s);

	Ok(())
}

pub fn exec_limit(context: &mut Context, node: &Box<parser::LimitNode>) -> Result<Object, Error> {
	if let Some(expr) = &node.expr {
		return Ok(exec_expr(context, expr)?);
	} else {
		return err_exec!("invalid state: limit");
	}
}

fn gen_limit_value(context: &mut Context, limit: &Box<parser::LimitNode>) -> Result<Option<i128>, Error> {
	let limit_value: Option<i128>;

	let o = exec_limit(context, limit)?;
	match o.kind {
		ObjectKind::Int => {
			limit_value = Some(o.int_value);
		}
		_ => return err_exec!("invalid limit expression"),
	}

	Ok(limit_value)
}

pub fn exec_where_clause(context: &mut Context, node: &parser::WhereClauseNode) -> Result<Object, Error> {
	if let Some(expr) = &node.expr {
		Ok(exec_expr(context, expr)?)
	} else {
		err_exec!("impossible")
	}
}

pub fn exec_expr_list(context: &mut Context, node: &parser::ExprListNode) -> Result<Vec<Object>, Error> {
	let mut v: Vec<Object> = vec![];

	for expr in node.exprs.iter() {
		let o = exec_expr(context, &expr)?;
		v.push(o);
	}

	Ok(v)
}

pub fn exec_expr(context: &mut Context, node: &parser::ExprNode) -> Result<Object, Error> {
	if let Some(ass_expr) = &node.ass_expr {
		Ok(exec_ass_expr(context, ass_expr)?)
	} else {
		err_exec!("impossible")
	}
}

pub fn exec_ass_expr(context: &mut Context, node: &parser::AssExprNode) -> Result<Object, Error> {
	let lhs: Object;
	let rhs: Object;

	if let Some(func_expr) = &node.left_func_expr {
		lhs = exec_func_expr(context, func_expr)?;
	} else {
		return err_exec!("impossible");
	}

	if node.right_func_expr.is_none() {
		return Ok(lhs);
	}

	if let Some(func_expr) = &node.right_func_expr {
		rhs = exec_func_expr(context, func_expr)?;
	} else {
		return err_exec!("impossible");
	}

	match lhs.kind {
		ObjectKind::Ident => {
			let key = lhs.ident.clone();
			context.vars.insert(key, Box::new(rhs));
		},
		_ => return err_exec!("can't assign to primitive object: {:?}", lhs),
	}

	Ok(lhs)
}

fn call_count(context: &mut Context, args: &Vec<Object>) -> Result<Object, Error> {
	let record;
	if context.skip {
		return Ok(Object::from_int(context.count_counter as i128));
	}
	if context.filtered {
		if context.matched {
			record = context.matched_record.clone();
		} else {
			return Ok(Object::from_int(context.count_counter as i128));
		}
	} else {
		record = context.get_current_table_scanned_record()?;
	}

	if args.len() != 1 {
		return err_exec!("invalid args length in count function");
	}
	let arg = &args[0];

	if arg.kind == ObjectKind::Star {
		// pass
	} else {
		let ident = arg.to_string();
		let header_idents = context.get_table_header_idents(context.current_table_name.clone().as_str())?;
		let Some(index) = header_idents.iter().position(|s| *s == ident) else {
			return err_exec!("invalid column '{}' in count", ident);
		};
		if index >= record.len() {
			return err_exec!("index out of range in count function");
		}
		let _ = &record[index];
	}
	
	context.count_counter += 1;

	return Ok(Object::from_int(context.count_counter as i128));
}

fn call_avg(context: &mut Context, args: &Vec<Object>) -> Result<Object, Error> {
	let record;
	if context.skip {
		if context.avg_counter == 0 {
			return Ok(Object::from_float(context.avg_sum_value));
		}
		return Ok(Object::from_float(context.avg_sum_value / context.avg_counter as f64));
	}
	if context.filtered {
		if context.matched {
			record = context.matched_record.clone();
			context.avg_counter += 1;
		} else {
			if context.avg_counter == 0 {
				return Ok(Object::from_float(context.avg_sum_value));
			}
			return Ok(Object::from_float(context.avg_sum_value / context.avg_counter as f64));
		}
	} else {
		record = context.get_current_table_scanned_record()?;
		context.avg_counter += 1;
	}

	if args.len() != 1 {
		return err_exec!("invalid args length in avg function");
	}
	let arg = &args[0];

	if arg.kind == ObjectKind::Star {
		return err_exec!("can't use star in avg function");
	} else {
		let ident = arg.to_string();
		let types = context.get_table_header_types(context.current_table_name.clone().as_str())?;
		let header_idents = context.get_table_header_idents(context.current_table_name.clone().as_str())?;
		let Some(index) = header_idents.iter().position(|s| *s == ident) else {
			return err_exec!("invalid column '{}' in avg", ident);
		};
		if index >= record.len() {
			return err_exec!("index out of range in avg function");
		}
		let typ = &types[index];
		let field = &record[index];
		let obj = typ.parse_str(field)?;
		match obj.kind {
			ObjectKind::Int => {
				context.avg_sum_value += obj.int_value as f64;
			}
			ObjectKind::Float => {
				context.avg_sum_value += obj.float_value;
			}
			_ => { return err_exec!("invalid value type in avg function"); }
		}
	}

	if context.avg_counter == 0 {
		return Ok(Object::from_float(context.avg_sum_value));
	}
	return Ok(Object::from_float(context.avg_sum_value / context.avg_counter as f64));
}

fn call_sum(context: &mut Context, args: &Vec<Object>) -> Result<Object, Error> {
	let record;
	if context.skip {
		return Ok(Object::from_float(context.sum_value));
	}
	if context.filtered {
		if context.matched {
			record = context.matched_record.clone();
		} else {
			return Ok(Object::from_float(context.sum_value));
		}
	} else {
		record = context.get_current_table_scanned_record()?;
	}

	if args.len() != 1 {
		return err_exec!("invalid args length in sum function");
	}
	let arg = &args[0];

	if arg.kind == ObjectKind::Star {
		return err_exec!("can't use star in sum function");
	} else {
		let ident = arg.to_string();
		let types = context.get_table_header_types(context.current_table_name.clone().as_str())?;
		let header_idents = context.get_table_header_idents(context.current_table_name.clone().as_str())?;
		let Some(index) = header_idents.iter().position(|s| *s == ident) else {
			return err_exec!("invalid column '{}' in sum", ident);
		};
		if index >= record.len() {
			return err_exec!("index out of range in sum function");
		}
		let typ = &types[index];
		let field = &record[index];
		let obj = typ.parse_str(field)?;
		match obj.kind {
			ObjectKind::Int => {
				context.sum_value += obj.int_value as f64;
			}
			ObjectKind::Float => {
				context.sum_value += obj.float_value as f64;
			}
			_ => { return err_exec!("invalid value type in sum function"); }
		}
	}

	return Ok(Object::from_float(context.sum_value));
}

fn call_max(context: &mut Context, args: &Vec<Object>) -> Result<Object, Error> {
	let record;
	if context.skip {
		return Ok(Object::from_float(context.max_value));
	}
	if context.filtered {
		if context.matched {
			record = context.matched_record.clone();
		} else {
			return Ok(Object::from_float(context.max_value));
		}
	} else {
		record = context.get_current_table_scanned_record()?;
	}

	if args.len() != 1 {
		return err_exec!("invalid args length in max function");
	}
	let arg = &args[0];

	if arg.kind == ObjectKind::Star {
		return err_exec!("can't use star in max function");
	} else {
		let ident = arg.to_string();
		let types = context.get_table_header_types(context.current_table_name.clone().as_str())?;
		let header_idents = context.get_table_header_idents(context.current_table_name.clone().as_str())?;
		let Some(index) = header_idents.iter().position(|s| *s == ident) else {
			return err_exec!("invalid column '{}' in max", ident);
		};
		if index >= record.len() {
			return err_exec!("index out of range in max function");
		}
		let typ = &types[index];
		let field = &record[index];
		let obj = typ.parse_str(field)?;
		match obj.kind {
			ObjectKind::Int => {
				context.max_value = context.max_value.max(obj.int_value as f64);
			}
			ObjectKind::Float => {
				context.max_value = context.max_value.max(obj.float_value);
			}
			_ => { return err_exec!("invalid value type in max function"); }
		}
	}

	return Ok(Object::from_float(context.max_value));
}

fn call_min(context: &mut Context, args: &Vec<Object>) -> Result<Object, Error> {
	let record;
	if context.skip {
		return Ok(Object::from_float(context.min_value));
	}
	if context.filtered {
		if context.matched {
			record = context.matched_record.clone();
		} else {
			return Ok(Object::from_float(context.min_value));
		}
	} else {
		record = context.get_current_table_scanned_record()?;
	}

	if args.len() != 1 {
		return err_exec!("invalid args length in min function");
	}
	let arg = &args[0];

	if arg.kind == ObjectKind::Star {
		return err_exec!("can't use star in min function");
	} else {
		let ident = arg.to_string();
		let types = context.get_table_header_types(context.current_table_name.clone().as_str())?;
		let header_idents = context.get_table_header_idents(context.current_table_name.clone().as_str())?;
		let Some(index) = header_idents.iter().position(|s| *s == ident) else {
			return err_exec!("invalid column '{}' in min", ident);
		};
		if index >= record.len() {
			return err_exec!("index out of range in min function");
		}
		let typ = &types[index];
		let field = &record[index];
		let obj = typ.parse_str(field)?;
		match obj.kind {
			ObjectKind::Int => {
				context.min_value = context.min_value.min(obj.int_value as f64);
			}
			ObjectKind::Float => {
				context.min_value = context.min_value.min(obj.float_value);
			}
			_ => { return err_exec!("invalid value type in min function"); }
		}
	}

	return Ok(Object::from_float(context.min_value));
}

fn call_func(context: &mut Context, func_name: &Object, args: &Vec<Object>) -> Result<Object, Error> {
	if func_name.kind != ObjectKind::Ident {
		return err_exec!("function name was not ident");
	}
	let func_name = func_name.ident.to_lowercase();

	match func_name.as_str() {
		"count" => { return call_count(context, args); }
		"sum" => { return call_sum(context, args); }
		"avg" => { return call_avg(context, args); }
		"min" => { return call_min(context, args); }
		"max" => { return call_max(context, args); }
		&_ => return err_exec!("unknown function name '{}'", func_name),
	}
}

pub fn exec_func_expr(context: &mut Context, node: &parser::FuncExprNode) -> Result<Object, Error> {
	if let Some(ident) = &node.ident {
		let ident_obj = exec_ident(context, ident)?;
		let mut arg_objs = vec![];

		for or_expr in node.or_exprs.iter() {
			let obj = exec_or_expr(context, or_expr)?;
			arg_objs.push(obj);
		}

		return Ok(call_func(context, &ident_obj, &arg_objs)?);
	} else if let Some(or_expr) = &node.or_expr {
		return Ok(exec_or_expr(context, or_expr)?);
	}
	
	return err_exec!("invalid state: func expr");
}

pub fn or_objects(context: &mut Context, a: &Object, b: &Object) -> Result<Object, Error> {
	match a.kind {
		ObjectKind::Bool => {
			match b.kind {
				ObjectKind::Bool => {
					let n = a.bool_value || b.bool_value;
					Ok(Object::from_bool(n))
				}
				ObjectKind::Ident => {
					let bo = refer_ident(context, &b)?;
					Ok(or_objects(context, a, &bo)?)
				}
				_ => return err_exec!("can't compare or"),
			}
		}
		ObjectKind::Ident => {
			match b.kind {
				ObjectKind::Bool => {
					let ao = refer_ident(context, &a)?;
					Ok(or_objects(context, &ao, b)?)
				}
				ObjectKind::Ident => {
					let ao = refer_ident(context, &a)?;
					let bo = refer_ident(context, &b)?;
					Ok(or_objects(context, &ao, &bo)?)
				}
				_ => return err_exec!("can't compare or"),
			}
		}
		_ => return err_exec!("can't compare or (2)"),
	}	
}

pub fn and_objects(context: &mut Context, a: &Object, b: &Object) -> Result<Object, Error> {
	match a.kind {
		ObjectKind::Bool => {
			match b.kind {
				ObjectKind::Bool => {
					let n = a.bool_value && b.bool_value;
					Ok(Object::from_bool(n))
				}
				ObjectKind::Ident => {
					let bo = refer_ident(context, &b)?;
					Ok(or_objects(context, a, &bo)?)
				}
				_ => return err_exec!("can't compare or"),
			}
		}
		ObjectKind::Ident => {
			match b.kind {
				ObjectKind::Bool => {
					let ao = refer_ident(context, &a)?;
					Ok(or_objects(context, &ao, b)?)
				}
				ObjectKind::Ident => {
					let ao = refer_ident(context, &a)?;
					let bo = refer_ident(context, &b)?;
					Ok(or_objects(context, &ao, &bo)?)
				}
				_ => return err_exec!("can't compare or"),
			}
		}
		_ => return err_exec!("can't compare or (2)"),
	}	
}

pub fn exec_or_expr(context: &mut Context, node: &parser::OrExprNode) -> Result<Object, Error> {
	let mut a;
	let mut b;
	let mut c;

	let lhs = &node.nodes[0];
	a = exec_and_expr(context, &*lhs)?;
	c = a.clone();

	if a.kind == ObjectKind::Bool &&
	   a.bool_value {
	   	return Ok(a);
	}

	for i in 1..node.nodes.len() {
		let rhs = &node.nodes[i];

		b = exec_and_expr(context, &*rhs)?;
		c = or_objects(context, &a, &b)?;
		a = c.clone();
		if c.kind == ObjectKind::Bool &&
		   c.bool_value {
		   	break;
		}
	}

	Ok(c)
}

pub fn exec_and_expr(context: &mut Context, node: &parser::AndExprNode) -> Result<Object, Error> {
	let mut a;
	let mut b;
	let mut c;

	let lhs = &node.nodes[0];
	a = exec_compare_expr(context, &*lhs)?;
	c = a.clone();

	for i in 1..node.nodes.len() {
		let rhs = &node.nodes[i];
		b = exec_compare_expr(context, &*rhs)?;
		c = and_objects(context, &a, &b)?;
		a = c.clone();
	}

	Ok(c)
}

pub fn exec_compare_expr(context: &mut Context, node: &parser::CompareExprNode) -> Result<Object, Error> {
	let mut a;
	let mut b;
	let mut c;

	if let parser::CompareExprItemNode::Left(operand) = &node.nodes[0] {
		a = exec_add_sub_expr(context, &*operand)?;	
	} else {
		return err_exec!("impossible");
	}

	c = a.clone();

	for i in (1..node.nodes.len()).step_by(2) {
		let op = &node.nodes[i];
		let rhs = &node.nodes[i+1];

		if let parser::CompareExprItemNode::Right(operand) = rhs {
			b = exec_add_sub_expr(context, &*operand)?;
		} else {
			return err_exec!("impossible");
		}

		if let parser::CompareExprItemNode::Op(compare_op) = op {
			c = compare_objects(context, &a, &compare_op, &b)?;
			a = c.clone();
		} else {
			return err_exec!("impossible");
		}		
	}

	Ok(c)
}

pub fn exec_add_sub_expr(context: &mut Context, node: &parser::AddSubExprNode) -> Result<Object, Error> {
	let mut a;
	let mut b;
	let mut c;

	if let parser::AddSubExprItemNode::Left(left) = &node.nodes[0] {
		a = exec_mul_div_expr(context, &*left)?;	
	} else {
		return err_exec!("impossible");
	}

	c = a.clone();

	for i in (1..node.nodes.len()).step_by(2) {
		let op = &node.nodes[i];
		let rhs = &node.nodes[i+1];

		if let parser::AddSubExprItemNode::Right(right) = rhs {
			b = exec_mul_div_expr(context, &*right)?;
		} else {
			return err_exec!("impossible");
		}

		if let parser::AddSubExprItemNode::Op(op) = op {
			c = add_sub_objects(context, &a, &op, &b)?;
			a = c.clone();
		} else {
			return err_exec!("impossible");
		}		
	}

	Ok(c)
}

pub fn exec_mul_div_expr(context: &mut Context, node: &parser::MulDivExprNode) -> Result<Object, Error> {
	let mut a;
	let mut b;
	let mut c;

	if let parser::MulDivExprItemNode::Left(dot_chain) = &node.nodes[0] {
		a = exec_dot_chain(context, &*dot_chain)?;	
	} else {
		return err_exec!("impossible");
	}

	c = a.clone();

	for i in (1..node.nodes.len()).step_by(2) {
		let op = &node.nodes[i];
		let rhs = &node.nodes[i+1];

		if let parser::MulDivExprItemNode::Right(dot_chain) = rhs {
			b = exec_dot_chain(context, &*dot_chain)?;
		} else {
			return err_exec!("impossible");
		}

		if let parser::MulDivExprItemNode::Op(op) = op {
			c = mul_div_objects(context, &a, &op, &b)?;
			a = c.clone();
		} else {
			return err_exec!("impossible");
		}		
	}

	Ok(c)
}

pub fn exec_dot_chain(context: &mut Context, node: &parser::DotChainNode) -> Result<Object, Error> {
	let mut o = exec_operand(context, &node.nodes[0])?;

	for i in 1..node.nodes.len() {
		let mut child = exec_operand(context, &node.nodes[i])?;
		child.parent = Some(Box::new(o));
		o = child;
	}

	Ok(o)
}

pub fn parse_column_by_head(head: &str, col: &str) -> Result<Object, Error> {
	let head = head.to_lowercase();

	if head.contains("int") {
		let n = match col.parse::<i128>() {
			Ok(v) => v,
			Err(e) => return err_exec!("failed to parse column as i64. {}", e),
		};
		return Ok(Object::from_int(n));
	} else if head.contains("float") {
		let n = match col.parse::<f64>() {
			Ok(v) => v,
			Err(e) => return err_exec!("failed to parse column as f64. {}", e),
		};
		return Ok(Object::from_float(n));
	} else if head.contains("bool") {
		let n = match col.parse::<bool>() {
			Ok(v) => v,
			Err(e) => return err_exec!("failed to parse column as bool. {}", e),
		};
		return Ok(Object::from_bool(n));
	} else {
		return Ok(Object::from_string(col.to_string()));
	}

	err_exec!("can't parse column by header info")
}

pub fn refer_ident(context: &mut Context, ident: &Object) -> Result<Object, Error> {
	let table_name = if let Some(parent) = &ident.parent {
		match parent.kind {
			ObjectKind::Ident => {
				parent.ident.clone()
			}
			_ => {
				return err_exec!("invalid parent object");
			}
		}
	} else {
		context.current_table_name.clone()
	};
	let header_idents = context.get_table_header_idents(table_name.as_str())?;
	if let Some(index) = header_idents.iter().position(|s| *s == *ident.ident) {
		let head = context.get_table_headers(table_name.as_str())?[index].to_string();
		let col = context.get_table_scanned_record(table_name.as_str())?[index].to_string();
		let o = parse_column_by_head(&head, col.as_str())?;
		Ok(o)
	} else {
		err_exec!("not found ident in CSV header")
	}
}

pub fn add_sub_objects(context: &mut Context, lhs: &Object, op: &parser::AddSubOpNode, rhs: &Object) -> Result<Object, Error> {
	match op {
		parser::AddSubOpNode::Add => {
			match lhs.kind {
				ObjectKind::Int => {
					match rhs.kind {
						ObjectKind::Int => {
							let n = lhs.int_value + rhs.int_value;
							Ok(Object::from_int(n))
						}
						ObjectKind::Float => {
							let n = (lhs.int_value as f64) + rhs.float_value;
							Ok(Object::from_float(n))
						}
						_ => {
							return err_exec!("can't add: a + b");
						}
					}
				}
				ObjectKind::Float => {
					match rhs.kind {
						ObjectKind::Int => {
							let n = lhs.float_value + (rhs.int_value as f64);
							Ok(Object::from_float(n))
						}
						ObjectKind::Float => {
							let n = lhs.float_value + rhs.float_value;
							Ok(Object::from_float(n))
						}
						_ => {
							return err_exec!("can't add: a + b (2)");
						}
					}
				}
				ObjectKind::String => {
					match rhs.kind {
						ObjectKind::String => {
							let mut n = lhs.string.clone();
							n.push_str(&rhs.string);
							Ok(Object::from_string(n))
						}
						_ => {
							return err_exec!("can't add: a + b (3)");
						}
					}
				}
				ObjectKind::Ident => {
					match rhs.kind {
						ObjectKind::Int |
						ObjectKind::Float |
						ObjectKind::String => {
							let lo = refer_ident(context, &lhs)?;
							return add_sub_objects(context, &lo, op, rhs);
						}
						_ => {
							return err_exec!("can't add: a + b (4)")
						}
					}
				}
				_ => err_exec!("can't add: a + b (5)"),
			}
		}
		parser::AddSubOpNode::Sub => {
			match lhs.kind {
				ObjectKind::Int => {
					match rhs.kind {
						ObjectKind::Int => {
							let n = lhs.int_value - rhs.int_value;
							Ok(Object::from_int(n))
						}
						ObjectKind::Float => {
							let n = (lhs.int_value as f64) - rhs.float_value;
							Ok(Object::from_float(n))
						}
						_ => {
							return err_exec!("can't sub: a - b");
						}
					}
				}
				ObjectKind::Float => {
					match rhs.kind {
						ObjectKind::Int => {
							let n = lhs.float_value - (rhs.int_value as f64);
							Ok(Object::from_float(n))
						}
						ObjectKind::Float => {
							let n = lhs.float_value - rhs.float_value;
							Ok(Object::from_float(n))
						}
						_ => {
							return err_exec!("can't sub: a - b (2)");
						}
					}
				}
				ObjectKind::Ident => {
					match rhs.kind {
						ObjectKind::Int |
						ObjectKind::Float |
						ObjectKind::String => {
							let lo = refer_ident(context, &lhs)?;
							return add_sub_objects(context, &lo, op, rhs);
						}
						_ => {
							return err_exec!("can't sub: a - b (4)")
						}
					}
				}
				_ => err_exec!("can't sub: a - b (5)"),
			}
		}
	}
}

pub fn mul_div_objects(context: &mut Context, lhs: &Object, op: &parser::MulDivOpNode, rhs: &Object) -> Result<Object, Error> {
	match op {
		parser::MulDivOpNode::Mul => {
			match lhs.kind {
				ObjectKind::Int => {
					match rhs.kind {
						ObjectKind::Int => {
							let n = lhs.int_value * rhs.int_value;
							Ok(Object::from_int(n))
						}
						ObjectKind::Float => {
							let n = (lhs.int_value as f64) * rhs.float_value;
							Ok(Object::from_float(n))
						}
						_ => {
							return err_exec!("can't mul: a * b");
						}
					}
				}
				ObjectKind::Float => {
					match rhs.kind {
						ObjectKind::Int => {
							let n = lhs.float_value * (rhs.int_value as f64);
							Ok(Object::from_float(n))
						}
						ObjectKind::Float => {
							let n = lhs.float_value * rhs.float_value;
							Ok(Object::from_float(n))
						}
						_ => {
							return err_exec!("can't mul: a * b (2)");
						}
					}
				}
				ObjectKind::Ident => {
					match rhs.kind {
						ObjectKind::Int |
						ObjectKind::Float |
						ObjectKind::String => {
							let lo = refer_ident(context, &lhs)?;
							return mul_div_objects(context, &lo, op, rhs);
						}
						_ => {
							return err_exec!("can't mul: a * b (4)")
						}
					}
				}
				_ => err_exec!("can't mul: a * b (5)"),
			}
		}
		parser::MulDivOpNode::Div => {
			match lhs.kind {
				ObjectKind::Int => {
					match rhs.kind {
						ObjectKind::Int => {
							if rhs.int_value == 0 {
								return err_exec!("zero division error: a / b");
							}
							let n = lhs.int_value / rhs.int_value;
							Ok(Object::from_int(n))
						}
						ObjectKind::Float => {
							if rhs.float_value == 0.0 {
								return err_exec!("zero division error: a / b");
							}
							let n = (lhs.int_value as f64) / rhs.float_value;
							Ok(Object::from_float(n))
						}
						_ => {
							return err_exec!("can't div: a / b");
						}
					}
				}
				ObjectKind::Float => {
					match rhs.kind {
						ObjectKind::Int => {
							if rhs.int_value == 0 {
								return err_exec!("zero division error: a / b");
							}
							let n = lhs.float_value / (rhs.int_value as f64);
							Ok(Object::from_float(n))
						}
						ObjectKind::Float => {
							if rhs.float_value == 0.0 {
								return err_exec!("zero division error: a / b");
							}
							let n = lhs.float_value / rhs.float_value;
							Ok(Object::from_float(n))
						}
						_ => {
							return err_exec!("can't div: a / b (2)");
						}
					}
				}
				ObjectKind::Ident => {
					match rhs.kind {
						ObjectKind::Int |
						ObjectKind::Float |
						ObjectKind::String => {
							let lo = refer_ident(context, &lhs)?;
							return mul_div_objects(context, &lo, op, rhs);
						}
						_ => {
							return err_exec!("can't div: a / b (4)")
						}
					}
				}
				_ => err_exec!("can't div: a / b (5)"),
			}
		}
		parser::MulDivOpNode::Mod => {
			match lhs.kind {
				ObjectKind::Int => {
					match rhs.kind {
						ObjectKind::Int => {
							if rhs.int_value == 0 {
								return err_exec!("zero division error: a % b");
							}
							let n = lhs.int_value % rhs.int_value;
							Ok(Object::from_int(n))
						}
						ObjectKind::Float => {
							if rhs.float_value == 0.0 {
								return err_exec!("zero division error: a % b");
							}
							let n = (lhs.int_value as f64) % rhs.float_value;
							Ok(Object::from_float(n))
						}
						_ => {
							return err_exec!("can't mod: a % b");
						}
					}
				}
				ObjectKind::Float => {
					match rhs.kind {
						ObjectKind::Int => {
							if rhs.int_value == 0 {
								return err_exec!("zero division error: a % b");
							}
							let n = lhs.float_value % (rhs.int_value as f64);
							Ok(Object::from_float(n))
						}
						ObjectKind::Float => {
							if rhs.float_value == 0.0 {
								return err_exec!("zero division error: a % b");
							}
							let n = lhs.float_value % rhs.float_value;
							Ok(Object::from_float(n))
						}
						_ => {
							return err_exec!("can't mod: a % b (2)");
						}
					}
				}
				ObjectKind::Ident => {
					match rhs.kind {
						ObjectKind::Int |
						ObjectKind::Float |
						ObjectKind::String => {
							let lo = refer_ident(context, &lhs)?;
							return mul_div_objects(context, &lo, op, rhs);
						}
						_ => {
							return err_exec!("can't mod: a % b (4)")
						}
					}
				}
				_ => err_exec!("can't mod: a % b (5)"),
			}
		}
	}
}

pub fn compare_objects(context: &mut Context, lhs: &Object, op: &parser::CompareOpNode, rhs: &Object) -> Result<Object, Error> {
	match op {
		parser::CompareOpNode::Lt => {
			match lhs.kind {
				ObjectKind::Int => {
					match rhs.kind {
						ObjectKind::Int => {
							let b = lhs.int_value < rhs.int_value;
							Ok(Object::from_bool(b))
						},
						ObjectKind::Float => {
							let l = lhs.int_value as f64;
							let b = l < rhs.float_value;
							Ok(Object::from_bool(b))
						},
						ObjectKind::Ident => {
							let ro = refer_ident(context, &rhs)?;
							Ok(compare_objects(context, lhs, op, &ro)?)
						}
						_ => err_exec!("can't compare i64 and other: a < b"),
					}
				},
				ObjectKind::Float => {
					match rhs.kind {
						ObjectKind::Int => {
							let b = lhs.float_value < rhs.int_value as f64;
							Ok(Object::from_bool(b))
						},
						ObjectKind::Float => {
							let b = lhs.float_value < rhs.float_value;
							Ok(Object::from_bool(b))
						},
						ObjectKind::Ident => {
							let ro = refer_ident(context, &rhs)?;
							Ok(compare_objects(context, lhs, op, &ro)?)
						}
						_ => err_exec!("can't compare f64 and other: a < b"),
					}
				}
				ObjectKind::Ident => {
					match rhs.kind {
						ObjectKind::Int |
						ObjectKind::Float |
						ObjectKind::String => {
							let lo = refer_ident(context, &lhs)?;
							Ok(compare_objects(context, &lo, op, rhs)?)
						},
						ObjectKind::Ident => {
							let lo = refer_ident(context, &lhs)?;
							let ro = refer_ident(context, &rhs)?;
							Ok(compare_objects(context, &lo, op, &ro)?)
						}
						_ => err_exec!("can't compare f64 and other: a < b"),						
					}
				}
				_ => err_exec!("can't compare"),
			}
		},
		parser::CompareOpNode::LtEq => {
			match lhs.kind {
				ObjectKind::Int => {
					match rhs.kind {
						ObjectKind::Int => {
							let b = lhs.int_value <= rhs.int_value;
							Ok(Object::from_bool(b))
						},
						ObjectKind::Float => {
							let b = lhs.int_value as f64 <= rhs.float_value;
							Ok(Object::from_bool(b))
						},
						ObjectKind::Ident => {
							let ro = refer_ident(context, &rhs)?;
							Ok(compare_objects(context, lhs, op, &ro)?)
						}
						_ => err_exec!("can't compare i64 and other: a <= b"),
					}
				},
				ObjectKind::Float => {
					match rhs.kind {
						ObjectKind::Int => {
							let b = lhs.float_value <= rhs.int_value as f64;
							Ok(Object::from_bool(b))
						},
						ObjectKind::Float => {
							let b = lhs.float_value <= rhs.float_value;
							Ok(Object::from_bool(b))
						},
						ObjectKind::Ident => {
							let ro = refer_ident(context, &rhs)?;
							Ok(compare_objects(context, lhs, op, &ro)?)
						}
						_ => err_exec!("can't compare f64 and other: a <= b"),
					}
				}
				ObjectKind::Ident => {
					match rhs.kind {
						ObjectKind::Int |
						ObjectKind::Float |
						ObjectKind::String => {
							let lo = refer_ident(context, &lhs)?;
							Ok(compare_objects(context, &lo, op, rhs)?)
						},
						ObjectKind::Ident => {
							let lo = refer_ident(context, &lhs)?;
							let ro = refer_ident(context, &rhs)?;
							Ok(compare_objects(context, &lo, op, &ro)?)
						}
						_ => err_exec!("can't compare f64 and other: a <= b"),						
					}
				}
				_ => err_exec!("can't compare"),
			}
		},
		parser::CompareOpNode::Gt => {
			match lhs.kind {
				ObjectKind::Int => {
					match rhs.kind {
						ObjectKind::Int => {
							let b = lhs.int_value > rhs.int_value;
							Ok(Object::from_bool(b))
						},
						ObjectKind::Float => {
							let b = lhs.int_value as f64 > rhs.float_value;
							Ok(Object::from_bool(b))
						},
						ObjectKind::Ident => {
							let ro = refer_ident(context, &rhs)?;
							Ok(compare_objects(context, lhs, op, &ro)?)
						}
						_ => err_exec!("can't compare i64 and other: a > b"),
					}
				},
				ObjectKind::Float => {
					match rhs.kind {
						ObjectKind::Int => {
							let b = lhs.float_value > rhs.int_value as f64;
							Ok(Object::from_bool(b))
						},
						ObjectKind::Float => {
							let b = lhs.float_value > rhs.float_value;
							Ok(Object::from_bool(b))
						},
						ObjectKind::Ident => {
							let ro = refer_ident(context, &rhs)?;
							Ok(compare_objects(context, lhs, op, &ro)?)
						}
						_ => err_exec!("can't compare f64 and other: a > b"),
					}
				}
				ObjectKind::Ident => {
					match rhs.kind {
						ObjectKind::Int |
						ObjectKind::Float |
						ObjectKind::String => {
							let lo = refer_ident(context, &lhs)?;
							Ok(compare_objects(context, &lo, op, rhs)?)
						},
						ObjectKind::Ident => {
							let lo = refer_ident(context, &lhs)?;
							let ro = refer_ident(context, &rhs)?;
							Ok(compare_objects(context, &lo, op, &ro)?)
						}
						_ => err_exec!("can't compare f64 and other: a > b"),						
					}
				}
				_ => err_exec!("can't compare"),
			}
		},
		parser::CompareOpNode::GtEq => {
			match lhs.kind {
				ObjectKind::Int => {
					match rhs.kind {
						ObjectKind::Int => {
							let b = lhs.int_value >= rhs.int_value;
							Ok(Object::from_bool(b))
						},
						ObjectKind::Float => {
							let b = lhs.int_value as f64 >= rhs.float_value;
							Ok(Object::from_bool(b))
						},
						ObjectKind::Ident => {
							let ro = refer_ident(context, &rhs)?;
							Ok(compare_objects(context, lhs, op, &ro)?)
						}
						_ => err_exec!("can't compare i64 and other: a >= b"),
					}
				},
				ObjectKind::Float => {
					match rhs.kind {
						ObjectKind::Int => {
							let b = lhs.float_value >= rhs.int_value as f64;
							Ok(Object::from_bool(b))
						},
						ObjectKind::Float => {
							let b = lhs.float_value >= rhs.float_value;
							Ok(Object::from_bool(b))
						},
						ObjectKind::Ident => {
							let ro = refer_ident(context, &rhs)?;
							Ok(compare_objects(context, lhs, op, &ro)?)
						}
						_ => err_exec!("can't compare f64 and other: a >= b"),
					}
				}
				ObjectKind::Ident => {
					match rhs.kind {
						ObjectKind::Int |
						ObjectKind::Float |
						ObjectKind::String => {
							let lo = refer_ident(context, &lhs)?;
							Ok(compare_objects(context, &lo, op, rhs)?)
						},
						ObjectKind::Ident => {
							let lo = refer_ident(context, &lhs)?;
							let ro = refer_ident(context, &rhs)?;
							Ok(compare_objects(context, &lo, op, &ro)?)
						}
						_ => err_exec!("can't compare f64 and other: a >= b"),						
					}
				}
				_ => err_exec!("can't compare"),
			}
		},
		parser::CompareOpNode::Eq => {
			match lhs.kind {
				ObjectKind::Int => {
					match rhs.kind {
						ObjectKind::Int => {
							let b = lhs.int_value == rhs.int_value;
							Ok(Object::from_bool(b))
						},
						ObjectKind::Float => {
							let b = lhs.int_value as f64 == rhs.float_value;
							Ok(Object::from_bool(b))
						},
						ObjectKind::Ident => {
							let ro = refer_ident(context, &rhs)?;
							Ok(compare_objects(context, lhs, op, &ro)?)
						},
						_ => err_exec!("can't compare i64 and other: a == b"),
					}
				},
				ObjectKind::Float => {
					match rhs.kind {
						ObjectKind::Int => {
							let b = lhs.float_value == rhs.int_value as f64;
							Ok(Object::from_bool(b))
						},
						ObjectKind::Float => {
							let b = lhs.float_value == rhs.float_value;
							Ok(Object::from_bool(b))
						},
						ObjectKind::Ident => {
							let ro = refer_ident(context, &rhs)?;
							Ok(compare_objects(context, lhs, op, &ro)?)
						},
						_ => err_exec!("can't compare f64 and other: a == b"),
					}
				}
				ObjectKind::String => {
					match rhs.kind {
						ObjectKind::String => {
							let b = lhs.string == rhs.string;
							Ok(Object::from_bool(b))
						}
						ObjectKind::Ident => {
							let ro = refer_ident(context, &rhs)?;
							Ok(compare_objects(context, lhs, op, &ro)?)
						},
						_ => err_exec!("can't compare string and other: a == b"),
					}
				}
				ObjectKind::Bool => {
					match rhs.kind {
						ObjectKind::Bool => {
							let b = lhs.bool_value == rhs.bool_value;
							Ok(Object::from_bool(b))
						}
						ObjectKind::Ident => {
							let ro = refer_ident(context, &rhs)?;
							Ok(compare_objects(context, lhs, op, &ro)?)
						},
						_ => err_exec!("can't compare bool and other: a == b"),
					}
				}
				ObjectKind::Ident => {
					match rhs.kind {
						ObjectKind::Int |
						ObjectKind::Float |
						ObjectKind::Bool |
						ObjectKind::String => {
							let lo = refer_ident(context, &lhs)?;
							Ok(compare_objects(context, &lo, op, rhs)?)									
						},
						ObjectKind::Ident => {
							let lo = refer_ident(context, &lhs)?;
							let ro = refer_ident(context, &rhs)?;
							// println!("----");
							// context.print_tables_scanned_records();
							// println!("lhs: {} {}", lhs.to_string(), lo.to_string());
							// println!("rhs: {} {}", rhs.to_string(), ro.to_string());
							Ok(compare_objects(context, &lo, op, &ro)?)
						}
						_ => err_exec!("can't compare ident and other: a == b"),						
					}
				}
				_ => err_exec!("can't compare"),
			}
		},
		parser::CompareOpNode::NotEq => {
			match lhs.kind {
				ObjectKind::Int => {
					match rhs.kind {
						ObjectKind::Int => {
							let b = lhs.int_value != rhs.int_value;
							Ok(Object::from_bool(b))
						},
						ObjectKind::Float => {
							let b = lhs.int_value as f64 != rhs.float_value;
							Ok(Object::from_bool(b))
						},
						ObjectKind::Ident => {
							let ro = refer_ident(context, &rhs)?;
							Ok(compare_objects(context, lhs, op, &ro)?)
						},
						_ => err_exec!("can't compare i64 and other: a != b"),
					}
				},
				ObjectKind::Float => {
					match rhs.kind {
						ObjectKind::Int => {
							let b = lhs.float_value != rhs.int_value as f64;
							Ok(Object::from_bool(b))
						},
						ObjectKind::Float => {
							let b = lhs.float_value != rhs.float_value;
							Ok(Object::from_bool(b))
						},
						ObjectKind::Ident => {
							let ro = refer_ident(context, &rhs)?;
							Ok(compare_objects(context, lhs, op, &ro)?)
						},
						_ => err_exec!("can't compare f64 and other: a != b"),
					}
				}
				ObjectKind::String => {
					match rhs.kind {
						ObjectKind::String => {
							let b = lhs.string != rhs.string;
							Ok(Object::from_bool(b))
						}
						ObjectKind::Ident => {
							let ro = refer_ident(context, &rhs)?;
							Ok(compare_objects(context, lhs, op, &ro)?)
						},
						_ => err_exec!("can't compare string and other: a != b"),
					}
				}
				ObjectKind::Bool => {
					match rhs.kind {
						ObjectKind::Bool => {
							let b = lhs.bool_value != rhs.bool_value;
							Ok(Object::from_bool(b))
						}
						ObjectKind::Ident => {
							let ro = refer_ident(context, &rhs)?;
							Ok(compare_objects(context, lhs, op, &ro)?)
						},
						_ => err_exec!("can't compare bool and other: a != b"),
					}
				}
				ObjectKind::Ident => {
					match rhs.kind {
						ObjectKind::Int |
						ObjectKind::Float |
						ObjectKind::Bool |
						ObjectKind::String => {
							let lo = refer_ident(context, &lhs)?;
							Ok(compare_objects(context, &lo, op, rhs)?)
						},
						ObjectKind::Ident => {
							let lo = refer_ident(context, &lhs)?;
							let ro = refer_ident(context, &rhs)?;
							Ok(compare_objects(context, &lo, op, &ro)?)
						}
						_ => err_exec!("can't compare ident and other: a != b"),						
					}
				}
				_ => err_exec!("can't compare"),
			}
		},
	}
}

pub fn exec_paren_values(context: &mut Context, node: &parser::ParenValuesNode) -> Result<Vec<Object>, Error> {
	let mut v: Vec<Object> = vec![];

	for expr in node.exprs.iter() {
		let obj = exec_expr(context, expr)?;
		v.push(obj);
	}

	Ok(v)
}

pub fn exec_paren_idents(context: &mut Context, node: &parser::ParenIdentsNode) -> Result<Vec<Object>, Error> {
	let mut v: Vec<Object> = vec![];

	for ident in node.idents.iter() {
		let obj = exec_ident(context, ident)?;
		v.push(obj);
	}

	Ok(v)
}

pub fn exec_value(context: &mut Context, node: &parser::ValueNode) -> Result<Object, Error> {
	if let Some(int_value) = &node.int_value {
		return exec_i64_value(context, int_value);
	} else if let Some(float_value) = &node.float_value {
		return exec_f64_value(context, float_value);
	} else if let Some(bool_value) = &node.bool_value {
		return exec_bool_value(context, bool_value);
	} else if let Some(string) = &node.string {
		return exec_string(context, string);
	}
	return err_exec!("invalid state: value");
}

pub fn exec_operand(context: &mut Context, node: &parser::OperandNode) -> Result<Object, Error> {
	if let Some(int_value) = &node.int_value {
		return Ok(exec_i64_value(context, int_value)?);
	} else if let Some(float_value) = &node.float_value {
		return Ok(exec_f64_value(context, float_value)?);
	} else if let Some(bool_value) = &node.bool_value {
		return Ok(exec_bool_value(context, bool_value)?);
	} else if let Some(string) = &node.string {
		return Ok(exec_string(context, string)?);
	} else if let Some(ident) = &node.ident {
		return Ok(exec_ident(context, ident)?);
	} else if let Some(expr) = &node.expr {
		return Ok(exec_expr(context, expr)?);
	} else if node.star {
		return Ok(Object::from_star());
	}
	err_exec!("invalid state of operand in exec")
}

pub fn exec_bool_value(_: &mut Context, node: &parser::BoolValueNode) -> Result<Object, Error> {
	let mut o = Object::new();
	o.kind = ObjectKind::Bool;
	o.bool_value = node.value;
	Ok(o)
}

pub fn exec_i64_value(_: &mut Context, node: &parser::IntValueNode) -> Result<Object, Error> {
	let mut o = Object::new();
	o.kind = ObjectKind::Int;
	o.int_value = node.value;
	Ok(o)
}

pub fn exec_f64_value(_: &mut Context, node: &parser::FloatValueNode) -> Result<Object, Error> {
	let mut o = Object::new();
	o.kind = ObjectKind::Float;
	o.float_value = node.value;
	Ok(o)
}

pub fn exec_string(_: &mut Context, node: &parser::StringNode) -> Result<Object, Error> {
	let mut o = Object::new();
	o.kind = ObjectKind::String;
	o.string = node.value.clone();
	Ok(o)
}

pub fn exec_ident(_: &mut Context, node: &parser::IdentNode) -> Result<Object, Error> {
	let mut o = Object::new();
	o.kind = ObjectKind::Ident;
	o.ident = node.value.clone();
	Ok(o)
}

#[allow(dead_code)]
fn print_vec_string(head: &str, row: &Vec<String>) {
	if row.len() == 0 {
		println!("{}: []", head);
		return;
	}
	print!("{}: ", head);
	for col in row.iter() {
		print!("[{}] ", col);
	}
	println!("");
}

#[allow(dead_code)]
fn print_record(head: &str, row: &StringRecord) {
	if row.len() == 0 {
		println!("{}: []", head);
		return;
	}
	print!("{}: ", head);
	for col in row.iter() {
		print!("[{}] ", col);
	}
	println!("");
}

fn get_header_idents_by_obj(context: &Context, obj: &Object) -> Result<Vec<String>, Error> {
	if let Some(parent) = &obj.parent {
		match parent.kind {
			ObjectKind::Ident => {
				let table_name = parent.ident.clone();
				if context.tables.contains_key(&table_name) {
					let idents = context.get_table_header_idents(&table_name)?;
					Ok(idents)
				} else {
					return err_exec!("not found table ident '{}' in get header idents", table_name);
				}
			}
			_ => return err_exec!("invalid parent object"),
		}
	} else {
		let idents = context.get_table_header_idents(context.current_table_name.clone().as_str())?;
		Ok(idents)
	}
} 

fn get_table_record<'a>(context: &'a Context, obj: &'a Object) -> Result<&'a StringRecord, Error> {
	if let Some(parent) = &obj.parent {
		match parent.kind {
			ObjectKind::Ident => {
				let table_name = parent.ident.clone();
				if let Some(table) = context.tables.get(&table_name) {
					Ok(&table.scanned_record)
				} else {
					return err_exec!("not found table ident '{}' in get table record", table_name);
				}
			}
			_ => return err_exec!("invalid parent object"),
		}
	} else {
		if let Some(table) = context.tables.get(&context.current_table_name) {
			Ok(&table.scanned_record)
		} else {
			return err_exec!("not found table name '{}'", context.current_table_name);
		}
	}
}

fn get_table_name<'a>(context: &'a Context, obj: &'a Object) -> Result<&'a String, Error> {
	if let Some(parent) = &obj.parent {
		match parent.kind {
			ObjectKind::Ident => {
				Ok(&parent.ident)
			}
			_ => return err_exec!("invalid parent object"),
		}
	} else {
		Ok(&context.current_table_name)
	}
}

fn select_get_columns(context: &mut Context, node: &ProjectNode) -> Result<bool, Error> {
	context.selected_csv_columns.clear();

	let mut objs: Vec<Object> = vec![];
	if let Some(expr_list) = &node.expr_list {
		objs = exec_expr_list(context, expr_list)?;
	}

	let (records, ok) = select_record(context, &objs)?;
	if !ok {
		return Ok(ok);
	}

	context.selected_csv_columns = string_record_to_vev_string(&records);

	Ok(ok)
}

fn select_record(context: &mut Context, objs: &Vec<Object>) -> Result<(StringRecord, bool), Error> {
	let mut record = StringRecord::new();

	if objs.len() == 1 &&
	   objs[0].kind == ObjectKind::Star {
	   	let row = get_table_record(context, &objs[0])?;
	   	return Ok((row.clone(), true));
	}

	for obj in objs.iter() {
		let table_name = get_table_name(context, &obj)?;
		let row = get_table_record(context, &obj)?;
		let header_idents = get_header_idents_by_obj(context, &obj)?;
		if let Some(index) = header_idents.iter().position(|s| {
				if let Some(parent) = &obj.parent {
					let spar = parent.to_string();
					let rhs = format!("{}.{}", spar, obj.to_string());
					let lhs = format!("{}.{}", table_name, s);
					// println!("lhs[{}] == rhs[{}]", lhs, rhs);
					lhs == rhs
				} else {
					*s == *obj.to_string()
				}
			}) {
			println!("table_name[{}] index[{}] row.len[{}] row: {:?}", table_name, index, row.len(), row);
			if index >= row.len() {
				return Ok((record, false));
			}
			let col = &row[index];
			record.push_field(col.to_string().as_str());
		} else {
			if obj.kind == ObjectKind::Ident {
				return err_exec!("invalid column: {} in select record", obj.to_string());
			}
			record.push_field(obj.to_string().as_str());
		}
	}

	print_record("select record", &record);
	Ok((record, true))
}

fn get_indices(idents: &Vec<String>, objs: &Vec<Object>) -> Result<Vec<usize>, Error> {
	let mut indices = vec![];

	if objs.len() == 1 && objs[0].kind == ObjectKind::Star {
		for i in 0..idents.len() {
			indices.push(i);
		}
	} else {
		for obj in objs.iter() {
			let Some(index) = idents.iter().position(|s| *s == obj.to_string()) else {
				continue;
			};
			indices.push(index);
		}
	}

	Ok(indices)
}

fn collect_by_indices(row: &StringRecord, indices: &Vec<usize>) -> Result<Vec<String>, Error> {
	let mut dst = vec![];

	for index in indices.iter() {
		if *index > row.len() {
			return err_exec!("index out of range in collect");
		}
		dst.push(row[*index].to_string());
	}	

	Ok(dst)
}

fn vec_string_to_hashed_value_string(row: &Vec<String>) -> String {
	let mut s = String::new();

	for col in row.iter() {
		s.push_str(col);
		s.push_str(",");
	}

	s.pop();

	return s;
}

pub fn exec_aggregate(context: &mut Context, aggregate: &mut AggregateNode) -> Result<(), Error> {
	let mut limit_value = None;

	if let Some(limit) = &aggregate.limit {
		limit_value = gen_limit_value(context, limit)?;
	}

	if let Some(distinct) = aggregate.distinct.as_mut() {
		let mut record = StringRecord::new();
		let mut ok;

		loop {
			let result = exec_distinct(context, distinct)?;
			if !result.is_continue {
				break;
			}
			if result.record_is_empty {
				continue;
			}
			if context.filtered {
				if context.matched {
					if let Some(limit_value) = limit_value {
						if context.limit_counter >= limit_value {	
							break;
						}
					}
					let objs = context.cache_distinct_objs.clone();
					if let Some(objs) = objs {
						(record, ok) = select_record(context, &objs)?;
						if !ok {
							return Ok(());
						}
						context.cache_distinct_objs = None;
					} else if let Some(expr_list) = &aggregate.expr_list {
						let objs = exec_expr_list(context, expr_list)?;
						(record, ok) = select_record(context, &objs)?;
						if !ok {
							return Ok(());
						}
					}
					context.limit_counter += 1;
				}
			} else {
				if let Some(limit_value) = limit_value {
					if context.limit_counter >= limit_value {
						break;
					}
				}
				let objs = context.cache_distinct_objs.clone();
				if let Some(objs) = objs {
					(record, ok) = select_record(context, &objs)?;
					if !ok {
						return Ok(());
					}
					context.cache_distinct_objs = None;
				} else if let Some(expr_list) = &aggregate.expr_list {
					let objs = exec_expr_list(context, expr_list)?;
					(record, ok) = select_record(context, &objs)?;
					if !ok {
						return Ok(());
					}
				}
				context.limit_counter += 1;
			}
			if !aggregate.all {
				context.do_read_record = false;
				break;
			}
		}

		context.selected_csv_columns = string_record_to_vev_string(&record);
		print_string_record(&record)?;

		return Ok(());
	}
	return err_exec!("invalid state: aggregate");
}

pub fn exec_distinct(context: &mut Context, distinct: &mut DistinctNode) -> Result<ExecResult, Error> {
	let mut ret = ExecResult::new();

	if let Some(filter) = distinct.filter.as_mut() {
		let result = exec_filter(context, filter)?;
		ret.merge(&result);
		if !result.is_continue {
			return Ok(result);
		}
		if context.joins_enable_unmatched() {
			return Ok(ret);
		}

		if distinct.enable {
			context.skip = false;

			if let Some(expr_list) = &distinct.expr_list {
				let objs = exec_expr_list(context, expr_list)?;
				let row: &StringRecord = if let Some(table) = context.tables.get(&distinct.table_name) {
					&table.scanned_record
				} else {
					return err_exec!("not found table '{}' in distinct", distinct.table_name);
				};
				if row.len() > 0 {
					let idents = context.get_table_header_idents(context.current_table_name.clone().as_str())?;
					let indices = get_indices(&idents, &objs)?;
					let row = collect_by_indices(&row, &indices)?;
					let key = vec_string_to_hashed_value_string(&row);
					if context.distinct_map.contains_key(&key) {
						context.skip = true;
					} else {
						context.distinct_map.insert(key.clone(), true);
					}
				}
				context.cache_distinct_objs = Some(objs);
			} else {
				return err_exec!("invalid state: distinct");
			}
		}
	} else {
		return err_exec!("invalid state: distinct (2)");
	}

	Ok(ret)
}

fn vec_string_to_string_record(v: &Vec<String>) -> StringRecord {
	let mut r = StringRecord::new();
	for s in v.iter() {
		r.push_field(s);
	}
	r
}

pub fn exec_project(context: &mut Context, project: &mut ProjectNode) -> Result<bool, Error> {
	let mut limit_value = None;

	if let Some(limit) = &project.limit {
		limit_value = gen_limit_value(context, limit)?;
	}

	if let Some(distinct) = project.distinct.as_mut() {
		let result = exec_distinct(context, distinct)?;
		context.print_tables_scanned_records();
		// println!("result: {:?}", result);
		if !result.is_continue {
			return Ok(false);
		}
		if context.joins_enable_unmatched() {
			return Ok(result.is_continue);
		}
		if result.record_is_empty {
			return Ok(true);
		}
		if context.skip {
			return Ok(true);
		}
		if context.filtered {
			if context.matched {
				if let Some(limit_value) = limit_value {
					if context.limit_counter >= limit_value &&
					   project.method == TokenKind::Get {
						return Ok(false);						
					}
				}
				context.limit_counter += 1;
				if let Some(test_get_records) = context.test_get_records.as_mut() {
					test_get_records.push(context.matched_record.clone());
				}
				if select_get_columns(context, project)? {
					if let Some(test_selected_records) = context.test_selected_records.as_mut() {
						let rec = context.selected_csv_columns.clone();
						test_selected_records.push(vec_string_to_string_record(&rec));
					}
					if context.is_cli {
						print_selected_columns(context)?;
					}
					context.counter_selected += 1;
				}
			}
		} else {
			if let Some(limit_value) = limit_value {
				if context.limit_counter >= limit_value &&
				   project.method == TokenKind::Get {
					return Ok(false);						
				}
			}
			context.limit_counter += 1;
			let record = context.get_current_table_scanned_record()?;
			if let Some(test_get_records) = context.test_get_records.as_mut() {
				test_get_records.push(record);
			}
			if select_get_columns(context, project)? {				
				if let Some(test_selected_records) = context.test_selected_records.as_mut() {
					let rec = context.selected_csv_columns.clone();
					test_selected_records.push(vec_string_to_string_record(&rec));
				}
				if context.is_cli {
					print_selected_columns(context)?;
				}
				context.counter_selected += 1;
			}
		}
		if !project.all && context.counter_selected >= 1 {
			context.do_read_record = false;
			return Ok(false);
		}
		return Ok(result.is_continue);
	}
	return err_exec!("invalid state: project");
}

pub fn exec_filter(context: &mut Context, node: &mut FilterNode) -> Result<ExecResult, Error> {
	let mut ret = ExecResult::new();

	if let Some(where_clause) = &node.where_clause {
		context.filtered = true;
		if let Some(joins) = node.joins.as_mut() {
			let result = exec_joins(context, joins)?;
			if !result.is_continue {
				ret.merge(&result);
				return Ok(ret);
			}
			if context.joins_enable_unmatched() {
				ret.merge(&result);
				return Ok(ret);
			}

			let o = exec_where_clause(context, where_clause)?;
			if o.bool_value {
				context.matched_record = context.get_current_table_scanned_record()?;
				context.unmatched_record.clear();
				context.matched = true;
			} else {
				context.matched_record.clear();
				context.unmatched_record = context.get_current_table_scanned_record()?;
				context.matched = false;
			}
			return Ok(result);
		}
	} else {
		context.filtered = false;
		context.matched = false;
		if let Some(joins) = node.joins.as_mut() {
			let result = exec_joins(context, joins)?;
			ret.merge(&result);
			return Ok(ret);
		}
	}
	return err_exec!("invalid state: filter");
}

pub fn exec_joins(context: &mut Context, node: &mut JoinsNode) -> Result<ExecResult, Error> {
	let mut ret = ExecResult::new();

	if !context.wait_left_scan {
		if let Some(csv_file_scan) = node.csv_file_scan.as_mut() {
			let result = exec_csv_file_scan(context, csv_file_scan)?;
			if !result.is_continue {
				ret.merge(&result);
				return Ok(ret);
			}
		}
	}

	if let Some(join) = node.join.as_mut() {
		let result = exec_join(context, join)?;	
		ret.merge(&result);
		ret.is_continue = true;
		if result.join_matched {
			context.wait_left_scan = true;
			context.join_matched = true;
		} else {
			context.wait_left_scan = false;
			context.join_matched = false;
		}
	}

	Ok(ret)
}

macro_rules! solve_scan {
	($context:ident, $csv_file_scan:ident, $expr:ident, $ret:ident) => {

		let result = exec_csv_file_scan($context, $csv_file_scan)?;
		if !result.is_continue {
			$ret.merge(&result);
			break;
		}
		let o = exec_expr($context, $expr)?;
		if o.kind == ObjectKind::Bool && o.bool_value {
			// match
			$ret.join_matched = true;
			break;
		}								
	}
}

pub fn exec_join(context: &mut Context, node: &mut JoinNode) -> Result<ExecResult, Error> {
	let mut ret = ExecResult::new();

	/*
		for (users) {
			for (products) {
				for (countries) {
				}
			}
		}

	 */
	if let Some(item) = node.item.as_mut() {
		match item {
			JoinItemNode::InnerJoin(inner_join) => {
				if let Some(csv_file_scan) = inner_join.csv_file_scan.as_mut() {
				if let Some(expr) = &inner_join.expr {
					loop {
						// println!("inner_join[{}] scan", inner_join.table_name);
						if let Some(join) = node.join.as_mut() {
							let result = exec_join(context, join)?;	
							println!("result: {:?}", result);
							ret.merge(&result);
							if !result.is_continue {
								solve_scan!(context, csv_file_scan, expr, ret);
							}
							if result.join_matched {
								if !context.tables.contains_key(&inner_join.table_name) {
									let result = exec_csv_file_scan(context, csv_file_scan)?;
									if !result.is_continue {
										ret.merge(&result);
										break;
									}
									let o = exec_expr(context, expr)?;
									if o.kind == ObjectKind::Bool && o.bool_value {
										// match
										ret.join_matched = true;
										break;
									}								
									break;
								}
								break;
							}
						} else {
							solve_scan!(context, csv_file_scan, expr, ret);
						}
					}	
				}
				}
			}
		}
	}

	Ok(ret)
}

pub fn create_table(context: &mut Context, node: &mut CsvFileScanNode) -> Result<(), Error> {
	if !context.tables.contains_key(&node.table_name) {
		let mut table = Box::new(Table::from(node.table_name.clone()));
		table.name = node.table_name.clone();

		context.current_table_name = node.table_name.clone();

		let path = context.gen_table_file_path(&node.table_name)?;
		let mut reader = match Reader::from_path(&path) {
			Ok(v) => v,
			Err(e) => return err_exec!("failed to create csv reader. {}", e),
		};

		let headers = match reader.headers() {
			Ok(v) => v.clone(),
			Err(e) => return err_exec!("failed to read header of CSV file. {}", e),
		};
		let header_types = parse_csv_headers_as_types(&headers)?;
		let header_idents = parse_header_idents(&headers)?;
		table.csv_reader = Some(reader);
		table.headers = headers;
		table.header_types = header_types;
		table.header_idents = header_idents;

		context.tables.insert(node.table_name.clone(), table);
	}
	Ok(())
}

pub fn exec_csv_file_scan(context: &mut Context, node: &mut CsvFileScanNode) -> Result<ExecResult, Error> {
	let mut ret = ExecResult::new();
	let mut reader_is_none: bool = true;

	if context.tables.contains_key(&node.table_name) {
		let table = context.tables.get(&node.table_name).unwrap();
		reader_is_none = table.csv_reader.is_none();
	} else {
		let mut table = Box::new(Table::from(node.table_name.clone()));
		table.name = node.table_name.clone();
		// println!("scan table[{}]", table.name);
		context.tables.insert(node.table_name.clone(), table);
	}

	if reader_is_none {
		context.current_table_name = node.table_name.clone();

		let path = context.gen_table_file_path(&node.table_name)?;
		let mut reader = match Reader::from_path(&path) {
			Ok(v) => v,
			Err(e) => return err_exec!("failed to create csv reader. {}", e),
		};

		let headers = match reader.headers() {
			Ok(v) => v.clone(),
			Err(e) => return err_exec!("failed to read header of CSV file. {}", e),
		};
		let header_types = parse_csv_headers_as_types(&headers)?;
		let header_idents = parse_header_idents(&headers)?;
		if let Some(table) = context.tables.get_mut(&node.table_name) {
			table.csv_reader = Some(reader);
			table.headers = headers;
			table.header_types = header_types;
			table.header_idents = header_idents;
		}
		println!("table created: {}", node.table_name);
	}	
	if let Some(table) = context.tables.get_mut(&node.table_name) {
		if let Some(reader) = table.csv_reader.as_mut() {
			let mut scanned_record = StringRecord::new();
			println!("scan record: {}", node.table_name);
			match reader.read_record(&mut scanned_record) {
				Ok(_) => {
					table.scanned_record = scanned_record;
					if table.scanned_record.len() == 0 {
						table.csv_reader = None;
						ret.record_is_empty = true;
						ret.is_continue = false;
						println!("is continue false");
						return Ok(ret);
					}
					return Ok(ret);
				}
				Err(_) => {
					table.csv_reader = None;
					ret.is_continue = false;
					return Ok(ret);
				}
			};
		} else {
			return err_exec!("reader is none");
		}
	} else {
		return err_exec!("impossible");
	}
}

pub fn parse_header_idents(headers: &StringRecord) -> Result<Vec<String>, Error> {
	let mut v = vec![];

	for col in headers.iter() {
		if let Some((left, _right)) = col.split_once(':') {
			let val = left.trim().to_string();
			v.push(val);
		}
	}

	Ok(v)
}

fn is_db_dir(path: &Path) -> bool {
	path.join("id").exists() && path.join("tables").exists()
}

pub fn exec_dir_delete_all(context: &mut Context, node: &DirDeleteAllNode) -> Result<(), Error> {
	let db_name = node.db_name.clone().unwrap();
	if db_name == context.using_db_name {
		return err_exec!("{} database is using now. can't delete", db_name);
	}
	if db_name.contains("..") {
		return err_exec!("directory traversal error");
	}
	let path = context.gen_db_dir_path(&db_name)?;
	if path.as_os_str().is_empty() {
		return err_exec!("invalid path in dir delete all");
	}
	
	if node.if_exists {
		if path.exists() {
			if !is_db_dir(&path) {
				return err_exec!("does not database directory");
			}
			match fs::remove_dir_all(&path) {
				Ok(_) => {},
				Err(e) => return err_exec!("failed to remove directory. {}", e),
			}
		}
	} else {
		if !path.exists() {
			return err_exec!("does not exists database directory");
		}
		if !is_db_dir(&path) {
			return err_exec!("does not database directory");
		}
		match fs::remove_dir_all(&path) {
			Ok(_) => {},
			Err(e) => return err_exec!("failed to remove directory. {}", e),
		}		
	}
	Ok(())
}

pub fn exec_dir_list(context: &mut Context, node: &DirListNode) -> Result<(), Error> {
	if node.csv_file_grep.is_some() {
		// show tables
		let path = context.gen_using_db_tables_path()?;
		let dir = match fs::read_dir(&path) {
			Ok(v) => v,
			Err(e) => return err_exec!("failed to read dir: {}", e),
		};
		for entry in dir {
			let entry = entry.unwrap();
			let path = entry.path();
			if path.extension().and_then(|s| s.to_str()) == Some("csv") {
				println!("{}", path.file_stem().unwrap().to_str().unwrap());
			}
		}
	} else {
		// show databases
		let dir = match fs::read_dir(&context.root_dir_path) {
			Ok(v) => v,
			Err(e) => return err_exec!("failed to read dir: {}", e),
		};
		for entry in dir {
			let entry = entry.unwrap();
			let path = entry.path();
			println!("{}", path.file_stem().unwrap().to_str().unwrap());
		}
	}

	Ok(())
}

pub fn exec_database_create(context: &mut Context, node: &DatabaseCreateNode) -> Result<(), Error> {
	let path = context.gen_db_dir_path(&node.db_name)?;
	if path.exists() {
		return err_exec!("{} database already exists", node.db_name);
	}

	match fs::create_dir(&path) {
		Ok(_) => {},
		Err(e) => return err_exec!("failed to create database directory. {}", e),
	}
	match fs::create_dir(&path.join("tables")) {
		Ok(_) => {},
		Err(e) => return err_exec!("failed to create tables directory. {}", e),
	}		
	match fs::create_dir(&path.join("id")) {
		Ok(_) => {},
		Err(e) => return err_exec!("failed to create id directory. {}", e),
	}		
	Ok(())
}

pub fn exec_row_delete(context: &mut Context, row_delete: &mut RowDeleteNode, writer: &mut Writer<fs::File>) -> Result<(), Error> {
	if let Some(project) = row_delete.project.as_mut() {
		let all = row_delete.all;
		let mut limit_value = None;
		let mut limited = false;

		if let Some(limit) = &project.limit {
			limit_value = gen_limit_value(context, limit)?;
		}
		if let Some(limit_value) = limit_value {
			if context.limit_counter >= limit_value {
				limited = true;
			}
		}

		if all {
			while exec_project(context, project)? {
				if !limited {
					if context.filtered {
						if context.matched {
							// delete
							if let Some(limit_value) = limit_value {
								if context.limit_counter >= limit_value {
									limited = true;
								}
							}
						} else {
							writer.write_record(&context.unmatched_record).unwrap();
						}
					} else {
						// delete
						if let Some(limit_value) = limit_value {
							if context.limit_counter >= limit_value {
								limited = true;
							}
						}
					}
				} else {
					writer.write_record(&context.get_current_table_scanned_record()?).unwrap();
				}
			}
		} else {
			let mut count = 0;
			let mut limited = false;

			if let Some(limit_value) = limit_value {
				if context.limit_counter >= limit_value {
					limited = true;
				}
			}

			while exec_project(context, project)? {
				if !limited {
					if context.filtered {
						if count == 0 {
							if context.matched {
								count += 1;
								// delete
								if let Some(limit_value) = limit_value {
									if context.limit_counter >= limit_value {
										limited = true;
									}
								}
							} else {
								writer.write_record(&context.unmatched_record).unwrap();
							}
						} else {
							writer.write_record(&context.get_current_table_scanned_record()?).unwrap();
						}
					} else {
						if count == 0 {
							count += 1;
							// delete
							if let Some(limit_value) = limit_value {
								if context.limit_counter >= limit_value {
									limited = true;
								}
							}
						} else {
							writer.write_record(&context.get_current_table_scanned_record()?).unwrap();
						}
					}
				} else {
					writer.write_record(&context.get_current_table_scanned_record()?).unwrap();
				}
			}			
		}
	}

	Ok(())
}

pub fn replace_columns_by_objs(context: &mut Context, cols: &StringRecord, update_expr_list_objs: &Vec<Object>) -> Result<Vec<String>, Error> {
	let idents = context.get_table_header_idents(context.current_table_name.clone().as_str())?;
	let mut row: Vec<String> = vec![];

	for col in cols {
		row.push(col.to_string());
	}

	for obj in update_expr_list_objs.iter() {
		let key = obj.to_string();
		if let Some(o) = context.vars.get(&key) {
			if let Some(index) = idents.iter().position(|s| *s == key) {
				row[index] = o.to_string();
			}
		}
	}

	Ok(row)
}

fn drop_record_column(row: &StringRecord, index: usize) -> Result<StringRecord, Error> {
	let mut dst = StringRecord::new();

	for (i, field) in row.iter().enumerate() {
		if i != index {
			dst.push_field(field);
		}
	}

	Ok(dst)
}

fn alter_field_column_type(types: &Vec<HeaderType>, ident: &String, row: &StringRecord) -> Result<StringRecord, Error> {
	let index = types.iter().position(|t| t.ident == *ident);
	if index.is_none() {
		return err_exec!("not found '{}' in column types", *ident);
	}
	let index = index.unwrap();
	let val = row[index].to_string();
	let typ = &types[index];
	let obj = typ.parse_str(val.as_str())?;
	let mut dst = StringRecord::new();

	for (i, val) in row.iter().enumerate() {
		if i == index {
			dst.push_field(obj.to_string().as_str());
		} else {
			dst.push_field(val);
		}
	}

	return Ok(dst);
}

pub fn exec_column_alter_type(context: &mut Context, types: &Vec<HeaderType>, node: &mut ColumnAlterTypeNode, writer: &mut Writer<fs::File>) -> Result<(), Error> {

	if let Some(project) = node.project.as_mut() {
		while exec_project(context, project)? {
			if let Some(ident) = &node.ident {
				let row = alter_field_column_type(types, ident, &context.get_current_table_scanned_record()?)?;
				writer.write_record(&row).unwrap();
			}
		}
	}	

	Ok(())
}

pub fn exec_column_rename(context: &mut Context, node: &mut ColumnRenameNode, writer: &mut Writer<fs::File>) -> Result<(), Error> {
	if let Some(project) = node.project.as_mut() {
		while exec_project(context, project)? {
			let row = &context.get_current_table_scanned_record()?;
			writer.write_record(row).unwrap();
		}
	}
	Ok(())
}

pub fn exec_column_drop(context: &mut Context, node: &mut ColumnDropNode, writer: &mut Writer<fs::File>, types: &Vec<HeaderType>) -> Result<(), Error> {
	if let Some(project) = node.project.as_mut() {
		let mut drop_index: Option<usize> = None;

		if let Some(ident) = &node.ident {
			for (i, typ) in types.iter().enumerate() {
				if typ.ident == *ident {
					drop_index = Some(i);
					break;
				}
			}			
		}

		if drop_index.is_none() {
			return err_exec!("drop index is none");
		}
		let drop_index = drop_index.unwrap();

		while exec_project(context, project)? {
			let row = &context.	get_current_table_scanned_record()?;
			let row = drop_record_column(&row, drop_index)?;
			writer.write_record(&row).unwrap();
		}
	}
	Ok(())
}

pub fn exec_column_add(context: &mut Context, node: &mut ColumnAddNode, writer: &mut Writer<fs::File>, headers: &StringRecord) -> Result<(), Error> {
	if let Some(project) = node.project.as_mut() {
		let def_row = gen_default_record(headers)?;
		assert!(def_row.len() > 0);

		while exec_project(context, project)? {
			let mut row = context.get_current_table_scanned_record()?;
			row.push_field(def_row.to_vec().last().unwrap().as_str());
			writer.write_record(&row).unwrap();
		}
	}

	Ok(())
}

pub fn exec_row_update(context: &mut Context, row_update: &mut RowUpdateNode, writer: &mut Writer<fs::File>) -> Result<(), Error> {
	if let Some(project) = row_update.project.as_mut() {
		if let Some(expr_list) = &row_update.expr_list {
			let update_expr_list_objs = exec_expr_list(context, expr_list)?;
			let mut limit_value = None;
			let mut limited = false;

			if let Some(limit) = &project.limit {
				limit_value = gen_limit_value(context, limit)?;
			}
			if let Some(limit_value) = limit_value {
				if context.limit_counter >= limit_value {
					limited = true;
				}
			}

			if row_update.all {
				while exec_project(context, project)? {
					if !limited {
						if context.filtered {
							if context.matched {
								let cols = context.matched_record.clone();
								let cols = replace_columns_by_objs(context, &cols, &update_expr_list_objs)?;
								writer.write_record(&cols).unwrap();
								if let Some(limit_value) = limit_value {
									if context.limit_counter >= limit_value {
										limited = true;
									}
								}
							} else {
								let cols = context.unmatched_record.clone();
								writer.write_record(&cols).unwrap();
							}
						} else {
							let cols = context.get_current_table_scanned_record()?;
							let cols = replace_columns_by_objs(context, &cols, &update_expr_list_objs)?;
							writer.write_record(&cols).unwrap();
							if let Some(limit_value) = limit_value {
								if context.limit_counter >= limit_value {
									limited = true;
								}
							}
						}
					} else {
						let cols = context.get_current_table_scanned_record()?;
						writer.write_record(&cols).unwrap();
					}
				}
			} else {
				let mut writted = false;

				while exec_project(context, project)? {
					if !limited {
						if context.filtered {
							if context.matched && !writted {
								let cols = context.matched_record.clone();
								let cols = replace_columns_by_objs(context, &cols, &update_expr_list_objs)?;
								writer.write_record(&cols).unwrap();
								writted = true;
								if let Some(limit_value) = limit_value {
									if context.limit_counter >= limit_value {
										limited = true;
									}
								}
							} else {
								let cols = context.get_current_table_scanned_record()?;
								writer.write_record(&cols).unwrap();
							}
						} else {
							if !writted {
								let cols = context.get_current_table_scanned_record()?;
								let cols = replace_columns_by_objs(context, &cols, &update_expr_list_objs)?;
								writer.write_record(&cols).unwrap();
								writted = true;
								if let Some(limit_value) = limit_value {
									if context.limit_counter >= limit_value {
										limited = true;
									}
								}
							} else {
								writer.write_record(&context.get_current_table_scanned_record()?).unwrap();
							}
						}
					} else {
						writer.write_record(&context.get_current_table_scanned_record()?).unwrap();
					}
				}
			}

			return Ok(());
		}
	}

	return err_exec!("failed to row update");
}

fn drop_headers_column(types: &Vec<HeaderType>, headers: &mut StringRecord, ident: &String) -> Result<StringRecord, Error> {
	let mut row = StringRecord::new();

	for i in 0..types.len() {
		let typ = &types[i];
		if typ.ident != *ident {
			row.push_field(&headers[i]);
		}
	}

	Ok(row)
}

fn rename_headers_column(types: &Vec<HeaderType>, from_ident: &String, to_ident: &String) -> Result<StringRecord, Error> {
	let types = types.clone();
	let mut dst = StringRecord::new();

	for typ in types.iter() {
		let mut typ = typ.clone();
		if typ.ident == *from_ident {
			typ.ident = to_ident.clone();
		}
		dst.push_field(typ.to_string().as_str());
	}

	Ok(dst)
}

pub fn exec_csv_file_rename(context: &mut Context, node: &CsvFileRenameNode) -> Result<(), Error> {
	if let Some(table_name) = &node.table_name {
		if let Some(to_ident) = &node.to_ident {
			let old_path = context.gen_table_file_path(table_name)?;
			let new_path = context.gen_table_file_path(to_ident)?;
			if !old_path.exists() {
				return err_exec!("'{}' does not exists", table_name);
			}
			if new_path.exists() {
				return err_exec!("table '{}' is already exists", to_ident);
			}
			match fs::rename(old_path, new_path) {
				Ok(_) => {},
				Err(e) => return err_exec!("failed to rename table. {}", e),
			}
		}
	}

	Ok(())
}

pub fn alter_headers_column_type(context: &mut Context, types: &Vec<HeaderType>, column_ident: &String, column_types: &Vec<parser::ColumnTypeNode>) -> Result<StringRecord, Error> {
	let mut headers = StringRecord::new();

	for otyp in types.iter() {
		if otyp.ident == *column_ident {
			let mut typ = HeaderType::new();

			typ.ident = otyp.ident.clone();

			for col_type in column_types.iter() {
				match col_type {
					parser::ColumnTypeNode::Int => {
						typ.is_int = true;
					}
					parser::ColumnTypeNode::Float => {
						typ.is_float = true;
					}
					parser::ColumnTypeNode::Bool => {
						typ.is_bool = true;
					}
					parser::ColumnTypeNode::Char(size) => {
						typ.is_char = true;
						typ.char_size = *size;
					}
					parser::ColumnTypeNode::PrimaryKey => {
						typ.is_primary_key = true;
					}
					parser::ColumnTypeNode::AutoIncrement => {
						typ.is_auto_increment = true;
					}
					parser::ColumnTypeNode::Default(value) => {
						typ.is_default = true;
						typ.default_value = Some(exec_value(context, &value)?);
					}
				}
			}

			headers.push_field(typ.to_string().as_str());
		} else {
			headers.push_field(otyp.to_string().as_str());
		}
	}

	Ok(headers)
}

pub fn exec_csv_file_rewrite(context: &mut Context, node: &mut CsvFileRewriteNode) -> Result<(), Error> {
	if let Some(table_name) = &node.table_name {
		context.current_table_name = table_name.clone();
		let org_path = context.gen_table_file_path(&table_name)?;
		let tmp_path = context.gen_tmp_table_file_path(&table_name)?;
		let mut headers = read_table_headers(context, &table_name)?;
		let types = parse_csv_headers_as_types(&headers)?;
		let mut writer = match Writer::from_path(&tmp_path) {
			Ok(v) => v,
			Err(e) => return err_exec!("failed to open CSV writer: {}", e),
		};

		if let Some(column_add) = &node.column_add {
			if let Some(column_def_string) = &column_add.column_definition_string {
				headers.push_field(column_def_string.as_str());
			}
		} else if let Some(column_drop) = &node.column_drop {
			if let Some(ident) = &column_drop.ident {
				headers = drop_headers_column(&types, &mut headers, &ident)?;
			}
		} else if let Some(column_rename) = &node.column_rename {
			if let Some(from_ident) = &column_rename.from_ident {
				if let Some(to_ident) = &column_rename.to_ident {
					headers = rename_headers_column(&types, from_ident, to_ident)?;
				}
			}
		} else if let Some(column_alter_type) = &node.column_alter_type {
			if let Some(column_ident) = &column_alter_type.ident {
				headers = alter_headers_column_type(context, &types, column_ident, &column_alter_type.column_types)?;
			}
		}
		
		writer.write_record(&headers).unwrap();

		if let Some(row_delete) = node.row_delete.as_mut() {
			exec_row_delete(context, row_delete, &mut writer)?;
		} else if let Some(row_update) = node.row_update.as_mut() {
			exec_row_update(context, row_update, &mut writer)?;
		} else if let Some(column_add) = node.column_add.as_mut() {
			exec_column_add(context, column_add, &mut writer, &headers)?;
		} else if let Some(column_drop) = node.column_drop.as_mut() {
			exec_column_drop(context, column_drop, &mut writer, &types)?;
		} else if let Some(column_rename) = node.column_rename.as_mut() {
			exec_column_rename(context, column_rename, &mut writer)?;
		} else if let Some(column_alter_type) = node.column_alter_type .as_mut() {
			let types = parse_csv_headers_as_types(&headers)?;
			match exec_column_alter_type(context, &types, column_alter_type, &mut writer) {
				Ok(_) => {},
				Err(e) => {
					fs::remove_file(&tmp_path).unwrap();
					return Err(e);
				}
			}
		} else {
			return err_exec!("invalid state: csv file rewrite");
		}

		fs::rename(&tmp_path, &org_path).unwrap();

		return Ok(());
	}

	return err_exec!("failed to csv file rewrite");
}

pub fn read_table_headers(context: &Context, table_name: &str) -> Result<StringRecord, Error> {
	let path = context.gen_table_file_path(table_name)?;
	let mut reader = match Reader::from_path(&path) {
		Ok(v) => v,
		Err(e) => return err_exec!("failed to create CSV reader: {}", e),
	};
	let headers = match reader.headers() {
		Ok(v) => v,
		Err(e) => return err_exec!("failed to read CSV headers: {}", e),
	};
	Ok(headers.clone())
}

pub fn exec_csv_file_delete(context: &mut Context, node: &CsvFileDeleteNode) -> Result<(), Error> {
	let table_name = node.table_name.clone().unwrap();
	let path = context.gen_table_file_path(&table_name)?;
	if node.if_exists {
		if path.exists() {
			fs::remove_file(&path).unwrap();
		}		
	} else {
		match fs::remove_file(&path) {
			Ok(_) => {},
			Err(e) => return err_exec!("failed to remove CSV file. {}", e),
		}
	}
	Ok(())
}

pub fn exec_csv_file_create(context: &mut Context, node: &CsvFileCreateNode) -> Result<(), Error> {
	// create_table
	let path = context.gen_table_file_path(&node.table_name)?;

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

/*	fn setup(context: &mut Context) {
		if Path::new("test_env/test_db").exists() {
			fs::remove_dir_all("test_env/test_db").unwrap();
		}
		do_exec(context, "CREATE DATABASE test_db").unwrap();
		do_exec(context, "USE test_db").unwrap();
	}
*/
	fn remove_file<P: AsRef<Path>>(path: P) {
		if path.as_ref().exists() {
			fs::remove_file(path).unwrap();
		}
	}

	fn do_exec(context: &mut Context, query: &str) -> Result<(), Error> {
		let tests_dir = Path::new("test_env");
		if !tests_dir.exists() {
			fs::create_dir(tests_dir).unwrap();
		}
		context.root_dir_path = PathBuf::from("test_env");
		let tokens: Vec<Token> = tokenize(query.to_string()).unwrap();
		let mut tok_strm = TokenStream::new(tokens);
		let node: QueryNode = match parse(&mut tok_strm) {
			Ok(v) => v,
			Err(e) => return err_parse!("{}", e),
		};
		let mut node: PlansNode = match planning(&node) {
			Ok(v) => v,
			Err(e) => return err_planning!("{}", e),
		};
		return exec(context, &mut node);
	}

	#[test]
	fn test_use_db() {
		let mut context = Context::new();
		do_exec(&mut context, "DROP DATABASE IF EXISTS hige").unwrap();
		do_exec(&mut context, "CREATE DATABASE hige").unwrap();
		do_exec(&mut context, "USE hige").unwrap();
		assert!(context.using_db_name == "hige");
	}

	#[test]
	fn test_use_db_1() {
		let mut context = Context::new();
		do_exec(&mut context, "DROP DATABASE IF EXISTS hige").unwrap();
		match do_exec(&mut context, "USE hige") {
			Ok(_) => panic!("failed"),
			Err(_) => {}
		}
	}

	#[test]
	fn test_use_db_2() {
		let mut context = Context::new();
		do_exec(&mut context, "DROP DATABASE IF EXISTS hige").unwrap();
		do_exec(&mut context, "CREATE DATABASE hige").unwrap();
		match do_exec(&mut context, "USE ..") {
			Ok(_) => panic!("failed"),
			Err(_) => {},
		}
	}

	#[test]
	fn test_database_create() {
		let path = Path::new("test_env").join("test_db");
		if path.exists() {
			fs::remove_dir_all(&path).unwrap();
		}
		let mut context = Context::new();
		do_exec(&mut context, "DROP DATABASE IF EXISTS test_db").unwrap();
		do_exec(&mut context, "CREATE DATABASE test_db").unwrap();
		assert!(path.exists());
		assert!(path.join("tables").exists());
		assert!(path.join("id").exists());
	}

	fn gen_test_table_path() -> PathBuf {
		Path::new("test_env").join("test_db").join("tables").join("test_table.csv")
	}

	#[test]
	fn test_create_table_stmt_0() {
		let path = gen_test_table_path();
		remove_file(&path);
		let mut context = Context::new();
		do_exec(&mut context, "DROP DATABASE IF EXISTS test_db").unwrap();
		do_exec(&mut context, "CREATE DATABASE test_db").unwrap();
		do_exec(&mut context, "USE test_db").unwrap();
		do_exec(&mut context, "CREATE TABLE test_table (id: INT, weight: FLOAT)").unwrap();
		assert!(path.exists());
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT\n");
	}

	#[test]
	fn test_create_table_stmt_1() {
		let path = gen_test_table_path();
		remove_file(&path);
		let mut context = Context::new();
		do_exec(&mut context, "DROP DATABASE IF EXISTS test_db").unwrap();
		do_exec(&mut context, "CREATE DATABASE test_db").unwrap();
		do_exec(&mut context, "USE test_db").unwrap();
		do_exec(&mut context, "CREATE TABLE test_table (id: INT PRIMARY_KEY AUTO_INCREMENT, weight: FLOAT, name: CHAR[4])").unwrap();
		assert!(path.exists());
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT PRIMARY_KEY AUTO_INCREMENT,weight: FLOAT,name: CHAR[4]\n");
	}

	#[test]
	fn test_add_stmt_0d() {
		let path = gen_test_table_path();
		remove_file(&path);
		let mut context = Context::new();
		do_exec(&mut context, "DROP DATABASE IF EXISTS test_db").unwrap();
		do_exec(&mut context, "CREATE DATABASE test_db").unwrap();
		do_exec(&mut context, "USE test_db").unwrap();
		do_exec(&mut context, "CREATE TABLE test_table (id: INT, weight: FLOAT, name: CHAR[128])").unwrap();
		assert!(path.exists());
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n");

		do_exec(&mut context, "ADD OF test_table (id, name, weight) VALUES (1, \"aaa\", 1.23), (2, \"bbb\", 2.23)").unwrap();

		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]
1,1.23,aaa
2,2.23,bbb
");
	}

	#[test]
	fn test_add_stmt_0c() {
		let path = gen_test_table_path();
		remove_file(&path);
		let mut context = Context::new();
		do_exec(&mut context, "DROP DATABASE IF EXISTS test_db").unwrap();
		do_exec(&mut context, "CREATE DATABASE test_db").unwrap();
		do_exec(&mut context, "USE test_db").unwrap();
		do_exec(&mut context, "CREATE TABLE test_table (id: INT, weight: FLOAT, name: CHAR[128])").unwrap();
		assert!(path.exists());
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n");
		do_exec(&mut context, "ADD OF test_table VALUES (1, 1.23, \"aaa\"), (2, 2.23, \"bbb\")").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]
1,1.23,aaa
2,2.23,bbb
");
	}

	#[test]
	fn test_add_stmt_0b() {
		let path = gen_test_table_path();
		remove_file(&path);
		let mut context = Context::new();
		do_exec(&mut context, "DROP DATABASE IF EXISTS test_db").unwrap();
		do_exec(&mut context, "CREATE DATABASE test_db").unwrap();
		do_exec(&mut context, "USE test_db").unwrap();
		do_exec(&mut context, "CREATE TABLE test_table (id: INT, weight: FLOAT, name: CHAR[128])").unwrap();
		assert!(path.exists());
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n");
		do_exec(&mut context, "ADD OF test_table VALUES (1, 1.23, \"aaa\")").unwrap();
		do_exec(&mut context, "ADD OF test_table VALUES (1, 2.23, \"bbb\")").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n1,1.23,aaa\n1,2.23,bbb\n");
	}

	#[test]
	fn test_add_stmt_0a() {
		let path = gen_test_table_path();
		remove_file(&path);
		let mut context = Context::new();
		do_exec(&mut context, "DROP DATABASE IF EXISTS test_db").unwrap();
		do_exec(&mut context, "CREATE DATABASE test_db").unwrap();
		do_exec(&mut context, "USE test_db").unwrap();
		do_exec(&mut context, "CREATE TABLE test_table (id: INT, weight: FLOAT, name: CHAR[128])").unwrap();
		assert!(path.exists());
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n");
		do_exec(&mut context, "ADD OF test_table").unwrap();
		do_exec(&mut context, "ADD OF test_table").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n0,0.0,\n0,0.0,\n");
	}

	#[test]
	fn test_add_stmt_0() {
		let path = gen_test_table_path();
		remove_file(&path);
		let mut context = Context::new();
		do_exec(&mut context, "DROP DATABASE IF EXISTS test_db").unwrap();
		do_exec(&mut context, "CREATE DATABASE test_db").unwrap();
		do_exec(&mut context, "USE test_db").unwrap();
		do_exec(&mut context, "CREATE TABLE test_table (id: INT, weight: FLOAT, name: CHAR[128])").unwrap();
		assert!(path.exists());
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n");
		do_exec(&mut context, "ADD id = 1, weight = 3.14, name = \"hige\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 2, weight = 3.14, name = \"hoge\" OF test_table").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n1,3.14,hige\n2,3.14,hoge\n");
	}

	#[test]
	fn test_add_stmt_1() {
		let path = gen_test_table_path();
		remove_file(&path);
		let mut context = Context::new();
		do_exec(&mut context, "DROP DATABASE IF EXISTS test_db").unwrap();
		do_exec(&mut context, "CREATE DATABASE test_db").unwrap();
		do_exec(&mut context, "USE test_db").unwrap();
		do_exec(&mut context, "CREATE TABLE test_table (id: INT, weight: FLOAT, name: CHAR[128])").unwrap();
		assert!(path.exists());
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n");
		do_exec(&mut context, "ADD id = 1 OF test_table").unwrap();
		do_exec(&mut context, "ADD weight = 3.14, name = \"hoge\" OF test_table").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n1,0.0,\n0,3.14,hoge\n");
	}

	#[test]
	fn test_add_stmt_default_value() {
		let path = gen_test_table_path();
		remove_file(&path);
		let mut context = Context::new();
		do_exec(&mut context, "DROP DATABASE IF EXISTS test_db").unwrap();
		do_exec(&mut context, "CREATE DATABASE test_db").unwrap();
		do_exec(&mut context, "USE test_db").unwrap();
		do_exec(&mut context, "CREATE TABLE test_table (id: INT, weight: FLOAT DEFAULT 1.23, name: CHAR[128] DEFAULT \"def\")").unwrap();
		assert!(path.exists());
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT DEFAULT 1.23,name: CHAR[128] DEFAULT \"def\"\n");
		do_exec(&mut context, "ADD id = 1 OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 2 OF test_table").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT DEFAULT 1.23,name: CHAR[128] DEFAULT \"def\"\n1,1.23,def\n2,1.23,def\n");
	}

	#[test]
	fn test_add_stmt_check_char_size() {
		let path = gen_test_table_path();
		remove_file(&path);
		let mut context = Context::new();
		do_exec(&mut context, "DROP DATABASE IF EXISTS test_db").unwrap();
		do_exec(&mut context, "CREATE DATABASE test_db").unwrap();
		do_exec(&mut context, "USE test_db").unwrap();
		do_exec(&mut context, "CREATE TABLE test_table (id: INT, name: CHAR[4])").unwrap();
		assert!(path.exists());
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,name: CHAR[4]\n");
		do_exec(&mut context, "ADD id = 1, name = \"hige\" OF test_table").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,name: CHAR[4]\n1,hige\n");
		match do_exec(&mut context, "ADD id = 2, name = \"hogehoge\" OF test_table") {
			Ok(_) => panic!("why ok?"),
			Err(e) => eprintln!("OK: {}", e),
		}
	}

	#[test]
	fn test_add_stmt_auto_increment_id() {
		let path = gen_test_table_path();
		remove_file(&path);
		let mut context = Context::new();
		do_exec(&mut context, "DROP DATABASE IF EXISTS test_db").unwrap();
		do_exec(&mut context, "CREATE DATABASE test_db").unwrap();
		do_exec(&mut context, "USE test_db").unwrap();
		do_exec(&mut context, "CREATE TABLE test_table (id: INT AUTO_INCREMENT, name: CHAR[4])").unwrap();
		assert!(path.exists());
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT AUTO_INCREMENT,name: CHAR[4]\n");
		do_exec(&mut context, "ADD name = \"hige\" OF test_table").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT AUTO_INCREMENT,name: CHAR[4]\n1,hige\n");
		do_exec(&mut context, "ADD name = \"hoge\" OF test_table").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT AUTO_INCREMENT,name: CHAR[4]\n1,hige\n2,hoge\n");
		assert!(Path::new("test_env/test_db/id/test_table__id.txt").exists());
	}

	#[test]
	fn test_get_stmt_where_order_by_limit_0() {
		let path = gen_test_table_path();
		let mut context = Context::new();
		remove_file(&path);
		do_exec(&mut context, "DROP DATABASE IF EXISTS test_db").unwrap();
		do_exec(&mut context, "CREATE DATABASE test_db").unwrap();
		do_exec(&mut context, "USE test_db").unwrap();
		do_exec(&mut context, "CREATE TABLE test_table (id: INT, name: CHAR[128])").unwrap();
		assert!(path.exists());
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,name: CHAR[128]\n");
		do_exec(&mut context, "ADD id = 1, name = \"hoge\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 5, name = \"moge\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 6, name = \"moge\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 4, name = \"moge\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 2, name = \"moge\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 3, name = \"moge\" OF test_table").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,name: CHAR[128]
1,hoge
5,moge
6,moge
4,moge
2,moge
3,moge
");
		context.test_get_records = Some(vec![]);
		do_exec(&mut context, "get all * of test_table where id < 5 order by id limit 4").unwrap();
		let s = test_get_records_to_string(&mut context);
		assert!(s == "1,hoge
2,moge
3,moge
4,moge
");
	}

	#[test]
	fn test_get_stmt_once_where_order_by() {
		let path = gen_test_table_path();
		let mut context = Context::new();
		remove_file(&path);
		do_exec(&mut context, "DROP DATABASE IF EXISTS test_db").unwrap();
		do_exec(&mut context, "CREATE DATABASE test_db").unwrap();
		do_exec(&mut context, "USE test_db").unwrap();
		do_exec(&mut context, "CREATE TABLE test_table (id: INT, name: CHAR[128])").unwrap();
		assert!(path.exists());
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,name: CHAR[128]\n");
		do_exec(&mut context, "ADD id = 1, name = \"hoge\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 5, name = \"moge\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 6, name = \"moge\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 4, name = \"moge\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 2, name = \"moge\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 3, name = \"moge\" OF test_table").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,name: CHAR[128]
1,hoge
5,moge
6,moge
4,moge
2,moge
3,moge
");
		context.test_get_records = Some(vec![]);

		do_exec(&mut context, "get * of test_table where id < 5 order by id desc").unwrap();
		assert!(context.selected_csv_columns.len() == 2);
		assert!(context.selected_csv_columns[0] == "4");
		assert!(context.selected_csv_columns[1] == "moge");
	}

	#[test]
	fn test_get_stmt_and_expr_0() {
		let path = gen_test_table_path();
		let mut context = Context::new();
		remove_file(&path);
		do_exec(&mut context, "DROP DATABASE IF EXISTS test_db").unwrap();
		do_exec(&mut context, "CREATE DATABASE test_db").unwrap();
		do_exec(&mut context, "USE test_db").unwrap();
		do_exec(&mut context, "CREATE TABLE test_table (id: INT, weight: FLOAT, name: CHAR[128])").unwrap();
		assert!(path.exists());
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n");
		do_exec(&mut context, "ADD id = 1, weight = 3.14, name = \"hoge\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 2, weight = 3.14, name = \"moge\" OF test_table").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]
1,3.14,hoge
2,3.14,moge
");
		do_exec(&mut context, "GET id + 1, weight OF test_table").unwrap();
		assert!(context.selected_csv_columns.len() == 2);
		assert!(context.selected_csv_columns[0] == "2");
		assert!(context.selected_csv_columns[1] == "3.14");
	}


	#[test]
	fn test_get_stmt_and_expr_1() {
		let path = gen_test_table_path();
		let mut context = Context::new();
		remove_file(&path);
		do_exec(&mut context, "DROP DATABASE IF EXISTS test_db").unwrap();
		do_exec(&mut context, "CREATE DATABASE test_db").unwrap();
		do_exec(&mut context, "USE test_db").unwrap();
		do_exec(&mut context, "CREATE TABLE test_table (id: INT, weight: FLOAT, name: CHAR[128])").unwrap();
		assert!(path.exists());
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n");
		do_exec(&mut context, "ADD id = 1, weight = 3.14, name = \"hoge\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 2, weight = 3.14, name = \"moge\" OF test_table").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]
1,3.14,hoge
2,3.14,moge
");
		do_exec(&mut context, "GET weight, id + 1 * 2 OF test_table").unwrap();
		assert!(context.selected_csv_columns.len() == 2);
		assert!(context.selected_csv_columns[0] == "3.14");
		assert!(context.selected_csv_columns[1] == "3");
	}

	#[test]
	fn test_get_stmt_0a() {
		let path = gen_test_table_path();
		remove_file(&path);
		let mut context = Context::new();
		do_exec(&mut context, "DROP DATABASE IF EXISTS test_db").unwrap();
		do_exec(&mut context, "CREATE DATABASE test_db").unwrap();
		do_exec(&mut context, "USE test_db").unwrap();
		do_exec(&mut context, "CREATE TABLE test_table (id: INT, weight: FLOAT, name: CHAR[128])").unwrap();
		assert!(path.exists());
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n");
		do_exec(&mut context, "ADD id = 1, weight = 3.14, name = \"hige\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 2, weight = 3.14, name = \"hoge\" OF test_table").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n1,3.14,hige\n2,3.14,hoge\n");
		do_exec(&mut context, "GET id, name OF test_table").unwrap();
		assert!(context.selected_csv_columns.len() == 2);
		assert!(context.selected_csv_columns[0] == "1");
		assert!(context.selected_csv_columns[1] == "hige");
	}

	#[test]
	fn test_get_stmt_0b() {
		let path = gen_test_table_path();
		remove_file(&path);
		let mut context = Context::new();
		do_exec(&mut context, "DROP DATABASE IF EXISTS test_db").unwrap();
		do_exec(&mut context, "CREATE DATABASE test_db").unwrap();
		do_exec(&mut context, "USE test_db").unwrap();
		do_exec(&mut context, "CREATE TABLE test_table (id: INT, weight: FLOAT, name: CHAR[128])").unwrap();
		assert!(path.exists());
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n");
		do_exec(&mut context, "ADD id = 1, weight = 3.14, name = \"hige\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 2, weight = 3.14, name = \"hoge\" OF test_table").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n1,3.14,hige\n2,3.14,hoge\n");
		do_exec(&mut context, "GET ALL id, name OF test_table").unwrap();
		assert!(context.selected_csv_columns.len() == 2);
		assert!(context.selected_csv_columns[0] == "2");
		assert!(context.selected_csv_columns[1] == "hoge");
	}

	#[test]
	fn test_get_stmt_0c() {
		let path = gen_test_table_path();
		remove_file(&path);
		let mut context = Context::new();
		do_exec(&mut context, "DROP DATABASE IF EXISTS test_db").unwrap();
		do_exec(&mut context, "CREATE DATABASE test_db").unwrap();
		do_exec(&mut context, "USE test_db").unwrap();
		do_exec(&mut context, "CREATE TABLE test_table (id: INT, weight: FLOAT, name: CHAR[128])").unwrap();
		assert!(path.exists());
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n");
		do_exec(&mut context, "ADD id = 1, weight = 3.14, name = \"hige\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 2, weight = 3.14, name = \"hoge\" OF test_table").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n1,3.14,hige\n2,3.14,hoge\n");
		do_exec(&mut context, "GET id, name OF test_table WHERE id == 2").unwrap();
		print_selected_columns(&mut context).unwrap();
		assert!(context.selected_csv_columns.len() == 2);
		assert!(context.selected_csv_columns[0] == "2");
		assert!(context.selected_csv_columns[1] == "hoge");
	}

	#[test]
	fn test_get_stmt_0d() {
		let path = gen_test_table_path();
		remove_file(&path);
		let mut context = Context::new();
		do_exec(&mut context, "DROP DATABASE IF EXISTS test_db").unwrap();
		do_exec(&mut context, "CREATE DATABASE test_db").unwrap();
		do_exec(&mut context, "USE test_db").unwrap();
		do_exec(&mut context, "CREATE TABLE test_table (id: INT, weight: FLOAT, name: CHAR[128])").unwrap();
		assert!(path.exists());
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n");
		do_exec(&mut context, "ADD id = 1, weight = 3.14, name = \"hige\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 2, weight = 3.14, name = \"hoge\" OF test_table").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n1,3.14,hige\n2,3.14,hoge\n");
		do_exec(&mut context, "GET id, name OF test_table WHERE name == \"hoge\"").unwrap();
		assert!(context.selected_csv_columns.len() == 2);
		assert!(context.selected_csv_columns[0] == "2");
		assert!(context.selected_csv_columns[1] == "hoge");
	}

	#[test]
	fn test_get_stmt_0e() {
		let path = gen_test_table_path();
		remove_file(&path);
		let mut context = Context::new();
		do_exec(&mut context, "DROP DATABASE IF EXISTS test_db").unwrap();
		do_exec(&mut context, "CREATE DATABASE test_db").unwrap();
		do_exec(&mut context, "USE test_db").unwrap();
		do_exec(&mut context, "CREATE TABLE test_table (id: INT, weight: FLOAT, name: CHAR[128])").unwrap();
		assert!(path.exists());
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n");
		do_exec(&mut context, "ADD id = 1, weight = 3.14, name = \"hige\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 2, weight = 3.14, name = \"hoge\" OF test_table").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n1,3.14,hige\n2,3.14,hoge\n");
		do_exec(&mut context, "GET id, name OF test_table WHERE id == 1 AND name == \"hige\"").unwrap();
		assert!(context.selected_csv_columns.len() == 2);
		assert!(context.selected_csv_columns[0] == "1");
		assert!(context.selected_csv_columns[1] == "hige");
	}

	#[test]
	fn test_get_stmt_0f() {
		let path = gen_test_table_path();
		remove_file(&path);
		let mut context = Context::new();
		do_exec(&mut context, "DROP DATABASE IF EXISTS test_db").unwrap();
		do_exec(&mut context, "CREATE DATABASE test_db").unwrap();
		do_exec(&mut context, "USE test_db").unwrap();
		do_exec(&mut context, "CREATE TABLE test_table (id: INT, weight: FLOAT, name: CHAR[128])").unwrap();
		assert!(path.exists());
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n");
		do_exec(&mut context, "ADD id = 1, weight = 3.14, name = \"hige\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 2, weight = 3.14, name = \"hoge\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 3, weight = 3.14, name = \"hoge\" OF test_table").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]
1,3.14,hige
2,3.14,hoge
3,3.14,hoge
");
		do_exec(&mut context, "GET id,weight,name OF test_table WHERE id == 2").unwrap();
		assert!(context.selected_csv_columns.len() == 3);
		assert!(context.selected_csv_columns[0] == "2");
		assert!(context.selected_csv_columns[1] == "3.14");
		assert!(context.selected_csv_columns[2] == "hoge");
	}

	#[test]
	fn test_get_stmt_0g() {
		let path = gen_test_table_path();
		remove_file(&path);
		let mut context = Context::new();
		do_exec(&mut context, "DROP DATABASE IF EXISTS test_db").unwrap();
		do_exec(&mut context, "CREATE DATABASE test_db").unwrap();
		do_exec(&mut context, "USE test_db").unwrap();
		do_exec(&mut context, "CREATE TABLE test_table (id: INT, weight: FLOAT, name: CHAR[128])").unwrap();
		assert!(path.exists());
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n");
		do_exec(&mut context, "ADD id = 1, weight = 3.14, name = \"hige\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 2, weight = 3.14, name = \"hoge\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 3, weight = 3.14, name = \"hoge\" OF test_table").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]
1,3.14,hige
2,3.14,hoge
3,3.14,hoge
");
		// 2 repeat call
		do_exec(&mut context, "GET id,weight,name OF test_table WHERE id == 2").unwrap();
		assert!(context.selected_csv_columns.len() == 3);
		assert!(context.selected_csv_columns[0] == "2");
		assert!(context.selected_csv_columns[1] == "3.14");
		assert!(context.selected_csv_columns[2] == "hoge");
		do_exec(&mut context, "GET id,weight,name OF test_table WHERE id == 2").unwrap();
		assert!(context.selected_csv_columns.len() == 3);
		assert!(context.selected_csv_columns[0] == "2");
		assert!(context.selected_csv_columns[1] == "3.14");
		assert!(context.selected_csv_columns[2] == "hoge");
	}

	#[test]
	fn test_get_stmt_1() {
		let path = gen_test_table_path();
		remove_file(&path);
		let mut context = Context::new();
		do_exec(&mut context, "DROP DATABASE IF EXISTS test_db").unwrap();
		do_exec(&mut context, "CREATE DATABASE test_db").unwrap();
		do_exec(&mut context, "USE test_db").unwrap();
		do_exec(&mut context, "CREATE TABLE test_table (id: INT, weight: FLOAT, name: CHAR[128])").unwrap();
		assert!(path.exists());
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n");
		do_exec(&mut context, "ADD id = 1, weight = 3.14, name = \"hige\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 2, weight = 3.14, name = \"hoge\" OF test_table").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n1,3.14,hige\n2,3.14,hoge\n");
		do_exec(&mut context, "GET * OF test_table").unwrap();
		assert!(context.selected_csv_columns.len() == 3);
		assert!(context.selected_csv_columns[0] == "1");
		assert!(context.selected_csv_columns[1] == "3.14");
		assert!(context.selected_csv_columns[2] == "hige");
	}

	#[test]
	fn test_get_stmt_1a() {
		let path = gen_test_table_path();
		remove_file(&path);
		let mut context = Context::new();
		do_exec(&mut context, "DROP DATABASE IF EXISTS test_db").unwrap();
		do_exec(&mut context, "CREATE DATABASE test_db").unwrap();
		do_exec(&mut context, "USE test_db").unwrap();
		do_exec(&mut context, "CREATE TABLE test_table (id: INT, weight: FLOAT, name: CHAR[128])").unwrap();
		assert!(path.exists());
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n");
		do_exec(&mut context, "ADD id = 1, weight = 3.14, name = \"hige\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 2, weight = 3.14, name = \"hoge\" OF test_table").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n1,3.14,hige\n2,3.14,hoge\n");

		context.test_get_records = Some(vec![]);
		do_exec(&mut context, "GET ALL * OF test_table").unwrap();
		let s = test_get_records_to_string(&mut context);
		assert!(s == "1,3.14,hige\n2,3.14,hoge\n");
	}

	fn test_get_records_to_string(context: &mut Context) -> String {
		let mut writer = Writer::from_writer(vec![]);
		for rec in context.test_get_records.clone().unwrap().iter() {
			writer.write_record(rec).unwrap();
		}
		let bytes = writer.into_inner().unwrap();
		let csv = String::from_utf8(bytes).unwrap();
		return csv;
	}

	fn test_selected_records_to_string(context: &mut Context) -> String {
		let mut writer = Writer::from_writer(vec![]);
		for rec in context.test_selected_records.clone().unwrap().iter() {
			writer.write_record(rec).unwrap();
		}
		let bytes = writer.into_inner().unwrap();
		let csv = String::from_utf8(bytes).unwrap();
		return csv;
	}

	#[test]
	fn test_get_stmt_or_0() {
		let path = gen_test_table_path();
		remove_file(&path);
		let mut context = Context::new();
		do_exec(&mut context, "DROP DATABASE IF EXISTS test_db").unwrap();
		do_exec(&mut context, "CREATE DATABASE test_db").unwrap();
		do_exec(&mut context, "USE test_db").unwrap();
		do_exec(&mut context, "CREATE TABLE test_table (id: INT, weight: FLOAT, name: CHAR[128])").unwrap();
		assert!(path.exists());
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n");
		do_exec(&mut context, "ADD id = 1, weight = 3.14, name = \"hige\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 2, weight = 3.14, name = \"hoge\" OF test_table").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n1,3.14,hige\n2,3.14,hoge\n");
		context.test_get_records = Some(vec![]);
		do_exec(&mut context, "GET ALL id, name OF test_table WHERE id == 1 OR name == \"hoge\"").unwrap();
		let s = test_get_records_to_string(&mut context);
		assert!(s == "1,3.14,hige\n2,3.14,hoge\n");
	}

	#[test]
	fn test_get_stmt_and_0() {
		let path = gen_test_table_path();
		remove_file(&path);
		let mut context = Context::new();
		do_exec(&mut context, "DROP DATABASE IF EXISTS test_db").unwrap();
		do_exec(&mut context, "CREATE DATABASE test_db").unwrap();
		do_exec(&mut context, "USE test_db").unwrap();
		do_exec(&mut context, "CREATE TABLE test_table (id: INT, weight: FLOAT, name: CHAR[128])").unwrap();
		assert!(path.exists());
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n");
		do_exec(&mut context, "ADD id = 1, weight = 3.14, name = \"hige\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 2, weight = 3.14, name = \"hoge\" OF test_table").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n1,3.14,hige\n2,3.14,hoge\n");
		context.test_get_records = Some(vec![]);
		do_exec(&mut context, "GET ALL id, name OF test_table WHERE id == 1 AND name == \"hige\"").unwrap();
		let s = test_get_records_to_string(&mut context);
		assert!(s == "1,3.14,hige\n");
	}

	macro_rules! setup_records {
	    ($context:ident) => {
    		let path = gen_test_table_path();
			remove_file(&path);
			do_exec(&mut $context, "DROP DATABASE IF EXISTS test_db").unwrap();
			do_exec(&mut $context, "CREATE DATABASE test_db").unwrap();
			do_exec(&mut $context, "USE test_db").unwrap();
			do_exec(&mut $context, "CREATE TABLE test_table (id: INT, weight: FLOAT, name: CHAR[128])").unwrap();
			assert!(path.exists());
			let s = fs::read_to_string(&path).unwrap();
			assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n");
			do_exec(&mut $context, "ADD id = 1, weight = 3.14, name = \"hige\" OF test_table").unwrap();
			do_exec(&mut $context, "ADD id = 2, weight = 3.14, name = \"hoge\" OF test_table").unwrap();
			let s = fs::read_to_string(&path).unwrap();
			assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n1,3.14,hige\n2,3.14,hoge\n");
	    };
	}

	macro_rules! setup_records_2 {
	    ($context:ident) => {
    		let path = gen_test_table_path();
			remove_file(&path);
			do_exec(&mut $context, "DROP DATABASE IF EXISTS test_db").unwrap();
			do_exec(&mut $context, "CREATE DATABASE test_db").unwrap();
			do_exec(&mut $context, "USE test_db").unwrap();
			do_exec(&mut $context, "CREATE TABLE test_table (id: INT, weight: FLOAT, name: CHAR[128])").unwrap();
			assert!(path.exists());
			let s = fs::read_to_string(&path).unwrap();
			assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n");
			do_exec(&mut $context, "ADD id = 1, weight = 3.14, name = \"hige\" OF test_table").unwrap();
			do_exec(&mut $context, "ADD id = 2, weight = 3.14, name = \"hoge\" OF test_table").unwrap();
			do_exec(&mut $context, "ADD id = 3, weight = 3.14, name = \"moge\" OF test_table").unwrap();
			do_exec(&mut $context, "ADD id = 4, weight = 3.14, name = \"huge\" OF test_table").unwrap();
			do_exec(&mut $context, "ADD id = 5, weight = 3.14, name = \"oge\" OF test_table").unwrap();
			let s = fs::read_to_string(&path).unwrap();
			assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n1,3.14,hige\n2,3.14,hoge\n3,3.14,moge\n4,3.14,huge\n5,3.14,oge\n");
	    };
	}

	macro_rules! setup_records_3 {
	    ($context:ident) => {
    		let path = gen_test_table_path();
			remove_file(&path);
			do_exec(&mut $context, "DROP DATABASE IF EXISTS test_db").unwrap();
			do_exec(&mut $context, "CREATE DATABASE test_db").unwrap();
			do_exec(&mut $context, "USE test_db").unwrap();
			do_exec(&mut $context, "CREATE TABLE test_table (id: INT AUTO_INCREMENT, weight: FLOAT, is_login: BOOL, name: CHAR[128])").unwrap();
			assert!(path.exists());
			let s = fs::read_to_string(&path).unwrap();
			assert!(s == "id: INT AUTO_INCREMENT,weight: FLOAT,is_login: BOOL,name: CHAR[128]\n");
			do_exec(&mut $context, "ADD weight = 60.2, is_login = true, name = \"hige\" OF test_table").unwrap();
			do_exec(&mut $context, "ADD weight = 60.2, is_login = false, name = \"hoge\" OF test_table").unwrap();
			let s = fs::read_to_string(&path).unwrap();
			assert!(s == "id: INT AUTO_INCREMENT,weight: FLOAT,is_login: BOOL,name: CHAR[128]\n1,60.2,true,hige\n2,60.2,false,hoge\n");
	    };
	}

	#[test]
	fn test_get_stmt_limit_0() {
		let path = gen_test_table_path();
		let mut context = Context::new();

		setup_records_2!(context);
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n1,3.14,hige\n2,3.14,hoge\n3,3.14,moge\n4,3.14,huge\n5,3.14,oge\n");

		context.test_get_records = Some(vec![]);
		do_exec(&mut context, "GET ALL * OF test_table LIMIT 2").unwrap();
		let s = test_get_records_to_string(&mut context);
		assert!(s == "1,3.14,hige\n2,3.14,hoge\n");
	}
	
	#[test]
	fn test_get_stmt_limit_0a() {
		let path = gen_test_table_path();
		let mut context = Context::new();

		setup_records_2!(context);
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n1,3.14,hige\n2,3.14,hoge\n3,3.14,moge\n4,3.14,huge\n5,3.14,oge\n");

		context.test_get_records = Some(vec![]);
		do_exec(&mut context, "GET ALL id OF test_table LIMIT 0").unwrap();
		let s = test_get_records_to_string(&mut context);
		assert!(s == "");
	}
	
	#[test]
	fn test_get_stmt_limit_1() {
		let path = gen_test_table_path();
		let mut context = Context::new();
		remove_file(&path);
		do_exec(&mut context, "DROP DATABASE IF EXISTS test_db").unwrap();
		do_exec(&mut context, "CREATE DATABASE test_db").unwrap();
		do_exec(&mut context, "USE test_db").unwrap();
		do_exec(&mut context, "CREATE TABLE test_table (id: INT, weight: FLOAT, name: CHAR[128])").unwrap();
		assert!(path.exists());
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n");
		do_exec(&mut context, "ADD id = 1, weight = 3.14, name = \"hige\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 2, weight = 3.14, name = \"hoge\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 3, weight = 3.14, name = \"moge\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 4, weight = 3.14, name = \"hoge\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 5, weight = 3.14, name = \"oge\" OF test_table").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n1,3.14,hige\n2,3.14,hoge\n3,3.14,moge\n4,3.14,hoge\n5,3.14,oge\n");

		context.test_get_records = Some(vec![]);
		do_exec(&mut context, "GET ALL * OF test_table WHERE name == \"hoge\" LIMIT 2").unwrap();
		let s = test_get_records_to_string(&mut context);
		assert!(s == "2,3.14,hoge\n4,3.14,hoge\n");
	}
	
	#[test]
	fn test_get_stmt_or_and_0() {
		let mut context = Context::new();
		setup_records!(context);
		context.test_get_records = Some(vec![]);
		do_exec(&mut context, "GET ALL id, name OF test_table WHERE id == 1 OR weight == 3.14 AND name == \"hige\"").unwrap();
		let s = test_get_records_to_string(&mut context);
		assert!(s == "1,3.14,hige\n");
	}

	#[test]
	fn test_get_stmt_or_and_1() {
		let mut context = Context::new();
		setup_records!(context);
		context.test_get_records = Some(vec![]);
		do_exec(&mut context, "GET ALL id, name OF test_table WHERE id == 1 OR weight == 100.0 AND name == \"hige\"").unwrap();
		let s = test_get_records_to_string(&mut context);
		assert!(s == "1,3.14,hige\n");
	}

	#[test]
	fn test_get_stmt_or_and_2() {
		let mut context = Context::new();
		setup_records!(context);
		context.test_get_records = Some(vec![]);
		do_exec(&mut context, "GET ALL id, name OF test_table WHERE id == 0 OR weight == 100.0 AND name == \"hige\"").unwrap();
		let s = test_get_records_to_string(&mut context);
		assert!(s == "");
	}

	#[test]
	fn test_get_stmt_or_and_3() {
		let mut context = Context::new();
		setup_records!(context);
		context.test_get_records = Some(vec![]);
		do_exec(&mut context, "GET ALL id, name OF test_table WHERE id == 0 OR weight == 3.14 AND name == \"hige\"").unwrap();
		let s = test_get_records_to_string(&mut context);
		assert!(s == "1,3.14,hige\n");
	}

	#[test]
	fn test_get_stmt_or_and_4() {
		let mut context = Context::new();
		setup_records!(context);
		context.test_get_records = Some(vec![]);
		do_exec(&mut context, "GET ALL id, name OF test_table WHERE (id == 0 OR weight == 3.14) AND name == \"hige\"").unwrap();
		let s = test_get_records_to_string(&mut context);
		assert!(s == "1,3.14,hige\n");
	}

	#[test]
	fn test_get_stmt_or_and_4a() {
		let mut context = Context::new();
		setup_records!(context);
		context.test_get_records = Some(vec![]);
		do_exec(&mut context, "GET ALL id, name OF test_table WHERE (id == 1 OR weight == 60) AND name == \"hige\"").unwrap();
		let s = test_get_records_to_string(&mut context);
		assert!(s == "1,3.14,hige\n");
	}

	#[test]
	fn test_get_stmt_or_and_5() {
		let mut context = Context::new();
		setup_records!(context);
		context.test_get_records = Some(vec![]);
		do_exec(&mut context, "GET ALL id, name OF test_table WHERE (id == 0 OR weight == 3.14) AND name == \"moge\"").unwrap();
		let s = test_get_records_to_string(&mut context);
		assert!(s == "");
	}

	#[test]
	fn test_drop_db() {
		let mut context = Context::new();
		do_exec(&mut context, "DROP DATABASE IF EXISTS test_db").unwrap();
		do_exec(&mut context, "CREATE DATABASE test_db").unwrap();
		assert!(Path::new("test_env").join("test_db").exists());
		do_exec(&mut context, "DROP DATABASE IF EXISTS test_db").unwrap();
		assert!(!Path::new("test_env").join("test_db").exists());
	}

	#[test]
	fn test_drop_table() {
		let path = gen_test_table_path();
		remove_file(&path);
		let mut context = Context::new();
		do_exec(&mut context, "DROP DATABASE IF EXISTS test_db").unwrap();
		do_exec(&mut context, "CREATE DATABASE test_db").unwrap();
		do_exec(&mut context, "USE test_db").unwrap();
		do_exec(&mut context, "CREATE TABLE test_table (id: INT, weight: FLOAT, name: CHAR[128])").unwrap();
		assert!(path.exists());
		do_exec(&mut context, "DROP TABLE test_table").unwrap();
		assert!(!path.exists());
	}

	#[test]
	fn test_del_stmt_0() {
		let path = gen_test_table_path();
		remove_file(&path);
		let mut context = Context::new();
		do_exec(&mut context, "DROP DATABASE IF EXISTS test_db").unwrap();
		do_exec(&mut context, "CREATE DATABASE test_db").unwrap();
		do_exec(&mut context, "USE test_db").unwrap();
		do_exec(&mut context, "CREATE TABLE test_table (id: INT, weight: FLOAT, name: CHAR[128])").unwrap();
		do_exec(&mut context, "ADD id = 1, weight = 3.14, name = \"hige\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 2, weight = 3.14, name = \"hoge\" OF test_table").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n1,3.14,hige\n2,3.14,hoge\n");
		do_exec(&mut context, "DEL ALL OF test_table").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n");
	}

	#[test]
	fn test_del_stmt_1() {
		let path = gen_test_table_path();
		remove_file(&path);
		let mut context = Context::new();
		do_exec(&mut context, "DROP DATABASE IF EXISTS test_db").unwrap();
		do_exec(&mut context, "CREATE DATABASE test_db").unwrap();
		do_exec(&mut context, "USE test_db").unwrap();
		do_exec(&mut context, "CREATE TABLE test_table (id: INT, weight: FLOAT, name: CHAR[128])").unwrap();
		do_exec(&mut context, "ADD id = 1, weight = 3.14, name = \"hige\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 2, weight = 3.14, name = \"hoge\" OF test_table").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n1,3.14,hige\n2,3.14,hoge\n");
		do_exec(&mut context, "DEL ALL OF test_table WHERE id == 1").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n2,3.14,hoge\n");
	}

	#[test]
	fn test_del_stmt_2() {
		let path = gen_test_table_path();
		remove_file(&path);
		let mut context = Context::new();
		do_exec(&mut context, "DROP DATABASE IF EXISTS test_db").unwrap();
		do_exec(&mut context, "CREATE DATABASE test_db").unwrap();
		do_exec(&mut context, "USE test_db").unwrap();
		do_exec(&mut context, "CREATE TABLE test_table (id: INT, weight: FLOAT, name: CHAR[128])").unwrap();
		do_exec(&mut context, "ADD id = 1, weight = 3.14, name = \"hige\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 2, weight = 3.14, name = \"hoge\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 3, weight = 3.14, name = \"moge\" OF test_table").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n1,3.14,hige\n2,3.14,hoge\n3,3.14,moge\n");
		do_exec(&mut context, "DEL ALL OF test_table WHERE id == 2").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n1,3.14,hige\n3,3.14,moge\n");
	}

	#[test]
	fn test_del_stmt_3() {
		let path = gen_test_table_path();
		let mut context = Context::new();

		setup_records_2!(context);
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n1,3.14,hige\n2,3.14,hoge\n3,3.14,moge\n4,3.14,huge\n5,3.14,oge\n");

		do_exec(&mut context, "DEL ALL OF test_table WHERE id == 1").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n2,3.14,hoge\n3,3.14,moge\n4,3.14,huge\n5,3.14,oge\n");

		do_exec(&mut context, "DEL ALL OF test_table WHERE name == \"oge\"").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n2,3.14,hoge\n3,3.14,moge\n4,3.14,huge\n");

		do_exec(&mut context, "DEL ALL OF test_table WHERE weight == 3.14").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n");
	}

	#[test]
	fn test_del_stmt_4() {
		let path = gen_test_table_path();
		let mut context = Context::new();

		setup_records_2!(context);
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n1,3.14,hige\n2,3.14,hoge\n3,3.14,moge\n4,3.14,huge\n5,3.14,oge\n");

		do_exec(&mut context, "DEL OF test_table").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n2,3.14,hoge\n3,3.14,moge\n4,3.14,huge\n5,3.14,oge\n");
	}

	#[test]
	fn test_del_stmt_5() {
		let path = gen_test_table_path();
		let mut context = Context::new();

		setup_records_2!(context);
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n1,3.14,hige\n2,3.14,hoge\n3,3.14,moge\n4,3.14,huge\n5,3.14,oge\n");

		do_exec(&mut context, "DEL OF test_table WHERE id == 2").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n1,3.14,hige\n3,3.14,moge\n4,3.14,huge\n5,3.14,oge\n");
	}

	#[test]
	fn test_del_stmt_limit_0() {
		let path = gen_test_table_path();
		let mut context = Context::new();

		setup_records_2!(context);
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n1,3.14,hige\n2,3.14,hoge\n3,3.14,moge\n4,3.14,huge\n5,3.14,oge\n");

		context.test_get_records = Some(vec![]);
		do_exec(&mut context, "DEL ALL OF test_table LIMIT 2").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]
3,3.14,moge
4,3.14,huge
5,3.14,oge
");
	}
	
	#[test]
	fn test_del_stmt_limit_1() {
		let path = gen_test_table_path();
		let mut context = Context::new();
		remove_file(&path);
		do_exec(&mut context, "DROP DATABASE IF EXISTS test_db").unwrap();
		do_exec(&mut context, "CREATE DATABASE test_db").unwrap();
		do_exec(&mut context, "USE test_db").unwrap();
		do_exec(&mut context, "CREATE TABLE test_table (id: INT, weight: FLOAT, name: CHAR[128])").unwrap();
		assert!(path.exists());
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n");
		do_exec(&mut context, "ADD id = 1, weight = 3.14, name = \"hige\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 2, weight = 3.14, name = \"hoge\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 3, weight = 3.14, name = \"moge\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 4, weight = 3.14, name = \"hoge\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 5, weight = 3.14, name = \"oge\" OF test_table").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]
1,3.14,hige
2,3.14,hoge
3,3.14,moge
4,3.14,hoge
5,3.14,oge
");

		do_exec(&mut context, "DEL ALL OF test_table WHERE name == \"hoge\" LIMIT 2").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]
1,3.14,hige
3,3.14,moge
5,3.14,oge
");
	}
	
	#[test]
	fn test_set_stmt_0() {
		let path = gen_test_table_path();
		remove_file(&path);
		let mut context = Context::new();
		do_exec(&mut context, "DROP DATABASE IF EXISTS test_db").unwrap();
		do_exec(&mut context, "CREATE DATABASE test_db").unwrap();
		do_exec(&mut context, "USE test_db").unwrap();
		do_exec(&mut context, "CREATE TABLE test_table (id: INT, weight: FLOAT, name: CHAR[128])").unwrap();
		do_exec(&mut context, "ADD id = 1, weight = 3.14, name = \"hige\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 2, weight = 3.14, name = \"hoge\" OF test_table").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n1,3.14,hige\n2,3.14,hoge\n");
		do_exec(&mut context, "SET ALL id=10 OF test_table WHERE weight == 3.14").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n10,3.14,hige\n10,3.14,hoge\n");
	}

	#[test]
	fn test_set_stmt_0a() {
		let path = gen_test_table_path();
		remove_file(&path);
		let mut context = Context::new();
		do_exec(&mut context, "DROP DATABASE IF EXISTS test_db").unwrap();
		do_exec(&mut context, "CREATE DATABASE test_db").unwrap();
		do_exec(&mut context, "USE test_db").unwrap();
		do_exec(&mut context, "CREATE TABLE test_table (id: INT, weight: FLOAT, name: CHAR[128])").unwrap();
		do_exec(&mut context, "ADD id = 1, weight = 3.14, name = \"hige\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 2, weight = 3.14, name = \"hoge\" OF test_table").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n1,3.14,hige\n2,3.14,hoge\n");
		do_exec(&mut context, "SET ALL id=10, name=\"HOGE\" OF test_table WHERE weight == 1234").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n1,3.14,hige\n2,3.14,hoge\n");
	}

	#[test]
	fn test_set_stmt_1() {
		let path = gen_test_table_path();
		let mut context = Context::new();
		setup_records!(context);
		do_exec(&mut context, "SET id=10, name=\"HOGE\" OF test_table WHERE weight == 3.14").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n10,3.14,HOGE\n2,3.14,hoge\n");
	}

	#[test]
	fn test_set_stmt_2() {
		let path = gen_test_table_path();
		let mut context = Context::new();
		setup_records!(context);
		do_exec(&mut context, "SET id=10, name=\"HOGE\" OF test_table WHERE name == \"hoge\"").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n1,3.14,hige\n10,3.14,HOGE\n");
	}

	#[test]
	fn test_set_stmt_3() {
		let path = gen_test_table_path();
		let mut context = Context::new();
		setup_records!(context);
		do_exec(&mut context, "SET ALL name=\"HOGE\" OF test_table WHERE weight == 3.14").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n1,3.14,HOGE\n2,3.14,HOGE\n");
	}

	#[test]
	fn test_set_stmt_4() {
		let path = gen_test_table_path();
		let mut context = Context::new();
		setup_records!(context);
		do_exec(&mut context, "SET ALL name=\"HOGE\" OF test_table").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n1,3.14,HOGE\n2,3.14,HOGE\n");
	}

	#[test]
	fn test_set_stmt_limit_0() {
		let path = gen_test_table_path();
		let mut context = Context::new();

		setup_records_2!(context);
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n1,3.14,hige\n2,3.14,hoge\n3,3.14,moge\n4,3.14,huge\n5,3.14,oge\n");

		context.test_get_records = Some(vec![]);
		do_exec(&mut context, "SET ALL weight = 1.23 OF test_table LIMIT 2").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]
1,1.23,hige
2,1.23,hoge
3,3.14,moge
4,3.14,huge
5,3.14,oge
");
	}
	
	#[test]
	fn test_set_stmt_limit_0a() {
		let path = gen_test_table_path();
		let mut context = Context::new();

		setup_records_2!(context);
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n1,3.14,hige\n2,3.14,hoge\n3,3.14,moge\n4,3.14,huge\n5,3.14,oge\n");

		context.test_get_records = Some(vec![]);
		do_exec(&mut context, "SET ALL weight = 1.23 OF test_table LIMIT 0").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]
1,3.14,hige
2,3.14,hoge
3,3.14,moge
4,3.14,huge
5,3.14,oge
");
	}
	
	#[test]
	fn test_set_stmt_limit_1() {
		let path = gen_test_table_path();
		let mut context = Context::new();
		remove_file(&path);
		do_exec(&mut context, "DROP DATABASE IF EXISTS test_db").unwrap();
		do_exec(&mut context, "CREATE DATABASE test_db").unwrap();
		do_exec(&mut context, "USE test_db").unwrap();
		do_exec(&mut context, "CREATE TABLE test_table (id: INT, weight: FLOAT, name: CHAR[128])").unwrap();
		assert!(path.exists());
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n");
		do_exec(&mut context, "ADD id = 1, weight = 3.14, name = \"hige\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 2, weight = 3.14, name = \"hoge\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 3, weight = 3.14, name = \"moge\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 4, weight = 3.14, name = \"hoge\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 5, weight = 3.14, name = \"oge\" OF test_table").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n1,3.14,hige\n2,3.14,hoge\n3,3.14,moge\n4,3.14,hoge\n5,3.14,oge\n");

		do_exec(&mut context, "SET ALL weight = 1.23 OF test_table WHERE name == \"hoge\" LIMIT 2").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]
1,3.14,hige
2,1.23,hoge
3,3.14,moge
4,1.23,hoge
5,3.14,oge
");
	}
	
	#[test]
	fn test_alter_column_type_0a() {
		let path = gen_test_table_path();
		let mut context = Context::new();
		setup_records_2!(context);
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n1,3.14,hige\n2,3.14,hoge\n3,3.14,moge\n4,3.14,huge\n5,3.14,oge\n");

		match do_exec(&mut context, "ALTER TABLE test_table ALTER COLUMN id TYPE FLOAT AUTO_INCREMENT") {
			Ok(_) => panic!("failed"),
			Err(_) => {},
		}
	}

	#[test]
	fn test_alter_column_type_0() {
		let path = gen_test_table_path();
		let mut context = Context::new();
		setup_records_2!(context);
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n1,3.14,hige\n2,3.14,hoge\n3,3.14,moge\n4,3.14,huge\n5,3.14,oge\n");

		do_exec(&mut context, "ALTER TABLE test_table ALTER COLUMN id TYPE INT AUTO_INCREMENT PRIMARY_KEY").unwrap();

		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT PRIMARY_KEY AUTO_INCREMENT,weight: FLOAT,name: CHAR[128]
1,3.14,hige
2,3.14,hoge
3,3.14,moge
4,3.14,huge
5,3.14,oge
");
	}

	#[test]
	fn test_alter_column_type_1() {
		let path = gen_test_table_path();
		let mut context = Context::new();
		remove_file(&path);
		do_exec(&mut context, "DROP DATABASE IF EXISTS test_db").unwrap();
		do_exec(&mut context, "CREATE DATABASE test_db").unwrap();
		do_exec(&mut context, "USE test_db").unwrap();
		do_exec(&mut context, "CREATE TABLE test_table (id: INT, weight: FLOAT, name: CHAR[128])").unwrap();
		assert!(path.exists());
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n");
		do_exec(&mut context, "ADD id = 1, weight = 3.14, name = \"hige\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 2, weight = 3.14, name = \"hoge\" OF test_table").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n1,3.14,hige\n2,3.14,hoge\n");

		do_exec(&mut context, "ALTER TABLE test_table ALTER COLUMN name TYPE CHAR[10]").unwrap();

		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[10]
1,3.14,hige
2,3.14,hoge
");
	}

	#[test]
	fn test_alter_column_type_2() {
		let path = gen_test_table_path();
		let mut context = Context::new();
		remove_file(&path);
		do_exec(&mut context, "DROP DATABASE IF EXISTS test_db").unwrap();
		do_exec(&mut context, "CREATE DATABASE test_db").unwrap();
		do_exec(&mut context, "USE test_db").unwrap();
		do_exec(&mut context, "CREATE TABLE test_table (id: INT, weight: FLOAT, name: CHAR[128])").unwrap();
		assert!(path.exists());
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n");
		do_exec(&mut context, "ADD id = 1, weight = 3.14, name = \"hige\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 2, weight = 3.14, name = \"hoge\" OF test_table").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n1,3.14,hige\n2,3.14,hoge\n");

		match do_exec(&mut context, "ALTER TABLE test_table ALTER COLUMN name TYPE CHAR[2]") {
			Ok(_) => panic!("failed"),
			Err(_) => {},
		}
	}

	#[test]
	fn test_alter_column_type_2a() {
		let path = gen_test_table_path();
		let mut context = Context::new();
		remove_file(&path);
		do_exec(&mut context, "DROP DATABASE IF EXISTS test_db").unwrap();
		do_exec(&mut context, "CREATE DATABASE test_db").unwrap();
		do_exec(&mut context, "USE test_db").unwrap();
		do_exec(&mut context, "CREATE TABLE test_table (id: INT, weight: FLOAT, name: CHAR[128])").unwrap();
		assert!(path.exists());
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n");
		do_exec(&mut context, "ADD id = 1, weight = 3.14, name = \"hige\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 2, weight = 3.14, name = \"hoge\" OF test_table").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n1,3.14,hige\n2,3.14,hoge\n");

		match do_exec(&mut context, "ALTER TABLE test_table ALTER COLUMN weight TYPE INT") {
			Ok(_) => panic!("failed"),
			Err(_) => {},
		}
	}

	#[test]
	fn test_alter_column_type_3() {
		let path = gen_test_table_path();
		let mut context = Context::new();
		remove_file(&path);
		do_exec(&mut context, "DROP DATABASE IF EXISTS test_db").unwrap();
		do_exec(&mut context, "CREATE DATABASE test_db").unwrap();
		do_exec(&mut context, "USE test_db").unwrap();
		do_exec(&mut context, "CREATE TABLE test_table (id: INT, weight: FLOAT, name: CHAR[128])").unwrap();
		assert!(path.exists());
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n");
		do_exec(&mut context, "ADD id = 1, weight = 3.14, name = \"hige\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 2, weight = 3.14, name = \"hoge\" OF test_table").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n1,3.14,hige\n2,3.14,hoge\n");

		do_exec(&mut context, "ALTER TABLE test_table ALTER COLUMN id TYPE FLOAT").unwrap();

		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: FLOAT,weight: FLOAT,name: CHAR[128]
1,3.14,hige
2,3.14,hoge
");
	}

	#[test]
	fn test_alter_add_column_0() {
		let path = gen_test_table_path();
		let mut context = Context::new();
		setup_records_2!(context);
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n1,3.14,hige\n2,3.14,hoge\n3,3.14,moge\n4,3.14,huge\n5,3.14,oge\n");
		do_exec(&mut context, "ALTER TABLE test_table ADD COLUMN uge INT").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128],uge: INT\n1,3.14,hige,0\n2,3.14,hoge,0\n3,3.14,moge,0\n4,3.14,huge,0\n5,3.14,oge,0\n");
	}

	#[test]
	fn test_alter_add_column_1() {
		let path = gen_test_table_path();
		let mut context = Context::new();
		setup_records_2!(context);
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n1,3.14,hige\n2,3.14,hoge\n3,3.14,moge\n4,3.14,huge\n5,3.14,oge\n");
		do_exec(&mut context, "ALTER TABLE test_table ADD COLUMN uge INT DEFAULT 100").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128],uge: INT DEFAULT 100\n1,3.14,hige,100\n2,3.14,hoge,100\n3,3.14,moge,100\n4,3.14,huge,100\n5,3.14,oge,100\n");
	}

	#[test]
	fn test_where_lt() {
		let path = gen_test_table_path();
		let mut context = Context::new();
		setup_records_2!(context);
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n1,3.14,hige\n2,3.14,hoge\n3,3.14,moge\n4,3.14,huge\n5,3.14,oge\n");
		context.test_get_records = Some(vec![]);
		do_exec(&mut context, "GET ALL id,weight OF test_table WHERE id < 3").unwrap();
		let s = test_get_records_to_string(&mut context);
		assert!(s == "1,3.14,hige\n2,3.14,hoge\n");
	}

	#[test]
	fn test_where_lteq() {
		let path = gen_test_table_path();
		let mut context = Context::new();
		setup_records_2!(context);
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n1,3.14,hige\n2,3.14,hoge\n3,3.14,moge\n4,3.14,huge\n5,3.14,oge\n");
		context.test_get_records = Some(vec![]);
		do_exec(&mut context, "GET ALL id,weight OF test_table WHERE id <= 3").unwrap();
		let s = test_get_records_to_string(&mut context);
		assert!(s == "1,3.14,hige\n2,3.14,hoge\n3,3.14,moge\n");
	}

	#[test]
	fn test_where_gt() {
		let path = gen_test_table_path();
		let mut context = Context::new();
		setup_records_2!(context);
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n1,3.14,hige\n2,3.14,hoge\n3,3.14,moge\n4,3.14,huge\n5,3.14,oge\n");
		context.test_get_records = Some(vec![]);
		do_exec(&mut context, "GET ALL id,weight OF test_table WHERE id > 2").unwrap();
		let s = test_get_records_to_string(&mut context);
		assert!(s == "3,3.14,moge\n4,3.14,huge\n5,3.14,oge\n");
	}

	#[test]
	fn test_where_gteq() {
		let path = gen_test_table_path();
		let mut context = Context::new();
		setup_records_2!(context);
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n1,3.14,hige\n2,3.14,hoge\n3,3.14,moge\n4,3.14,huge\n5,3.14,oge\n");
		context.test_get_records = Some(vec![]);
		do_exec(&mut context, "GET ALL id,weight OF test_table WHERE id >= 3").unwrap();
		let s = test_get_records_to_string(&mut context);
		assert!(s == "3,3.14,moge\n4,3.14,huge\n5,3.14,oge\n");
	}

	#[test]
	fn test_bool_0() {
		let path = gen_test_table_path();
		let mut context = Context::new();

		setup_records_3!(context);
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT AUTO_INCREMENT,weight: FLOAT,is_login: BOOL,name: CHAR[128]\n1,60.2,true,hige\n2,60.2,false,hoge\n");

		context.test_get_records = Some(vec![]);
		do_exec(&mut context, "GET ALL * OF test_table").unwrap();
		let s = test_get_records_to_string(&mut context);
		assert!(s == "1,60.2,true,hige\n2,60.2,false,hoge\n");
	}

	#[test]
	fn test_bool_1() {
		let path = gen_test_table_path();
		let mut context = Context::new();

		setup_records_3!(context);
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT AUTO_INCREMENT,weight: FLOAT,is_login: BOOL,name: CHAR[128]\n1,60.2,true,hige\n2,60.2,false,hoge\n");

		context.test_get_records = Some(vec![]);
		do_exec(&mut context, "GET ALL * OF test_table WHERE is_login == true").unwrap();
		let s = test_get_records_to_string(&mut context);
		assert!(s == "1,60.2,true,hige\n");
	}

	#[test]
	fn test_bool_2() {
		let path = gen_test_table_path();
		let mut context = Context::new();

		setup_records_3!(context);
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT AUTO_INCREMENT,weight: FLOAT,is_login: BOOL,name: CHAR[128]\n1,60.2,true,hige\n2,60.2,false,hoge\n");

		context.test_get_records = Some(vec![]);
		do_exec(&mut context, "GET ALL * OF test_table WHERE is_login == false").unwrap();
		let s = test_get_records_to_string(&mut context);
		assert!(s == "2,60.2,false,hoge\n");
	}

	#[test]
	fn test_bool_3() {
		let path = gen_test_table_path();
		let mut context = Context::new();

		setup_records_3!(context);
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT AUTO_INCREMENT,weight: FLOAT,is_login: BOOL,name: CHAR[128]\n1,60.2,true,hige\n2,60.2,false,hoge\n");

		context.test_get_records = Some(vec![]);
		do_exec(&mut context, "GET ALL * OF test_table WHERE is_login != true").unwrap();
		let s = test_get_records_to_string(&mut context);
		assert!(s == "2,60.2,false,hoge\n");
	}

	#[test]
	fn test_alter_drop_column_0() {
		let path = gen_test_table_path();
		let mut context = Context::new();

		setup_records_2!(context);
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n1,3.14,hige\n2,3.14,hoge\n3,3.14,moge\n4,3.14,huge\n5,3.14,oge\n");

		do_exec(&mut context, "ALTER TABLE test_table DROP COLUMN id").unwrap();

		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "weight: FLOAT,name: CHAR[128]\n3.14,hige\n3.14,hoge\n3.14,moge\n3.14,huge\n3.14,oge\n");
	}

	#[test]
	fn test_alter_drop_column_1() {
		let path = gen_test_table_path();
		let mut context = Context::new();

		setup_records_2!(context);
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n1,3.14,hige\n2,3.14,hoge\n3,3.14,moge\n4,3.14,huge\n5,3.14,oge\n");

		do_exec(&mut context, "ALTER TABLE test_table DROP COLUMN weight").unwrap();

		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,name: CHAR[128]\n1,hige\n2,hoge\n3,moge\n4,huge\n5,oge\n");
	}

	#[test]
	fn test_alter_drop_column_2() {
		let path = gen_test_table_path();
		let mut context = Context::new();

		setup_records_2!(context);
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n1,3.14,hige\n2,3.14,hoge\n3,3.14,moge\n4,3.14,huge\n5,3.14,oge\n");

		do_exec(&mut context, "ALTER TABLE test_table DROP COLUMN name").unwrap();

		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT\n1,3.14\n2,3.14\n3,3.14\n4,3.14\n5,3.14\n");
	}

	#[test]
	fn test_alter_drop_column_3() {
		let path = gen_test_table_path();
		let mut context = Context::new();

		setup_records_2!(context);
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n1,3.14,hige\n2,3.14,hoge\n3,3.14,moge\n4,3.14,huge\n5,3.14,oge\n");

		match do_exec(&mut context, "ALTER TABLE test_table DROP COLUMN nothing") {
			Ok(_) => panic!("failed"),
			Err(_) => {}
		}
	}

	#[test]
	fn test_alter_rename_column_0() {
		let path = gen_test_table_path();
		let mut context = Context::new();

		setup_records_2!(context);
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n1,3.14,hige\n2,3.14,hoge\n3,3.14,moge\n4,3.14,huge\n5,3.14,oge\n");

		do_exec(&mut context, "ALTER TABLE test_table RENAME COLUMN id TO user_id").unwrap();

		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "user_id: INT,weight: FLOAT,name: CHAR[128]
1,3.14,hige
2,3.14,hoge
3,3.14,moge
4,3.14,huge
5,3.14,oge
");
	}

	#[test]
	fn test_alter_rename_table_0() {
		let path = gen_test_table_path();
		let mut context = Context::new();

		setup_records_2!(context);
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n1,3.14,hige\n2,3.14,hoge\n3,3.14,moge\n4,3.14,huge\n5,3.14,oge\n");

		do_exec(&mut context, "ALTER TABLE test_table RENAME TO new_table").unwrap();
		assert!(Path::new("test_env/test_db/tables/new_table.csv").exists());
		assert!(!Path::new("test_env/test_db/tables/test_table.csv").exists());
	}

	#[test]
	fn test_alter_rename_table_error_0() {
		let path = gen_test_table_path();
		let mut context = Context::new();

		setup_records_2!(context);
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n1,3.14,hige\n2,3.14,hoge\n3,3.14,moge\n4,3.14,huge\n5,3.14,oge\n");

		match do_exec(&mut context, "ALTER TABLE nothing RENAME TO new_table") {
			Ok(_) => panic!("failed"),
			Err(_) => {},
		}
	}

	#[test]
	fn test_alter_rename_table_error_1() {
		let path = gen_test_table_path();
		let mut context = Context::new();

		setup_records_2!(context);
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n1,3.14,hige\n2,3.14,hoge\n3,3.14,moge\n4,3.14,huge\n5,3.14,oge\n");

		match do_exec(&mut context, "ALTER TABLE test_table RENAME TO test_table") {
			Ok(_) => panic!("failed"),
			Err(_) => {},
		}
	}

	#[test]
	fn test_dot_chain_0() {
		let path = gen_test_table_path();
		let mut context = Context::new();

		setup_records_2!(context);
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n1,3.14,hige\n2,3.14,hoge\n3,3.14,moge\n4,3.14,huge\n5,3.14,oge\n");

		context.test_get_records = Some(vec![]);
		do_exec(&mut context, "GET ALL test_table.id OF test_table").unwrap();

		let s = test_get_records_to_string(&mut context);
		println!("s[{}]", s);
		assert!(s == "1,3.14,hige\n2,3.14,hoge\n3,3.14,moge\n4,3.14,huge\n5,3.14,oge\n");
	}

	#[test]
	fn test_dot_chain_1() {
		let path = gen_test_table_path();
		let mut context = Context::new();

		setup_records_2!(context);
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n1,3.14,hige\n2,3.14,hoge\n3,3.14,moge\n4,3.14,huge\n5,3.14,oge\n");

		context.test_get_records = Some(vec![]);
		do_exec(&mut context, "GET ALL test_table.id OF test_table WHERE test_table.id < 3").unwrap();

		let s = test_get_records_to_string(&mut context);
		println!("s[{}]", s);
		assert!(s == "1,3.14,hige\n2,3.14,hoge\n");
	}

	#[test]
	fn test_add_sub_expr_0() {
		let path = gen_test_table_path();
		let mut context = Context::new();
		remove_file(&path);
		do_exec(&mut context, "DROP DATABASE IF EXISTS test_db").unwrap();
		do_exec(&mut context, "CREATE DATABASE test_db").unwrap();
		do_exec(&mut context, "USE test_db").unwrap();
		do_exec(&mut context, "CREATE TABLE test_table (id: INT, weight: FLOAT, name: CHAR[128])").unwrap();
		assert!(path.exists());
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n");
		do_exec(&mut context, "ADD id = 1 + 1, weight = 3.14, name = \"hige\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 2 - 1, weight = 3.14, name = \"hoge\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 2 - 1 + 1, weight = 3.14, name = \"moge\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 1 + (2 + 3), weight = 3.14, name = \"hoge\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 4 - (2 - 1), weight = 3.14, name = \"oge\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 4 - 2 - 1, weight = 3.14, name = \"oge\" OF test_table").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]
2,3.14,hige
1,3.14,hoge
2,3.14,moge
6,3.14,hoge
3,3.14,oge
1,3.14,oge
");
	}

	#[test]
	fn test_mul_div_expr_0() {
		let path = gen_test_table_path();
		let mut context = Context::new();
		remove_file(&path);
		do_exec(&mut context, "DROP DATABASE IF EXISTS test_db").unwrap();
		do_exec(&mut context, "CREATE DATABASE test_db").unwrap();
		do_exec(&mut context, "USE test_db").unwrap();
		do_exec(&mut context, "CREATE TABLE test_table (id: INT, weight: FLOAT, name: CHAR[128])").unwrap();
		assert!(path.exists());
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n");
		do_exec(&mut context, "ADD id = 2 * 2, weight = 3.14, name = \"hige\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 2 * 2 / 2, weight = 3.14, name = \"hoge\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 3 / 2, weight = 3.14, name = \"moge\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 2 * (2 * 3), weight = 3.14, name = \"hoge\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 4 / (4 / 2), weight = 3.14, name = \"oge\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = (4 * 2) / 2, weight = 3.14, name = \"oge\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 4 % 2, weight = 3.14, name = \"oge\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 2 * 3 % 2, weight = 3.14, name = \"oge\" OF test_table").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]
4,3.14,hige
2,3.14,hoge
1,3.14,moge
12,3.14,hoge
2,3.14,oge
4,3.14,oge
0,3.14,oge
0,3.14,oge
");
	}

	#[test]
	fn test_add_sub_mul_div_0() {
		let path = gen_test_table_path();
		let mut context = Context::new();
		remove_file(&path);
		do_exec(&mut context, "DROP DATABASE IF EXISTS test_db").unwrap();
		do_exec(&mut context, "CREATE DATABASE test_db").unwrap();
		do_exec(&mut context, "USE test_db").unwrap();
		do_exec(&mut context, "CREATE TABLE test_table (id: INT, weight: FLOAT, name: CHAR[128])").unwrap();
		assert!(path.exists());
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n");
		do_exec(&mut context, "ADD id = 1 + 2 * 3, weight = 3.14, name = \"hige\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 2 * 3 + 2, weight = 3.14, name = \"hoge\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 1 + 4 / 2, weight = 3.14, name = \"moge\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 2 - 2 * 3, weight = 3.14, name = \"hoge\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 2 - 4 / 2, weight = 3.14, name = \"oge\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 1 + 2 - 1 * 3 / 2, weight = 3.14, name = \"oge\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 3 - 2 + 1, weight = 3.14, name = \"oge\" OF test_table").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]
7,3.14,hige
8,3.14,hoge
3,3.14,moge
-4,3.14,hoge
0,3.14,oge
2,3.14,oge
2,3.14,oge
");
	}

	#[test]
	fn test_show_tables() {
		let path = gen_test_table_path();
		let mut context = Context::new();
		remove_file(&path);
		do_exec(&mut context, "DROP DATABASE IF EXISTS test_db").unwrap();
		do_exec(&mut context, "CREATE DATABASE test_db").unwrap();
		do_exec(&mut context, "USE test_db").unwrap();
		do_exec(&mut context, "SHOW TABLES").unwrap();
	}

	#[test]
	fn test_show_databases() {
		let path = gen_test_table_path();
		let mut context = Context::new();
		remove_file(&path);
		do_exec(&mut context, "SHOW DATABASES").unwrap();
	}

	#[test]
	fn test_sum_func() {
		let path = gen_test_table_path();
		let mut context = Context::new();
		remove_file(&path);
		do_exec(&mut context, "DROP DATABASE IF EXISTS test_db").unwrap();
		do_exec(&mut context, "CREATE DATABASE test_db").unwrap();
		do_exec(&mut context, "USE test_db").unwrap();
		do_exec(&mut context, "CREATE TABLE test_table (id: INT, weight: FLOAT, name: CHAR[128])").unwrap();
		assert!(path.exists());
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n");
		do_exec(&mut context, "ADD id = 5, weight = 3.14, name = \"hige\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 1, weight = 3.14, name = \"hoge\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 2, weight = 3.14, name = \"moge\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 4, weight = 3.14, name = \"hoge\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 3, weight = 3.14, name = \"oge\" OF test_table").unwrap();

		context.test_get_records = Some(vec![]);
		do_exec(&mut context, "GET ALL SUM(id) OF test_table").unwrap();

		assert!(context.selected_csv_columns.len() == 1);
		assert!(context.selected_csv_columns[0] == "15");
	}

	#[test]
	fn test_sum_func_1() {
		let path = gen_test_table_path();
		let mut context = Context::new();
		remove_file(&path);
		do_exec(&mut context, "DROP DATABASE IF EXISTS test_db").unwrap();
		do_exec(&mut context, "CREATE DATABASE test_db").unwrap();
		do_exec(&mut context, "USE test_db").unwrap();
		do_exec(&mut context, "CREATE TABLE test_table (id: INT, weight: FLOAT, name: CHAR[128])").unwrap();
		assert!(path.exists());
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n");
		do_exec(&mut context, "ADD id = 5, weight = 3.14, name = \"hige\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 1, weight = 3.14, name = \"hoge\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 2, weight = 3.14, name = \"moge\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 4, weight = 3.14, name = \"hoge\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 3, weight = 3.14, name = \"oge\" OF test_table").unwrap();

		context.test_get_records = Some(vec![]);
		do_exec(&mut context, "GET ALL SUM(id) OF test_table LIMIT 3").unwrap();

		assert!(context.selected_csv_columns.len() == 1);
		assert!(context.selected_csv_columns[0] == "8");
	}

	#[test]
	fn test_order_by_limit() {
		let path = gen_test_table_path();
		let mut context = Context::new();
		remove_file(&path);
		do_exec(&mut context, "DROP DATABASE IF EXISTS test_db").unwrap();
		do_exec(&mut context, "CREATE DATABASE test_db").unwrap();
		do_exec(&mut context, "USE test_db").unwrap();
		do_exec(&mut context, "CREATE TABLE test_table (id: INT, weight: FLOAT, name: CHAR[128])").unwrap();
		assert!(path.exists());
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n");
		do_exec(&mut context, "ADD id = 5, weight = 3.14, name = \"hige\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 1, weight = 3.14, name = \"hoge\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 2, weight = 3.14, name = \"moge\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 4, weight = 3.14, name = \"hoge\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 3, weight = 3.14, name = \"oge\" OF test_table").unwrap();

		context.test_get_records = Some(vec![]);
		do_exec(&mut context, "GET ALL id,weight,name OF test_table ORDER BY id DESC LIMIT 2").unwrap();

		let s = test_get_records_to_string(&mut context);
		assert!(s == "5,3.14,hige
4,3.14,hoge
");
	}

	#[test]
	fn test_order_by_0() {
		let path = gen_test_table_path();
		let mut context = Context::new();
		remove_file(&path);
		do_exec(&mut context, "DROP DATABASE IF EXISTS test_db").unwrap();
		do_exec(&mut context, "CREATE DATABASE test_db").unwrap();
		do_exec(&mut context, "USE test_db").unwrap();
		do_exec(&mut context, "CREATE TABLE test_table (id: INT, weight: FLOAT, name: CHAR[128])").unwrap();
		assert!(path.exists());
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n");
		do_exec(&mut context, "ADD id = 5, weight = 3.14, name = \"hige\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 1, weight = 3.14, name = \"hoge\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 2, weight = 3.14, name = \"moge\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 4, weight = 3.14, name = \"hoge\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 3, weight = 3.14, name = \"oge\" OF test_table").unwrap();

		context.test_get_records = Some(vec![]);
		do_exec(&mut context, "GET ALL id,weight,name OF test_table ORDER BY id").unwrap();

		let s = test_get_records_to_string(&mut context);
		assert!(s == "1,3.14,hoge
2,3.14,moge
3,3.14,oge
4,3.14,hoge
5,3.14,hige
");
	}

	#[test]
	fn test_order_by_0c() {
		let path = gen_test_table_path();
		let mut context = Context::new();
		remove_file(&path);
		do_exec(&mut context, "DROP DATABASE IF EXISTS test_db").unwrap();
		do_exec(&mut context, "CREATE DATABASE test_db").unwrap();
		do_exec(&mut context, "USE test_db").unwrap();
		do_exec(&mut context, "CREATE TABLE test_table (id: INT, weight: FLOAT, name: CHAR[128])").unwrap();
		assert!(path.exists());
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n");
		do_exec(&mut context, "ADD id = 5, weight = 3.14, name = \"hige\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 1, weight = 3.14, name = \"hoge\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 2, weight = 3.14, name = \"moge\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 4, weight = 3.14, name = \"hoge\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 3, weight = 3.14, name = \"oge\" OF test_table").unwrap();

		context.test_get_records = Some(vec![]);
		do_exec(&mut context, "GET ALL id,weight,name OF test_table WHERE name == \"hoge\" ORDER BY id").unwrap();

		let s = test_get_records_to_string(&mut context);
		assert!(s == "1,3.14,hoge
4,3.14,hoge
");
	}

	#[test]
	fn test_order_by_0b() {
		let path = gen_test_table_path();
		let mut context = Context::new();
		remove_file(&path);
		do_exec(&mut context, "DROP DATABASE IF EXISTS test_db").unwrap();
		do_exec(&mut context, "CREATE DATABASE test_db").unwrap();
		do_exec(&mut context, "USE test_db").unwrap();
		do_exec(&mut context, "CREATE TABLE test_table (id: INT, weight: FLOAT, name: CHAR[128])").unwrap();
		assert!(path.exists());
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n");
		do_exec(&mut context, "ADD id = 5, weight = 3.14, name = \"hige\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 1, weight = 3.14, name = \"hoge\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 2, weight = 3.14, name = \"moge\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 4, weight = 3.14, name = \"hoge\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 3, weight = 3.14, name = \"oge\" OF test_table").unwrap();

		context.test_get_records = Some(vec![]);
		do_exec(&mut context, "GET ALL id,weight,name OF test_table ORDER BY id ASC").unwrap();

		let s = test_get_records_to_string(&mut context);
		assert!(s == "1,3.14,hoge
2,3.14,moge
3,3.14,oge
4,3.14,hoge
5,3.14,hige
");
	}

	#[test]
	fn test_order_by_0a() {
		let path = gen_test_table_path();
		let mut context = Context::new();
		remove_file(&path);
		do_exec(&mut context, "DROP DATABASE IF EXISTS test_db").unwrap();
		do_exec(&mut context, "CREATE DATABASE test_db").unwrap();
		do_exec(&mut context, "USE test_db").unwrap();
		do_exec(&mut context, "CREATE TABLE test_table (id: INT, weight: FLOAT, name: CHAR[128])").unwrap();
		assert!(path.exists());
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n");
		do_exec(&mut context, "ADD id = 5, weight = 3.14, name = \"hige\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 1, weight = 3.14, name = \"hoge\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 2, weight = 3.14, name = \"moge\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 4, weight = 3.14, name = \"hoge\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 3, weight = 3.14, name = \"oge\" OF test_table").unwrap();

		context.test_get_records = Some(vec![]);
		do_exec(&mut context, "GET ALL id,weight,name OF test_table ORDER BY id DESC").unwrap();

		let s = test_get_records_to_string(&mut context);
		assert!(s == "5,3.14,hige
4,3.14,hoge
3,3.14,oge
2,3.14,moge
1,3.14,hoge
");
	}

	#[test]
	fn test_order_by_1() {
		let path = gen_test_table_path();
		let mut context = Context::new();
		remove_file(&path);
		do_exec(&mut context, "DROP DATABASE IF EXISTS test_db").unwrap();
		do_exec(&mut context, "CREATE DATABASE test_db").unwrap();
		do_exec(&mut context, "USE test_db").unwrap();
		do_exec(&mut context, "CREATE TABLE test_table (id: INT, weight: FLOAT, name: CHAR[128])").unwrap();
		assert!(path.exists());
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n");
		do_exec(&mut context, "ADD id = 5, weight = 3.14, name = \"zzz\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 1, weight = 3.14, name = \"aaa\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 2, weight = 3.14, name = \"bbb\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 4, weight = 3.14, name = \"xxx\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 3, weight = 3.14, name = \"ccc\" OF test_table").unwrap();

		context.test_get_records = Some(vec![]);
		do_exec(&mut context, "GET ALL id,weight,name OF test_table ORDER BY name").unwrap();

		let s = test_get_records_to_string(&mut context);
		assert!(s == "1,3.14,aaa
2,3.14,bbb
3,3.14,ccc
4,3.14,xxx
5,3.14,zzz
"); 
	}

	#[test]
	fn test_count_func_0() {
		let path = gen_test_table_path();
		let mut context = Context::new();
		remove_file(&path);
		do_exec(&mut context, "DROP DATABASE IF EXISTS test_db").unwrap();
		do_exec(&mut context, "CREATE DATABASE test_db").unwrap();
		do_exec(&mut context, "USE test_db").unwrap();
		do_exec(&mut context, "CREATE TABLE test_table (id: INT, weight: FLOAT, name: CHAR[128])").unwrap();
		assert!(path.exists());
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n");
		do_exec(&mut context, "ADD id = 1, weight = 3.14, name = \"zzz\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 2, weight = 3.14, name = \"aaa\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 3, weight = 3.14, name = \"bbb\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 4, weight = 3.14, name = \"xxx\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 5, weight = 3.14, name = \"ccc\" OF test_table").unwrap();

		context.test_get_records = Some(vec![]);
		do_exec(&mut context, "GET ALL COUNT(id) OF test_table").unwrap();

		assert!(context.selected_csv_columns.len() == 1);
		assert!(context.selected_csv_columns[0] == "5");
	}

	#[test]
	fn test_count_func_1() {
		let path = gen_test_table_path();
		let mut context = Context::new();
		remove_file(&path);
		do_exec(&mut context, "DROP DATABASE IF EXISTS test_db").unwrap();
		do_exec(&mut context, "CREATE DATABASE test_db").unwrap();
		do_exec(&mut context, "USE test_db").unwrap();
		do_exec(&mut context, "CREATE TABLE test_table (id: INT, weight: FLOAT, name: CHAR[128])").unwrap();
		assert!(path.exists());
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n");
		do_exec(&mut context, "ADD id = 1, weight = 3.14, name = \"zzz\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 2, weight = 3.14, name = \"aaa\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 3, weight = 3.14, name = \"bbb\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 4, weight = 3.14, name = \"xxx\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 5, weight = 3.14, name = \"ccc\" OF test_table").unwrap();

		context.test_get_records = Some(vec![]);
		do_exec(&mut context, "GET ALL id,COUNT(id) OF test_table").unwrap();

		assert!(context.selected_csv_columns.len() == 2);
		assert!(context.selected_csv_columns[0] == "5");
		assert!(context.selected_csv_columns[1] == "5");
	}

	#[test]
	fn test_count_func_2() {
		let path = gen_test_table_path();
		let mut context = Context::new();
		remove_file(&path);
		do_exec(&mut context, "DROP DATABASE IF EXISTS test_db").unwrap();
		do_exec(&mut context, "CREATE DATABASE test_db").unwrap();
		do_exec(&mut context, "USE test_db").unwrap();
		do_exec(&mut context, "CREATE TABLE test_table (id: INT, weight: FLOAT, name: CHAR[128])").unwrap();
		assert!(path.exists());
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n");
		do_exec(&mut context, "ADD id = 1, weight = 3.14, name = \"zzz\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 2, weight = 3.14, name = \"aaa\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 3, weight = 3.14, name = \"bbb\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 4, weight = 3.14, name = \"xxx\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 5, weight = 3.14, name = \"ccc\" OF test_table").unwrap();

		context.test_get_records = Some(vec![]);
		do_exec(&mut context, "GET ALL COUNT(*) OF test_table").unwrap();

		assert!(context.selected_csv_columns.len() == 1);
		assert!(context.selected_csv_columns[0] == "5");
	}

	#[test]
	fn test_count_func_3() {
		let path = gen_test_table_path();
		let mut context = Context::new();
		remove_file(&path);
		do_exec(&mut context, "DROP DATABASE IF EXISTS test_db").unwrap();
		do_exec(&mut context, "CREATE DATABASE test_db").unwrap();
		do_exec(&mut context, "USE test_db").unwrap();
		do_exec(&mut context, "CREATE TABLE test_table (id: INT, weight: FLOAT, name: CHAR[128])").unwrap();
		assert!(path.exists());
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n");
		do_exec(&mut context, "ADD id = 1, weight = 3.14, name = \"zzz\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 2, weight = 3.14, name = \"aaa\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 3, weight = 3.14, name = \"bbb\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 4, weight = 3.14, name = \"xxx\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 5, weight = 3.14, name = \"ccc\" OF test_table").unwrap();

		context.test_get_records = Some(vec![]);
		do_exec(&mut context, "GET COUNT(id) OF test_table").unwrap();

		assert!(context.selected_csv_columns.len() == 1);
		assert!(context.selected_csv_columns[0] == "1");
	}

	#[test]
	fn test_avg_func_0() {
		let path = gen_test_table_path();
		let mut context = Context::new();
		remove_file(&path);
		do_exec(&mut context, "DROP DATABASE IF EXISTS test_db").unwrap();
		do_exec(&mut context, "CREATE DATABASE test_db").unwrap();
		do_exec(&mut context, "USE test_db").unwrap();
		do_exec(&mut context, "CREATE TABLE test_table (id: INT, weight: FLOAT, name: CHAR[128])").unwrap();
		assert!(path.exists());
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n");
		do_exec(&mut context, "ADD id = 1, weight = 3.14, name = \"zzz\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 2, weight = 3.14, name = \"aaa\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 3, weight = 3.14, name = \"bbb\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 4, weight = 3.14, name = \"xxx\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 5, weight = 3.14, name = \"ccc\" OF test_table").unwrap();

		context.test_get_records = Some(vec![]);
		do_exec(&mut context, "GET ALL AVG(id) OF test_table").unwrap();

		println!("[{}]", context.selected_csv_columns[0]);
		println!("avg_sum_value[{}]", context.avg_sum_value);
		println!("avg_counter[{}]", context.avg_counter);
		assert!(context.selected_csv_columns.len() == 1);
		assert!(context.selected_csv_columns[0] == "3");
	}

	#[test]
	fn test_avg_func_1() {
		let path = gen_test_table_path();
		let mut context = Context::new();
		remove_file(&path);
		do_exec(&mut context, "DROP DATABASE IF EXISTS test_db").unwrap();
		do_exec(&mut context, "CREATE DATABASE test_db").unwrap();
		do_exec(&mut context, "USE test_db").unwrap();
		do_exec(&mut context, "CREATE TABLE test_table (id: INT, weight: FLOAT, name: CHAR[128])").unwrap();
		assert!(path.exists());
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n");
		do_exec(&mut context, "ADD id = 1, weight = 3.14, name = \"aaa\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 2, weight = 3.14, name = \"aaa\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 3, weight = 3.14, name = \"aaa\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 4, weight = 3.14, name = \"bbb\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 5, weight = 3.14, name = \"bbb\" OF test_table").unwrap();

		context.test_get_records = Some(vec![]);
		do_exec(&mut context, "GET ALL AVG(id) OF test_table WHERE name == \"aaa\"").unwrap();

		println!("[{}]", context.selected_csv_columns[0]);
		println!("avg_sum_value[{}]", context.avg_sum_value);
		println!("avg_counter[{}]", context.avg_counter);
		assert!(context.selected_csv_columns.len() == 1);
		assert!(context.selected_csv_columns[0] == "2");
	}

	#[test]
	fn test_min_func_0() {
		let path = gen_test_table_path();
		let mut context = Context::new();
		remove_file(&path);
		do_exec(&mut context, "DROP DATABASE IF EXISTS test_db").unwrap();
		do_exec(&mut context, "CREATE DATABASE test_db").unwrap();
		do_exec(&mut context, "USE test_db").unwrap();
		do_exec(&mut context, "CREATE TABLE test_table (id: INT, weight: FLOAT, name: CHAR[128])").unwrap();
		assert!(path.exists());
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n");
		do_exec(&mut context, "ADD id = 1, weight = 3.14, name = \"aaa\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 2, weight = 3.14, name = \"aaa\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 3, weight = 3.14, name = \"aaa\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 4, weight = 3.14, name = \"bbb\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 5, weight = 3.14, name = \"bbb\" OF test_table").unwrap();

		context.test_get_records = Some(vec![]);
		do_exec(&mut context, "GET ALL MIN(id) OF test_table").unwrap();

		assert!(context.selected_csv_columns.len() == 1);
		assert!(context.selected_csv_columns[0] == "1");
	}

	#[test]
	fn test_min_func_1() {
		let path = gen_test_table_path();
		let mut context = Context::new();
		remove_file(&path);
		do_exec(&mut context, "DROP DATABASE IF EXISTS test_db").unwrap();
		do_exec(&mut context, "CREATE DATABASE test_db").unwrap();
		do_exec(&mut context, "USE test_db").unwrap();
		do_exec(&mut context, "CREATE TABLE test_table (id: INT, weight: FLOAT, name: CHAR[128])").unwrap();
		assert!(path.exists());
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n");
		do_exec(&mut context, "ADD id = 1, weight = 3.14, name = \"aaa\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 2, weight = 3.14, name = \"aaa\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 3, weight = 3.14, name = \"aaa\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 4, weight = 3.14, name = \"bbb\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 5, weight = 3.14, name = \"bbb\" OF test_table").unwrap();

		context.test_get_records = Some(vec![]);
		do_exec(&mut context, "GET ALL MIN(id) OF test_table WHERE name == \"aaa\"").unwrap();

		assert!(context.selected_csv_columns.len() == 1);
		assert!(context.selected_csv_columns[0] == "1");
	}

	#[test]
	fn test_max_func_0() {
		let path = gen_test_table_path();
		let mut context = Context::new();
		remove_file(&path);
		do_exec(&mut context, "DROP DATABASE IF EXISTS test_db").unwrap();
		do_exec(&mut context, "CREATE DATABASE test_db").unwrap();
		do_exec(&mut context, "USE test_db").unwrap();
		do_exec(&mut context, "CREATE TABLE test_table (id: INT, weight: FLOAT, name: CHAR[128])").unwrap();
		assert!(path.exists());
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n");
		do_exec(&mut context, "ADD id = 1, weight = 3.14, name = \"aaa\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 2, weight = 3.14, name = \"aaa\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 3, weight = 3.14, name = \"aaa\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 4, weight = 3.14, name = \"bbb\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 5, weight = 3.14, name = \"bbb\" OF test_table").unwrap();

		context.test_get_records = Some(vec![]);
		do_exec(&mut context, "GET ALL MAX(id) OF test_table").unwrap();

		assert!(context.selected_csv_columns.len() == 1);
		assert!(context.selected_csv_columns[0] == "5");
	}

	#[test]
	fn test_max_func_1() {
		let path = gen_test_table_path();
		let mut context = Context::new();
		remove_file(&path);
		do_exec(&mut context, "DROP DATABASE IF EXISTS test_db").unwrap();
		do_exec(&mut context, "CREATE DATABASE test_db").unwrap();
		do_exec(&mut context, "USE test_db").unwrap();
		do_exec(&mut context, "CREATE TABLE test_table (id: INT, weight: FLOAT, name: CHAR[128])").unwrap();
		assert!(path.exists());
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n");
		do_exec(&mut context, "ADD id = 1, weight = 3.14, name = \"aaa\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 2, weight = 3.14, name = \"aaa\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 3, weight = 3.14, name = \"aaa\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 4, weight = 3.14, name = \"bbb\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 5, weight = 3.14, name = \"bbb\" OF test_table").unwrap();

		context.test_get_records = Some(vec![]);
		do_exec(&mut context, "GET ALL MAX(id) OF test_table WHERE name == \"aaa\"").unwrap();

		assert!(context.selected_csv_columns.len() == 1);
		assert!(context.selected_csv_columns[0] == "3");
	}

	#[test]
	fn test_distinct_0() {
		let path = gen_test_table_path();
		let mut context = Context::new();
		remove_file(&path);
		do_exec(&mut context, "DROP DATABASE IF EXISTS test_db").unwrap();
		do_exec(&mut context, "CREATE DATABASE test_db").unwrap();
		do_exec(&mut context, "USE test_db").unwrap();
		do_exec(&mut context, "CREATE TABLE test_table (id: INT, weight: FLOAT, name: CHAR[128])").unwrap();
		assert!(path.exists());
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n");
		do_exec(&mut context, "ADD id = 1, weight = 3.14, name = \"aaa\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 2, weight = 3.14, name = \"aaa\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 3, weight = 3.14, name = \"aaa\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 4, weight = 3.14, name = \"bbb\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 5, weight = 3.14, name = \"bbb\" OF test_table").unwrap();

		context.test_get_records = Some(vec![]);
		do_exec(&mut context, "GET ALL DISTINCT name OF test_table").unwrap();

		let s = test_get_records_to_string(&mut context);
		println!("s[{}]", s);
		assert!(s == "1,3.14,aaa
4,3.14,bbb
");
	}

	#[test]
	fn test_distinct_1() {
		let path = gen_test_table_path();
		let mut context = Context::new();
		remove_file(&path);
		do_exec(&mut context, "DROP DATABASE IF EXISTS test_db").unwrap();
		do_exec(&mut context, "CREATE DATABASE test_db").unwrap();
		do_exec(&mut context, "USE test_db").unwrap();
		do_exec(&mut context, "CREATE TABLE test_table (id: INT, weight: FLOAT, name: CHAR[128])").unwrap();
		assert!(path.exists());
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n");
		do_exec(&mut context, "ADD id = 1, weight = 1.0, name = \"aaa\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 2, weight = 2.0, name = \"aaa\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 3, weight = 1.0, name = \"aaa\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 4, weight = 2.0, name = \"bbb\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 5, weight = 2.0, name = \"bbb\" OF test_table").unwrap();

		context.test_get_records = Some(vec![]);
		do_exec(&mut context, "GET ALL DISTINCT weight, name OF test_table").unwrap();

		let s = test_get_records_to_string(&mut context);
		assert!(s == "1,1,aaa
2,2,aaa
4,2,bbb
");
	}

	#[test]
	fn test_distinct_2() {
		let path = gen_test_table_path();
		let mut context = Context::new();
		remove_file(&path);
		do_exec(&mut context, "DROP DATABASE IF EXISTS test_db").unwrap();
		do_exec(&mut context, "CREATE DATABASE test_db").unwrap();
		do_exec(&mut context, "USE test_db").unwrap();
		do_exec(&mut context, "CREATE TABLE test_table (id: INT, weight: FLOAT, name: CHAR[128])").unwrap();
		assert!(path.exists());
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n");
		do_exec(&mut context, "ADD id = 1, weight = 1.0, name = \"aaa\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 2, weight = 2.0, name = \"aaa\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 3, weight = 1.0, name = \"aaa\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 4, weight = 2.0, name = \"bbb\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 5, weight = 2.0, name = \"bbb\" OF test_table").unwrap();

		context.test_get_records = Some(vec![]);
		do_exec(&mut context, "GET ALL DISTINCT * OF test_table").unwrap();

		let s = test_get_records_to_string(&mut context);
		assert!(s == "1,1,aaa
2,2,aaa
3,1,aaa
4,2,bbb
5,2,bbb
");
	}

	#[test]
	fn test_distinct_count_0() {
		let path = gen_test_table_path();
		let mut context = Context::new();
		remove_file(&path);
		do_exec(&mut context, "DROP DATABASE IF EXISTS test_db").unwrap();
		do_exec(&mut context, "CREATE DATABASE test_db").unwrap();
		do_exec(&mut context, "USE test_db").unwrap();
		do_exec(&mut context, "CREATE TABLE test_table (id: INT, weight: FLOAT, name: CHAR[128])").unwrap();
		assert!(path.exists());
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n");
		do_exec(&mut context, "ADD id = 1, weight = 1.0, name = \"aaa\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 2, weight = 2.0, name = \"aaa\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 3, weight = 1.0, name = \"aaa\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 4, weight = 2.0, name = \"bbb\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 5, weight = 2.0, name = \"bbb\" OF test_table").unwrap();

		context.test_get_records = Some(vec![]);
		do_exec(&mut context, "GET ALL DISTINCT COUNT(name) OF test_table").unwrap();

		assert!(context.selected_csv_columns.len() == 1);
		assert!(context.selected_csv_columns[0] == "5");
	}

	#[test]
	fn test_distinct_count_1() {
		let path = gen_test_table_path();
		let mut context = Context::new();
		remove_file(&path);
		do_exec(&mut context, "DROP DATABASE IF EXISTS test_db").unwrap();
		do_exec(&mut context, "CREATE DATABASE test_db").unwrap();
		do_exec(&mut context, "USE test_db").unwrap();
		do_exec(&mut context, "CREATE TABLE test_table (id: INT, weight: FLOAT, name: CHAR[128])").unwrap();
		assert!(path.exists());
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n");
		do_exec(&mut context, "ADD id = 1, weight = 1.0, name = \"aaa\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 2, weight = 2.0, name = \"aaa\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 3, weight = 1.0, name = \"aaa\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 4, weight = 2.0, name = \"bbb\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 5, weight = 2.0, name = \"bbb\" OF test_table").unwrap();

		context.test_get_records = Some(vec![]);
		do_exec(&mut context, "GET ALL DISTINCT COUNT(id) OF test_table").unwrap();

		assert!(context.selected_csv_columns.len() == 1);
		assert!(context.selected_csv_columns[0] == "5");
	}

	#[test]
	fn test_distinct_order_by() {
		let path = gen_test_table_path();
		let mut context = Context::new();
		remove_file(&path);
		do_exec(&mut context, "DROP DATABASE IF EXISTS test_db").unwrap();
		do_exec(&mut context, "CREATE DATABASE test_db").unwrap();
		do_exec(&mut context, "USE test_db").unwrap();
		do_exec(&mut context, "CREATE TABLE test_table (id: INT, weight: FLOAT, name: CHAR[128])").unwrap();
		assert!(path.exists());
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: INT,weight: FLOAT,name: CHAR[128]\n");
		do_exec(&mut context, "ADD id = 1, weight = 1.0, name = \"aaa\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 2, weight = 2.0, name = \"aaa\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 3, weight = 1.0, name = \"aaa\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 4, weight = 2.0, name = \"bbb\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 5, weight = 2.0, name = \"bbb\" OF test_table").unwrap();

		context.test_get_records = Some(vec![]);
		do_exec(&mut context, "GET ALL DISTINCT name OF test_table ORDER BY name DESC").unwrap();

		let s = test_get_records_to_string(&mut context);
		println!("s[{}]", s);
		assert!(s == "4,2,bbb
1,1,aaa
");
	}

	#[test]
	fn test_inner_join_0() {
		let mut context = Context::new();
		do_exec(&mut context, "DROP DATABASE IF EXISTS test_db").unwrap();
		do_exec(&mut context, "CREATE DATABASE test_db").unwrap();
		do_exec(&mut context, "USE test_db").unwrap();
		do_exec(&mut context, "DROP TABLE IF EXISTS users").unwrap();
		do_exec(&mut context, "DROP TABLE IF EXISTS products").unwrap();
		do_exec(&mut context, "CREATE TABLE users (id: INT, weight: FLOAT, name: CHAR[128])").unwrap();
		do_exec(&mut context, "CREATE TABLE products (id: INT, user_id: INT, name: CHAR[128])").unwrap();
		do_exec(&mut context, "ADD id = 1, name = \"aaa\" OF users").unwrap();
		do_exec(&mut context, "ADD id = 2, name = \"bbb\" OF users").unwrap();
		do_exec(&mut context, "ADD id = 3, name = \"ccc\" OF users").unwrap();
		do_exec(&mut context, "ADD id = 4, name = \"ddd\" OF users").unwrap();
		do_exec(&mut context, "ADD id = 5, name = \"ddd\" OF users").unwrap();
		do_exec(&mut context, "ADD id = 1, user_id = 1, name = \"aaa product 1\" OF products").unwrap();
		do_exec(&mut context, "ADD id = 2, user_id = 1, name = \"aaa product 2\" OF products").unwrap();
		do_exec(&mut context, "ADD id = 3, user_id = 2, name = \"bbb product 1\" OF products").unwrap();
		do_exec(&mut context, "ADD id = 4, user_id = 2, name = \"bbb product 2\" OF products").unwrap();
		do_exec(&mut context, "ADD id = 5, user_id = 3, name = \"ccc product 1\" OF products").unwrap();

		context.test_selected_records = Some(vec![]);
		do_exec(&mut context, "GET ALL users.id, products.id OF users INNER JOIN products ON users.id == products.user_id").unwrap();

		let s = test_selected_records_to_string(&mut context);
		println!("s[{}]", s);
		assert!(s == "1,1
1,2
2,3
2,4
3,5
");
	}

	#[test]
	fn test_inner_join_multi() {
		let mut context = Context::new();
		do_exec(&mut context, "DROP DATABASE IF EXISTS test_db").unwrap();
		do_exec(&mut context, "CREATE DATABASE test_db").unwrap();
		do_exec(&mut context, "USE test_db").unwrap();
		do_exec(&mut context, "DROP TABLE IF EXISTS users").unwrap();
		do_exec(&mut context, "DROP TABLE IF EXISTS products").unwrap();
		do_exec(&mut context, "CREATE TABLE users (id: INT, weight: FLOAT, name: CHAR[128])").unwrap();
		do_exec(&mut context, "CREATE TABLE products (id: INT, user_id: INT, name: CHAR[128])").unwrap();
		do_exec(&mut context, "CREATE TABLE countries (id: INT, user_id: INT, name: CHAR[128])").unwrap();
		do_exec(&mut context, "ADD id = 1, name = \"aaa\" OF users").unwrap();
		do_exec(&mut context, "ADD id = 2, name = \"bbb\" OF users").unwrap();
		do_exec(&mut context, "ADD id = 3, name = \"ccc\" OF users").unwrap();
		do_exec(&mut context, "ADD id = 4, name = \"ddd\" OF users").unwrap();
		do_exec(&mut context, "ADD id = 5, name = \"ddd\" OF users").unwrap();

		do_exec(&mut context, "ADD id = 1, user_id = 1, name = \"aaa product 1\" OF products").unwrap();
		do_exec(&mut context, "ADD id = 2, user_id = 1, name = \"aaa product 2\" OF products").unwrap();
		do_exec(&mut context, "ADD id = 3, user_id = 2, name = \"bbb product 1\" OF products").unwrap();
		do_exec(&mut context, "ADD id = 4, user_id = 2, name = \"bbb product 2\" OF products").unwrap();
		do_exec(&mut context, "ADD id = 5, user_id = 3, name = \"ccc product 1\" OF products").unwrap();

		do_exec(&mut context, "ADD id = 1, user_id = 1, name = \"japan\" OF countries").unwrap();
		do_exec(&mut context, "ADD id = 2, user_id = 2, name = \"usa\" OF countries").unwrap();
		do_exec(&mut context, "ADD id = 3, user_id = 3, name = \"korean\" OF countries").unwrap();

/*
+----+------+----+---------+---------------+----+---------+--------+
| id | name | id | user_id | name          | id | user_id | name   |
+----+------+----+---------+---------------+----+---------+--------+
|  1 | aaa  |  1 |       1 | aaa product 1 |  1 |       1 | japan  |
|  1 | aaa  |  2 |       1 | aaa product 2 |  1 |       1 | japan  |
|  2 | bbb  |  3 |       2 | bbb product 1 |  2 |       2 | usa    |
|  2 | bbb  |  4 |       2 | bbb product 2 |  2 |       2 | usa    |
|  3 | ccc  |  5 |       3 | ccc product 1 |  3 |       3 | korean |
+----+------+----+---------+---------------+----+---------+--------+
 */
		context.test_selected_records = Some(vec![]);
		do_exec(&mut context, "GET ALL users.id, products.id, countries.id OF users INNER JOIN products ON users.id == products.user_id INNER JOIN countries ON users.id == countries.user_id").unwrap();

		let s = test_selected_records_to_string(&mut context);
		println!("s[{}]", s);
		assert!(s == "1,1,1
1,2,1
2,3,2
2,4,2
3,5,3
");
	}
/*
	#[test]
	fn test_inner_join_star() {
		let mut context = Context::new();
		do_exec(&mut context, "DROP DATABASE IF EXISTS test_db").unwrap();
		do_exec(&mut context, "CREATE DATABASE test_db").unwrap();
		do_exec(&mut context, "USE test_db").unwrap();
		do_exec(&mut context, "DROP TABLE IF EXISTS users").unwrap();
		do_exec(&mut context, "DROP TABLE IF EXISTS products").unwrap();
		do_exec(&mut context, "CREATE TABLE users (id: INT, weight: FLOAT, name: CHAR[128])").unwrap();
		do_exec(&mut context, "CREATE TABLE products (id: INT, user_id: INT, name: CHAR[128])").unwrap();
		do_exec(&mut context, "ADD id = 1, name = \"aaa\" OF users").unwrap();
		do_exec(&mut context, "ADD id = 2, name = \"bbb\" OF users").unwrap();
		do_exec(&mut context, "ADD id = 3, name = \"ccc\" OF users").unwrap();
		do_exec(&mut context, "ADD id = 4, name = \"ddd\" OF users").unwrap();
		do_exec(&mut context, "ADD id = 5, name = \"ddd\" OF users").unwrap();
		do_exec(&mut context, "ADD id = 1, user_id = 1, name = \"aaa product 1\" OF products").unwrap();
		do_exec(&mut context, "ADD id = 2, user_id = 1, name = \"aaa product 2\" OF products").unwrap();
		do_exec(&mut context, "ADD id = 3, user_id = 2, name = \"bbb product 1\" OF products").unwrap();
		do_exec(&mut context, "ADD id = 4, user_id = 2, name = \"bbb product 2\" OF products").unwrap();
		do_exec(&mut context, "ADD id = 5, user_id = 3, name = \"ccc product 1\" OF products").unwrap();

		context.test_selected_records = Some(vec![]);
		do_exec(&mut context, "GET ALL * OF users INNER JOIN products ON users.id == products.user_id").unwrap();

		let s = test_selected_records_to_string(&mut context);
		println!("s[{}]", s);
		assert!(s == "1,aaa,1,1,aaa product 1
1,aaa,2,1,aaa product 2
2,bbb,3,2,bbb product 1
2,bbb,4,2,bbb product 2
3,ccc,5,3,ccc product 1
");
*/
/*		assert!(s == "1,aaa,1,1,aaa product 1
1,aaa,2,1,aaa product 2
2,bbb,3,2,bbb product 1
2,bbb,4,2,bbb product 2
3,ccc,5,3,ccc product 1
");
*/
}
