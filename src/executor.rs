use crate::error::{Error, make_error, err_exec, err_parse, err_planning};
use crate::parser;
use crate::planner;
use crate::tokenizer::{TokenKind};
use crate::context::{Context};
use crate::objects::{Object, ObjectKind, HeaderType};
use std::path::{Path, PathBuf};
use std::fs;
use std::fs::{OpenOptions};
use std::io::{Write};
use csv::{Reader, Writer, StringRecord};
use regex::Regex;

pub fn exec(context: &mut Context, node: &planner::PlansNode) -> Result<(), Error> {
	for plan in node.plans.iter() {
		exec_plan(context, &plan)?
	}
	Ok(())
}

pub fn exec_plan(context: &mut Context, node: &planner::PlanNode) -> Result<(), Error> {
	if let Some(use_db) = &node.use_db {
		exec_use_db(context, &use_db)?;
	} else if let Some(desc_table) = &node.desc_table {
		exec_desc_table(context, &desc_table)?;
	} else if let Some(project) = &node.project {
		while exec_project(context, &project)? {
		}
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
	} else if let Some(csv_file_rewrite) = &node.csv_file_rewrite {
		exec_csv_file_rewrite(context, &csv_file_rewrite)?;
	}

	Ok(())
}

fn exec_desc_table(context: &mut Context, node: &planner::DescTableNode) -> Result<(), Error> {
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

fn csv_header_to_idents(headers: &StringRecord) -> Vec<String> {
	let mut header_idents: Vec<String> = vec![];
	for header in headers.iter() {
		if let Some((left, _right)) = header.split_once(":") {
			header_idents.push(left.trim().to_string());
		}
	}
	header_idents
}

pub fn find_header_position(headers: &StringRecord, col_name: &str) -> Result<Option<usize>, Error> {
	let header_idents = csv_header_to_idents(headers);

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

fn rewrite_append_record_by_vars(context: &mut Context, node: &planner::CsvFileAppendNode, headers: &StringRecord, row: &mut Vec<String>) -> Result<(), Error> {
	if let Some(expr_list) = &node.expr_list {
		let objs = exec_expr_list(context, &expr_list)?;
		for obj in objs.iter() {
			let key = obj.to_string();
			if let Some(o) = context.vars.get(key.as_str()) {
				if let Some(index) = find_header_position(&headers, key.as_str())? {
					row[index] = o.to_string();
				} else {
					return err_exec!("invalid column: {}", key);
				}
			} else {
				return err_exec!("failed to get value of vars");
			}
		}
	} else {
		return err_exec!("invalid state: csv file append");
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
		if !typ.is_i64 {
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

			if stype.contains("i64") {
				typ.is_i64 = true;
			} else if stype.contains("f64") {
				typ.is_f64 = true;
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
				    	typ.default_value = Some(Object::from_i64(v.as_str().parse::<i64>().unwrap()));
				    } else if let Some(v) = cap.name("float") {
				    	typ.default_value = Some(Object::from_f64(v.as_str().parse::<f64>().unwrap()));
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

pub fn exec_csv_file_append(context: &mut Context, node: &planner::CsvFileAppendNode) -> Result<(), Error> {
	let path = context.gen_table_file_path(&node.table_name)?;
    let headers = read_table_headers(context, &node.table_name)?;
	let mut row: Vec<String> = gen_default_record(&headers)?;
    let mut writer = open_append_writer(&path)?;

    rewrite_append_record_by_vars(context, node, &headers, &mut row)?;
    set_auto_increment_ids(context, &node.table_name, &headers, &mut row)?;
    check_invalid_append_record(&row, &headers)?;

	match writer.write_record(&row) {
		Ok(_) => {},
		Err(e) => return err_exec!("failed to write CSV row {}", e),
	}

	Ok(())
}

pub fn exec_use_db(context: &mut Context, node: &planner::UseDatabaseNode) -> Result<(), Error> {
	context.using_db_name = node.db_name.clone();
	let path = context.gen_db_dir_path(&context.using_db_name)?;
	if !path.exists() {
		return err_exec!("{} does not exists database", context.using_db_name);
	}
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

fn gen_limit_value(context: &mut Context, node: &planner::ProjectNode) -> Result<Option<i64>, Error> {
	let mut limit_value: Option<i64> = None;

	if let Some(limit) = &node.limit {
		let o = exec_limit(context, limit)?;
		match o.kind {
			ObjectKind::I64 => {
				limit_value = Some(o.i64_value);
			}
			_ => return err_exec!("invalid limit expression"),
		}
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

	if let Some(or_expr) = &node.left_or_expr {
		lhs = exec_or_expr(context, or_expr)?;
	} else {
		return err_exec!("impossible");
	}

	if node.right_or_expr.is_none() {
		return Ok(lhs);
	}

	if let Some(or_expr) = &node.right_or_expr {
		rhs = exec_or_expr(context, or_expr)?;
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

pub fn or_objects(context: &mut Context, a: &Object, b: &Object) -> Result<Object, Error> {
	match a.kind {
		ObjectKind::Bool => {
			match b.kind {
				ObjectKind::Bool => {
					let n = a.bool_value || b.bool_value;
					Ok(Object::from_bool(n))
				}
				ObjectKind::Ident => {
					let bo = refer_ident(context, &b.ident)?;
					Ok(or_objects(context, a, &bo)?)
				}
				_ => return err_exec!("can't compare or"),
			}
		}
		ObjectKind::Ident => {
			match b.kind {
				ObjectKind::Bool => {
					let ao = refer_ident(context, &a.ident)?;
					Ok(or_objects(context, &ao, b)?)
				}
				ObjectKind::Ident => {
					let ao = refer_ident(context, &a.ident)?;
					let bo = refer_ident(context, &b.ident)?;
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
					let bo = refer_ident(context, &b.ident)?;
					Ok(or_objects(context, a, &bo)?)
				}
				_ => return err_exec!("can't compare or"),
			}
		}
		ObjectKind::Ident => {
			match b.kind {
				ObjectKind::Bool => {
					let ao = refer_ident(context, &a.ident)?;
					Ok(or_objects(context, &ao, b)?)
				}
				ObjectKind::Ident => {
					let ao = refer_ident(context, &a.ident)?;
					let bo = refer_ident(context, &b.ident)?;
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
		a = exec_operand(context, &*operand)?;	
	} else {
		return err_exec!("impossible");
	}

	c = a.clone();

	for i in (1..node.nodes.len()).step_by(2) {
		let op = &node.nodes[i];
		let rhs = &node.nodes[i+1];

		if let parser::CompareExprItemNode::Right(operand) = rhs {
			b = exec_operand(context, &*operand)?;
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

pub fn parse_column_by_head(head: &str, col: &str) -> Result<Object, Error> {
	let head = head.to_lowercase();

	if head.contains("i64") {
		let n = match col.parse::<i64>() {
			Ok(v) => v,
			Err(e) => return err_exec!("failed to parse column as i64. {}", e),
		};
		return Ok(Object::from_i64(n));
	} else if head.contains("f64") {
		let n = match col.parse::<f64>() {
			Ok(v) => v,
			Err(e) => return err_exec!("failed to parse column as f64. {}", e),
		};
		return Ok(Object::from_f64(n));
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

pub fn refer_ident(context: &mut Context, ident: &String) -> Result<Object, Error> {
	if let Some(index) = context.csv_header_idents.iter().position(|s| *s == *ident) {
		let head = &context.csv_header[index];
		let col = &context.scan_record[index];
		let o = parse_column_by_head(head, col)?;
		Ok(o)
	} else {
		err_exec!("not found ident in CSV header")
	}
}


pub fn compare_objects(context: &mut Context, lhs: &Object, op: &parser::CompareOpNode, rhs: &Object) -> Result<Object, Error> {
	match op {
		parser::CompareOpNode::Lt => {
			match lhs.kind {
				ObjectKind::I64 => {
					match rhs.kind {
						ObjectKind::I64 => {
							let b = lhs.i64_value < rhs.i64_value;
							Ok(Object::from_bool(b))
						},
						ObjectKind::F64 => {
							let l = lhs.i64_value as f64;
							let b = l < rhs.f64_value;
							Ok(Object::from_bool(b))
						},
						ObjectKind::Ident => {
							let ro = refer_ident(context, &rhs.ident)?;
							Ok(compare_objects(context, lhs, op, &ro)?)
						}
						_ => err_exec!("can't compare i64 and other: a < b"),
					}
				},
				ObjectKind::F64 => {
					match rhs.kind {
						ObjectKind::I64 => {
							let b = lhs.f64_value < rhs.i64_value as f64;
							Ok(Object::from_bool(b))
						},
						ObjectKind::F64 => {
							let b = lhs.f64_value < rhs.f64_value;
							Ok(Object::from_bool(b))
						},
						ObjectKind::Ident => {
							let ro = refer_ident(context, &rhs.ident)?;
							Ok(compare_objects(context, lhs, op, &ro)?)
						}
						_ => err_exec!("can't compare f64 and other: a < b"),
					}
				}
				ObjectKind::Ident => {
					match rhs.kind {
						ObjectKind::I64 |
						ObjectKind::F64 |
						ObjectKind::String => {
							let lo = refer_ident(context, &lhs.ident)?;
							Ok(compare_objects(context, &lo, op, rhs)?)
						},
						ObjectKind::Ident => {
							let lo = refer_ident(context, &lhs.ident)?;
							let ro = refer_ident(context, &rhs.ident)?;
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
				ObjectKind::I64 => {
					match rhs.kind {
						ObjectKind::I64 => {
							let b = lhs.i64_value <= rhs.i64_value;
							Ok(Object::from_bool(b))
						},
						ObjectKind::F64 => {
							let b = lhs.i64_value as f64 <= rhs.f64_value;
							Ok(Object::from_bool(b))
						},
						ObjectKind::Ident => {
							let ro = refer_ident(context, &rhs.ident)?;
							Ok(compare_objects(context, lhs, op, &ro)?)
						}
						_ => err_exec!("can't compare i64 and other: a <= b"),
					}
				},
				ObjectKind::F64 => {
					match rhs.kind {
						ObjectKind::I64 => {
							let b = lhs.f64_value <= rhs.i64_value as f64;
							Ok(Object::from_bool(b))
						},
						ObjectKind::F64 => {
							let b = lhs.f64_value <= rhs.f64_value;
							Ok(Object::from_bool(b))
						},
						ObjectKind::Ident => {
							let ro = refer_ident(context, &rhs.ident)?;
							Ok(compare_objects(context, lhs, op, &ro)?)
						}
						_ => err_exec!("can't compare f64 and other: a <= b"),
					}
				}
				ObjectKind::Ident => {
					match rhs.kind {
						ObjectKind::I64 |
						ObjectKind::F64 |
						ObjectKind::String => {
							let lo = refer_ident(context, &lhs.ident)?;
							Ok(compare_objects(context, &lo, op, rhs)?)
						},
						ObjectKind::Ident => {
							let lo = refer_ident(context, &lhs.ident)?;
							let ro = refer_ident(context, &rhs.ident)?;
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
				ObjectKind::I64 => {
					match rhs.kind {
						ObjectKind::I64 => {
							let b = lhs.i64_value > rhs.i64_value;
							Ok(Object::from_bool(b))
						},
						ObjectKind::F64 => {
							let b = lhs.i64_value as f64 > rhs.f64_value;
							Ok(Object::from_bool(b))
						},
						ObjectKind::Ident => {
							let ro = refer_ident(context, &rhs.ident)?;
							Ok(compare_objects(context, lhs, op, &ro)?)
						}
						_ => err_exec!("can't compare i64 and other: a > b"),
					}
				},
				ObjectKind::F64 => {
					match rhs.kind {
						ObjectKind::I64 => {
							let b = lhs.f64_value > rhs.i64_value as f64;
							Ok(Object::from_bool(b))
						},
						ObjectKind::F64 => {
							let b = lhs.f64_value > rhs.f64_value;
							Ok(Object::from_bool(b))
						},
						ObjectKind::Ident => {
							let ro = refer_ident(context, &rhs.ident)?;
							Ok(compare_objects(context, lhs, op, &ro)?)
						}
						_ => err_exec!("can't compare f64 and other: a > b"),
					}
				}
				ObjectKind::Ident => {
					match rhs.kind {
						ObjectKind::I64 |
						ObjectKind::F64 |
						ObjectKind::String => {
							let lo = refer_ident(context, &lhs.ident)?;
							Ok(compare_objects(context, &lo, op, rhs)?)
						},
						ObjectKind::Ident => {
							let lo = refer_ident(context, &lhs.ident)?;
							let ro = refer_ident(context, &rhs.ident)?;
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
				ObjectKind::I64 => {
					match rhs.kind {
						ObjectKind::I64 => {
							let b = lhs.i64_value >= rhs.i64_value;
							Ok(Object::from_bool(b))
						},
						ObjectKind::F64 => {
							let b = lhs.i64_value as f64 >= rhs.f64_value;
							Ok(Object::from_bool(b))
						},
						ObjectKind::Ident => {
							let ro = refer_ident(context, &rhs.ident)?;
							Ok(compare_objects(context, lhs, op, &ro)?)
						}
						_ => err_exec!("can't compare i64 and other: a >= b"),
					}
				},
				ObjectKind::F64 => {
					match rhs.kind {
						ObjectKind::I64 => {
							let b = lhs.f64_value >= rhs.i64_value as f64;
							Ok(Object::from_bool(b))
						},
						ObjectKind::F64 => {
							let b = lhs.f64_value >= rhs.f64_value;
							Ok(Object::from_bool(b))
						},
						ObjectKind::Ident => {
							let ro = refer_ident(context, &rhs.ident)?;
							Ok(compare_objects(context, lhs, op, &ro)?)
						}
						_ => err_exec!("can't compare f64 and other: a >= b"),
					}
				}
				ObjectKind::Ident => {
					match rhs.kind {
						ObjectKind::I64 |
						ObjectKind::F64 |
						ObjectKind::String => {
							let lo = refer_ident(context, &lhs.ident)?;
							Ok(compare_objects(context, &lo, op, rhs)?)
						},
						ObjectKind::Ident => {
							let lo = refer_ident(context, &lhs.ident)?;
							let ro = refer_ident(context, &rhs.ident)?;
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
				ObjectKind::I64 => {
					match rhs.kind {
						ObjectKind::I64 => {
							let b = lhs.i64_value == rhs.i64_value;
							Ok(Object::from_bool(b))
						},
						ObjectKind::F64 => {
							let b = lhs.i64_value as f64 == rhs.f64_value;
							Ok(Object::from_bool(b))
						},
						ObjectKind::Ident => {
							let ro = refer_ident(context, &rhs.ident)?;
							Ok(compare_objects(context, lhs, op, &ro)?)
						},
						_ => err_exec!("can't compare i64 and other: a == b"),
					}
				},
				ObjectKind::F64 => {
					match rhs.kind {
						ObjectKind::I64 => {
							let b = lhs.f64_value == rhs.i64_value as f64;
							Ok(Object::from_bool(b))
						},
						ObjectKind::F64 => {
							let b = lhs.f64_value == rhs.f64_value;
							Ok(Object::from_bool(b))
						},
						ObjectKind::Ident => {
							let ro = refer_ident(context, &rhs.ident)?;
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
							let ro = refer_ident(context, &rhs.ident)?;
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
							let ro = refer_ident(context, &rhs.ident)?;
							Ok(compare_objects(context, lhs, op, &ro)?)
						},
						_ => err_exec!("can't compare bool and other: a == b"),
					}
				}
				ObjectKind::Ident => {
					match rhs.kind {
						ObjectKind::I64 |
						ObjectKind::F64 |
						ObjectKind::Bool |
						ObjectKind::String => {
							let lo = refer_ident(context, &lhs.ident)?;
							Ok(compare_objects(context, &lo, op, rhs)?)									
						},
						ObjectKind::Ident => {
							let lo = refer_ident(context, &lhs.ident)?;
							let ro = refer_ident(context, &rhs.ident)?;
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
				ObjectKind::I64 => {
					match rhs.kind {
						ObjectKind::I64 => {
							let b = lhs.i64_value != rhs.i64_value;
							Ok(Object::from_bool(b))
						},
						ObjectKind::F64 => {
							let b = lhs.i64_value as f64 != rhs.f64_value;
							Ok(Object::from_bool(b))
						},
						ObjectKind::Ident => {
							let ro = refer_ident(context, &rhs.ident)?;
							Ok(compare_objects(context, lhs, op, &ro)?)
						},
						_ => err_exec!("can't compare i64 and other: a != b"),
					}
				},
				ObjectKind::F64 => {
					match rhs.kind {
						ObjectKind::I64 => {
							let b = lhs.f64_value != rhs.i64_value as f64;
							Ok(Object::from_bool(b))
						},
						ObjectKind::F64 => {
							let b = lhs.f64_value != rhs.f64_value;
							Ok(Object::from_bool(b))
						},
						ObjectKind::Ident => {
							let ro = refer_ident(context, &rhs.ident)?;
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
							let ro = refer_ident(context, &rhs.ident)?;
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
							let ro = refer_ident(context, &rhs.ident)?;
							Ok(compare_objects(context, lhs, op, &ro)?)
						},
						_ => err_exec!("can't compare bool and other: a != b"),
					}
				}
				ObjectKind::Ident => {
					match rhs.kind {
						ObjectKind::I64 |
						ObjectKind::F64 |
						ObjectKind::Bool |
						ObjectKind::String => {
							let lo = refer_ident(context, &lhs.ident)?;
							Ok(compare_objects(context, &lo, op, rhs)?)
						},
						ObjectKind::Ident => {
							let lo = refer_ident(context, &lhs.ident)?;
							let ro = refer_ident(context, &rhs.ident)?;
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

pub fn exec_operand(context: &mut Context, node: &parser::OperandNode) -> Result<Object, Error> {
	if let Some(i64_value) = &node.i64_value {
		return Ok(exec_i64_value(context, i64_value)?);
	} else if let Some(f64_value) = &node.f64_value {
		return Ok(exec_f64_value(context, f64_value)?);
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

pub fn exec_i64_value(_: &mut Context, node: &parser::I64ValueNode) -> Result<Object, Error> {
	let mut o = Object::new();
	o.kind = ObjectKind::I64;
	o.i64_value = node.value;
	Ok(o)
}

pub fn exec_f64_value(_: &mut Context, node: &parser::F64ValueNode) -> Result<Object, Error> {
	let mut o = Object::new();
	o.kind = ObjectKind::F64;
	o.f64_value = node.value;
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

pub fn select_get_columns(context: &mut Context, node: &planner::ProjectNode) -> Result<(), Error> {
	let mut indices: Vec<usize> = vec![];
	let row;

	context.selected_csv_columns.clear();

	if context.filtered {
		row = context.matched_csv_record.clone();
	} else {
		row = context.scan_record.clone();
	}
	if row.len() == 0 {
		return Ok(());
	}

	if node.get_stmt_objs.len() == 1 &&
	   node.get_stmt_objs[0].kind == ObjectKind::Star {
	   	for col in row.iter() {
	   		context.selected_csv_columns.push(col.to_string());
	   	}
	   	return Ok(());
	}

	for get_obj in node.get_stmt_objs.iter() {
		if let Some(index) = context.csv_header_idents.iter().position(|s| {
				return *s == *get_obj.to_string();
			}) {
			indices.push(index);
		} else {
			return err_exec!("invalid column: {}", get_obj.to_string());
		}
	}

	for index in indices {
		let col = &row[index];
		context.selected_csv_columns.push(col.to_string());
	}

	Ok(())
}

pub fn exec_project(context: &mut Context, project: &planner::ProjectNode) -> Result<bool, Error> {
	let limit_value = gen_limit_value(context, project)?;

	if let Some(filter) = &project.filter {
		let result = exec_filter(context, filter)?;
		if !result {
			return Ok(result);
		}
		select_get_columns(context, project)?;
		if context.is_cli {
			print_selected_columns(context)?;
		}
		if context.filtered {
			if context.matched {
				context.counter_selected += 1;
				if let Some(limit_value) = limit_value {
					if context.limit_counter >= limit_value &&
					   project.method == TokenKind::Get {
						return Ok(false);						
					}
				}
				context.limit_counter += 1;
				if let Some(test_get_records) = context.test_get_records.as_mut() {
					test_get_records.push(context.matched_csv_record.clone());
				}
			}
		} else {
			context.counter_selected += 1;
			if let Some(limit_value) = limit_value {
				if context.limit_counter >= limit_value &&
				   project.method == TokenKind::Get {
					return Ok(false);						
				}
			}
			context.limit_counter += 1;
			if let Some(test_get_records) = context.test_get_records.as_mut() {
				test_get_records.push(context.scan_record.clone());
			}
		}
		if !project.all && context.counter_selected >= 1 {
			context.table_csv_reader = None;
			return Ok(false);
		}
		return Ok(result);
	}
	return err_exec!("invalid state: project");
}

pub fn exec_filter(context: &mut Context, node: &planner::FilterNode) -> Result<bool, Error> {
	if let Some(where_clause) = &node.where_clause {
		context.filtered = true;
		if let Some(csv_file_scan) = &node.csv_file_scan {
			let result = exec_csv_file_scan(context, csv_file_scan)?;
			if !result {
				return Ok(result);
			}

			let o = exec_where_clause(context, where_clause)?;
			if o.bool_value {
				context.matched_csv_record = context.scan_record.clone();
				context.unmatched_csv_record.clear();
				print_record("matched", &context.matched_csv_record);
				context.matched = true;
			} else {
				context.matched_csv_record.clear();
				context.unmatched_csv_record = context.scan_record.clone();
				context.matched = false;
				print_record("unmatched", &context.unmatched_csv_record);
			}
			return Ok(result);
		}
	} else {
		context.filtered = false;
		context.matched = false;
		if let Some(csv_file_scan) = &node.csv_file_scan {
			let result = exec_csv_file_scan(context, csv_file_scan)?;
			return Ok(result);
		}
	}
	return err_exec!("invalid state: filter");
}

pub fn exec_csv_file_scan(context: &mut Context, node: &planner::CsvFileScanNode) -> Result<bool, Error> {
	if context.table_csv_reader.is_none() {
		let path = context.gen_table_file_path(&node.table_name)?;
		let reader = match Reader::from_path(&path) {
			Ok(v) => v,
			Err(e) => return err_exec!("failed to create csv reader. {}", e),
		};

		context.table_csv_reader = Some(reader);

		// read header
		if let Some(reader) = context.table_csv_reader.as_mut() {
			context.csv_header = match reader.headers() {
				Ok(v) => v.clone(),
				Err(e) => return err_exec!("failed to read header of CSV file. {}", e),
			};
		}

		parse_csv_header_idents(context)?;
	}	
	if let Some(reader) = context.table_csv_reader.as_mut() {
		match reader.read_record(&mut context.scan_record) {
			Ok(_) => {
				print_record("csv_file_scan", &context.scan_record);
				if context.scan_record.len() == 0 {
					context.table_csv_reader = None;
					return Ok(false);
				}
				return Ok(true);
			}
			Err(_) => {
				context.table_csv_reader = None;
				return Ok(false);
			}
		};
	} else {
		return err_exec!("reader is none");
	}
}

pub fn parse_csv_header_idents(context: &mut Context) -> Result<(), Error> {
	context.csv_header_idents.clear();

	for col in context.csv_header.iter() {
		if let Some((left, _right)) = col.split_once(':') {
			let val = left.trim().to_string();
			context.csv_header_idents.push(val);
		}
	}

	Ok(())
}

pub fn exec_dir_delete_all(context: &mut Context, node: &planner::DirDeleteAllNode) -> Result<(), Error> {
	let db_name = node.db_name.clone().unwrap();
	if db_name == context.using_db_name {
		return err_exec!("{} database is using now. can't delete", db_name);
	}
	let path = context.gen_db_dir_path(&db_name)?;
	if path.as_os_str().is_empty() {
		return err_exec!("invalid path in dir delete all");
	}
	if node.if_exists {
		if path.exists() {
			match fs::remove_dir_all(&path) {
				Ok(_) => {},
				Err(e) => return err_exec!("failed to remove directory. {}", e),
			}
		}
	} else {
		match fs::remove_dir_all(&path) {
			Ok(_) => {},
			Err(e) => return err_exec!("failed to remove directory. {}", e),
		}		
	}
	Ok(())
}

pub fn exec_dir_list(context: &mut Context, node: &planner::DirListNode) -> Result<(), Error> {
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

pub fn exec_database_create(context: &mut Context, node: &planner::DatabaseCreateNode) -> Result<(), Error> {
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

pub fn exec_row_delete(context: &mut Context, row_delete: &planner::RowDeleteNode, writer: &mut Writer<fs::File>) -> Result<(), Error> {
	if let Some(project) = &row_delete.project {
		let all = row_delete.all;
		let limit_value = gen_limit_value(context, project)?;
		let mut limited = false;

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
							writer.write_record(&context.unmatched_csv_record).unwrap();
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
					writer.write_record(&context.scan_record).unwrap();
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
								writer.write_record(&context.unmatched_csv_record).unwrap();
							}
						} else {
							writer.write_record(&context.scan_record).unwrap();
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
							writer.write_record(&context.scan_record).unwrap();
						}
					}
				} else {
					writer.write_record(&context.scan_record).unwrap();
				}
			}			
		}
	}

	Ok(())
}

pub fn replace_columns_by_objs(context: &mut Context, cols: &StringRecord, update_expr_list_objs: &Vec<Object>) -> Result<Vec<String>, Error> {
	let idents = &context.csv_header_idents;
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

pub fn exec_column_drop(context: &mut Context, node: &planner::ColumnDropNode, writer: &mut Writer<fs::File>, types: &Vec<HeaderType>, headers: &StringRecord) -> Result<(), Error> {
	if let Some(project) = &node.project {
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

		let seq = context.is_sequential;
		context.is_sequential = true;

		while exec_project(context, &project)? {
			let row = &context.scan_record;
			let row = drop_record_column(&row, drop_index)?;
			writer.write_record(&row).unwrap();
		}
		
		context.is_sequential = seq;
	}
	Ok(())
}

pub fn exec_column_add(context: &mut Context, node: &planner::ColumnAddNode, writer: &mut Writer<fs::File>, headers: &StringRecord) -> Result<(), Error> {
	if let Some(project) = &node.project {
		let def_row = gen_default_record(headers)?;
		assert!(def_row.len() > 0);
		
		let seq = context.is_sequential;
		context.is_sequential = true;

		while exec_project(context, &project)? {
			let mut row = context.scan_record.clone();
			row.push_field(def_row.to_vec().last().unwrap().as_str());
			writer.write_record(&row).unwrap();
		}
		
		context.is_sequential = seq;
	}

	Ok(())
}

pub fn exec_row_update(context: &mut Context, row_update: &planner::RowUpdateNode, writer: &mut Writer<fs::File>) -> Result<(), Error> {
	if let Some(project) = &row_update.project {
		if let Some(expr_list) = &row_update.expr_list {
			let update_expr_list_objs = exec_expr_list(context, expr_list)?;
			let limit_value = gen_limit_value(context, project)?;
			let mut limited = false;

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
								let cols = context.matched_csv_record.clone();
								let cols = replace_columns_by_objs(context, &cols, &update_expr_list_objs)?;
								writer.write_record(&cols).unwrap();
								if let Some(limit_value) = limit_value {
									if context.limit_counter >= limit_value {
										limited = true;
									}
								}
							} else {
								let cols = context.unmatched_csv_record.clone();
								writer.write_record(&cols).unwrap();
							}
						} else {
							let cols = context.scan_record.clone();
							let cols = replace_columns_by_objs(context, &cols, &update_expr_list_objs)?;
							writer.write_record(&cols).unwrap();
							if let Some(limit_value) = limit_value {
								if context.limit_counter >= limit_value {
									limited = true;
								}
							}
						}
					} else {
						let cols = context.scan_record.clone();
						writer.write_record(&cols).unwrap();
					}
				}
			} else {
				let mut writted = false;

				while exec_project(context, project)? {
					if !limited {
						if context.filtered {
							if context.matched && !writted {
								let cols = context.matched_csv_record.clone();
								let cols = replace_columns_by_objs(context, &cols, &update_expr_list_objs)?;
								writer.write_record(&cols).unwrap();
								writted = true;
								if let Some(limit_value) = limit_value {
									if context.limit_counter >= limit_value {
										limited = true;
									}
								}
							} else {
								let cols = context.scan_record.clone();
								writer.write_record(&cols).unwrap();
							}
						} else {
							if !writted {
								let cols = context.scan_record.clone();
								let cols = replace_columns_by_objs(context, &cols, &update_expr_list_objs)?;
								writer.write_record(&cols).unwrap();
								writted = true;
								if let Some(limit_value) = limit_value {
									if context.limit_counter >= limit_value {
										limited = true;
									}
								}
							} else {
								writer.write_record(&context.scan_record).unwrap();
							}
						}
					} else {
						writer.write_record(&context.scan_record).unwrap();
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

pub fn exec_csv_file_rewrite(context: &mut Context, node: &planner::CsvFileRewriteNode) -> Result<(), Error> {
	if let Some(table_name) = &node.table_name {
		let org_path = context.gen_table_file_path(&table_name)?;
		let tmp_path = context.gen_tmp_table_file_path(&table_name)?;
		let mut headers = read_table_headers(context, &table_name)?;
		let mut org_headers = StringRecord::new();
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
				org_headers = headers.clone();
				headers = drop_headers_column(&types, &mut headers, &ident)?;
			}
		}
		
		writer.write_record(&headers).unwrap();

		if let Some(row_delete) = &node.row_delete {
			exec_row_delete(context, row_delete, &mut writer)?;
		} else if let Some(row_update) = &node.row_update {
			exec_row_update(context, row_update, &mut writer)?;
		} else if let Some(column_add) = &node.column_add {
			exec_column_add(context, column_add, &mut writer, &headers)?;
		} else if let Some(column_drop) = &node.column_drop {
			exec_column_drop(context, column_drop, &mut writer, &types, &org_headers)?;
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

pub fn exec_csv_file_delete(context: &mut Context, node: &planner::CsvFileDeleteNode) -> Result<(), Error> {
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

pub fn exec_csv_file_create(context: &mut Context, node: &planner::CsvFileCreateNode) -> Result<(), Error> {
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
	use crate::planner::{PlansNode, planning};

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
		let node: PlansNode = match planning(&node) {
			Ok(v) => v,
			Err(e) => return err_planning!("{}", e),
		};
		return exec(context, &node);
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
		do_exec(&mut context, "CREATE TABLE test_table (id: I64, weight: F64)").unwrap();
		assert!(path.exists());
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: I64,weight: F64\n");
	}

	#[test]
	fn test_create_table_stmt_1() {
		let path = gen_test_table_path();
		remove_file(&path);
		let mut context = Context::new();
		do_exec(&mut context, "DROP DATABASE IF EXISTS test_db").unwrap();
		do_exec(&mut context, "CREATE DATABASE test_db").unwrap();
		do_exec(&mut context, "USE test_db").unwrap();
		do_exec(&mut context, "CREATE TABLE test_table (id: I64 PRIMARY_KEY AUTO_INCREMENT, weight: F64, name: CHAR[4])").unwrap();
		assert!(path.exists());
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: I64 PRIMARY_KEY AUTO_INCREMENT,weight: F64,name: CHAR[4]\n");
	}

	#[test]
	fn test_add_stmt_0() {
		let path = gen_test_table_path();
		remove_file(&path);
		let mut context = Context::new();
		do_exec(&mut context, "DROP DATABASE IF EXISTS test_db").unwrap();
		do_exec(&mut context, "CREATE DATABASE test_db").unwrap();
		do_exec(&mut context, "USE test_db").unwrap();
		do_exec(&mut context, "CREATE TABLE test_table (id: I64, weight: F64, name: CHAR[128])").unwrap();
		assert!(path.exists());
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: I64,weight: F64,name: CHAR[128]\n");
		do_exec(&mut context, "ADD id = 1, weight = 3.14, name = \"hige\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 2, weight = 3.14, name = \"hoge\" OF test_table").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: I64,weight: F64,name: CHAR[128]\n1,3.14,hige\n2,3.14,hoge\n");
	}

	#[test]
	fn test_add_stmt_1() {
		let path = gen_test_table_path();
		remove_file(&path);
		let mut context = Context::new();
		do_exec(&mut context, "DROP DATABASE IF EXISTS test_db").unwrap();
		do_exec(&mut context, "CREATE DATABASE test_db").unwrap();
		do_exec(&mut context, "USE test_db").unwrap();
		do_exec(&mut context, "CREATE TABLE test_table (id: I64, weight: F64, name: CHAR[128])").unwrap();
		assert!(path.exists());
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: I64,weight: F64,name: CHAR[128]\n");
		do_exec(&mut context, "ADD id = 1 OF test_table").unwrap();
		do_exec(&mut context, "ADD weight = 3.14, name = \"hoge\" OF test_table").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: I64,weight: F64,name: CHAR[128]\n1,0.0,\n0,3.14,hoge\n");
	}

	#[test]
	fn test_add_stmt_default_value() {
		let path = gen_test_table_path();
		remove_file(&path);
		let mut context = Context::new();
		do_exec(&mut context, "DROP DATABASE IF EXISTS test_db").unwrap();
		do_exec(&mut context, "CREATE DATABASE test_db").unwrap();
		do_exec(&mut context, "USE test_db").unwrap();
		do_exec(&mut context, "CREATE TABLE test_table (id: I64, weight: F64 DEFAULT 1.23, name: CHAR[128] DEFAULT \"def\")").unwrap();
		assert!(path.exists());
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: I64,weight: F64 DEFAULT 1.23,name: CHAR[128] DEFAULT \"def\"\n");
		do_exec(&mut context, "ADD id = 1 OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 2 OF test_table").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: I64,weight: F64 DEFAULT 1.23,name: CHAR[128] DEFAULT \"def\"\n1,1.23,def\n2,1.23,def\n");
	}

	#[test]
	fn test_add_stmt_check_char_size() {
		let path = gen_test_table_path();
		remove_file(&path);
		let mut context = Context::new();
		do_exec(&mut context, "DROP DATABASE IF EXISTS test_db").unwrap();
		do_exec(&mut context, "CREATE DATABASE test_db").unwrap();
		do_exec(&mut context, "USE test_db").unwrap();
		do_exec(&mut context, "CREATE TABLE test_table (id: I64, name: CHAR[4])").unwrap();
		assert!(path.exists());
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: I64,name: CHAR[4]\n");
		do_exec(&mut context, "ADD id = 1, name = \"hige\" OF test_table").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: I64,name: CHAR[4]\n1,hige\n");
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
		do_exec(&mut context, "CREATE TABLE test_table (id: I64 AUTO_INCREMENT, name: CHAR[4])").unwrap();
		assert!(path.exists());
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: I64 AUTO_INCREMENT,name: CHAR[4]\n");
		do_exec(&mut context, "ADD name = \"hige\" OF test_table").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: I64 AUTO_INCREMENT,name: CHAR[4]\n1,hige\n");
		do_exec(&mut context, "ADD name = \"hoge\" OF test_table").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: I64 AUTO_INCREMENT,name: CHAR[4]\n1,hige\n2,hoge\n");
		assert!(Path::new("test_env/test_db/id/test_table__id.txt").exists());
	}

	#[test]
	fn test_get_stmt_0a() {
		let path = gen_test_table_path();
		remove_file(&path);
		let mut context = Context::new();
		do_exec(&mut context, "DROP DATABASE IF EXISTS test_db").unwrap();
		do_exec(&mut context, "CREATE DATABASE test_db").unwrap();
		do_exec(&mut context, "USE test_db").unwrap();
		do_exec(&mut context, "CREATE TABLE test_table (id: I64, weight: F64, name: CHAR[128])").unwrap();
		assert!(path.exists());
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: I64,weight: F64,name: CHAR[128]\n");
		do_exec(&mut context, "ADD id = 1, weight = 3.14, name = \"hige\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 2, weight = 3.14, name = \"hoge\" OF test_table").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: I64,weight: F64,name: CHAR[128]\n1,3.14,hige\n2,3.14,hoge\n");
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
		do_exec(&mut context, "CREATE TABLE test_table (id: I64, weight: F64, name: CHAR[128])").unwrap();
		assert!(path.exists());
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: I64,weight: F64,name: CHAR[128]\n");
		do_exec(&mut context, "ADD id = 1, weight = 3.14, name = \"hige\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 2, weight = 3.14, name = \"hoge\" OF test_table").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: I64,weight: F64,name: CHAR[128]\n1,3.14,hige\n2,3.14,hoge\n");
		do_exec(&mut context, "GET ALL id, name OF test_table").unwrap();
		print_selected_columns(&mut context).unwrap();
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
		do_exec(&mut context, "CREATE TABLE test_table (id: I64, weight: F64, name: CHAR[128])").unwrap();
		assert!(path.exists());
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: I64,weight: F64,name: CHAR[128]\n");
		do_exec(&mut context, "ADD id = 1, weight = 3.14, name = \"hige\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 2, weight = 3.14, name = \"hoge\" OF test_table").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: I64,weight: F64,name: CHAR[128]\n1,3.14,hige\n2,3.14,hoge\n");
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
		do_exec(&mut context, "CREATE TABLE test_table (id: I64, weight: F64, name: CHAR[128])").unwrap();
		assert!(path.exists());
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: I64,weight: F64,name: CHAR[128]\n");
		do_exec(&mut context, "ADD id = 1, weight = 3.14, name = \"hige\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 2, weight = 3.14, name = \"hoge\" OF test_table").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: I64,weight: F64,name: CHAR[128]\n1,3.14,hige\n2,3.14,hoge\n");
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
		do_exec(&mut context, "CREATE TABLE test_table (id: I64, weight: F64, name: CHAR[128])").unwrap();
		assert!(path.exists());
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: I64,weight: F64,name: CHAR[128]\n");
		do_exec(&mut context, "ADD id = 1, weight = 3.14, name = \"hige\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 2, weight = 3.14, name = \"hoge\" OF test_table").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: I64,weight: F64,name: CHAR[128]\n1,3.14,hige\n2,3.14,hoge\n");
		do_exec(&mut context, "GET id, name OF test_table WHERE id == 1 AND name == \"hige\"").unwrap();
		assert!(context.selected_csv_columns.len() == 2);
		assert!(context.selected_csv_columns[0] == "1");
		assert!(context.selected_csv_columns[1] == "hige");
	}

	#[test]
	fn test_get_stmt_1() {
		let path = gen_test_table_path();
		remove_file(&path);
		let mut context = Context::new();
		do_exec(&mut context, "DROP DATABASE IF EXISTS test_db").unwrap();
		do_exec(&mut context, "CREATE DATABASE test_db").unwrap();
		do_exec(&mut context, "USE test_db").unwrap();
		do_exec(&mut context, "CREATE TABLE test_table (id: I64, weight: F64, name: CHAR[128])").unwrap();
		assert!(path.exists());
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: I64,weight: F64,name: CHAR[128]\n");
		do_exec(&mut context, "ADD id = 1, weight = 3.14, name = \"hige\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 2, weight = 3.14, name = \"hoge\" OF test_table").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: I64,weight: F64,name: CHAR[128]\n1,3.14,hige\n2,3.14,hoge\n");
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
		do_exec(&mut context, "CREATE TABLE test_table (id: I64, weight: F64, name: CHAR[128])").unwrap();
		assert!(path.exists());
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: I64,weight: F64,name: CHAR[128]\n");
		do_exec(&mut context, "ADD id = 1, weight = 3.14, name = \"hige\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 2, weight = 3.14, name = \"hoge\" OF test_table").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: I64,weight: F64,name: CHAR[128]\n1,3.14,hige\n2,3.14,hoge\n");

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

	#[test]
	fn test_get_stmt_or_0() {
		let path = gen_test_table_path();
		remove_file(&path);
		let mut context = Context::new();
		do_exec(&mut context, "DROP DATABASE IF EXISTS test_db").unwrap();
		do_exec(&mut context, "CREATE DATABASE test_db").unwrap();
		do_exec(&mut context, "USE test_db").unwrap();
		do_exec(&mut context, "CREATE TABLE test_table (id: I64, weight: F64, name: CHAR[128])").unwrap();
		assert!(path.exists());
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: I64,weight: F64,name: CHAR[128]\n");
		do_exec(&mut context, "ADD id = 1, weight = 3.14, name = \"hige\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 2, weight = 3.14, name = \"hoge\" OF test_table").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: I64,weight: F64,name: CHAR[128]\n1,3.14,hige\n2,3.14,hoge\n");
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
		do_exec(&mut context, "CREATE TABLE test_table (id: I64, weight: F64, name: CHAR[128])").unwrap();
		assert!(path.exists());
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: I64,weight: F64,name: CHAR[128]\n");
		do_exec(&mut context, "ADD id = 1, weight = 3.14, name = \"hige\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 2, weight = 3.14, name = \"hoge\" OF test_table").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: I64,weight: F64,name: CHAR[128]\n1,3.14,hige\n2,3.14,hoge\n");
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
			do_exec(&mut $context, "CREATE TABLE test_table (id: I64, weight: F64, name: CHAR[128])").unwrap();
			assert!(path.exists());
			let s = fs::read_to_string(&path).unwrap();
			assert!(s == "id: I64,weight: F64,name: CHAR[128]\n");
			do_exec(&mut $context, "ADD id = 1, weight = 3.14, name = \"hige\" OF test_table").unwrap();
			do_exec(&mut $context, "ADD id = 2, weight = 3.14, name = \"hoge\" OF test_table").unwrap();
			let s = fs::read_to_string(&path).unwrap();
			assert!(s == "id: I64,weight: F64,name: CHAR[128]\n1,3.14,hige\n2,3.14,hoge\n");
	    };
	}

	macro_rules! setup_records_2 {
	    ($context:ident) => {
    		let path = gen_test_table_path();
			remove_file(&path);
			do_exec(&mut $context, "DROP DATABASE IF EXISTS test_db").unwrap();
			do_exec(&mut $context, "CREATE DATABASE test_db").unwrap();
			do_exec(&mut $context, "USE test_db").unwrap();
			do_exec(&mut $context, "CREATE TABLE test_table (id: I64, weight: F64, name: CHAR[128])").unwrap();
			assert!(path.exists());
			let s = fs::read_to_string(&path).unwrap();
			assert!(s == "id: I64,weight: F64,name: CHAR[128]\n");
			do_exec(&mut $context, "ADD id = 1, weight = 3.14, name = \"hige\" OF test_table").unwrap();
			do_exec(&mut $context, "ADD id = 2, weight = 3.14, name = \"hoge\" OF test_table").unwrap();
			do_exec(&mut $context, "ADD id = 3, weight = 3.14, name = \"moge\" OF test_table").unwrap();
			do_exec(&mut $context, "ADD id = 4, weight = 3.14, name = \"huge\" OF test_table").unwrap();
			do_exec(&mut $context, "ADD id = 5, weight = 3.14, name = \"oge\" OF test_table").unwrap();
			let s = fs::read_to_string(&path).unwrap();
			assert!(s == "id: I64,weight: F64,name: CHAR[128]\n1,3.14,hige\n2,3.14,hoge\n3,3.14,moge\n4,3.14,huge\n5,3.14,oge\n");
	    };
	}

	macro_rules! setup_records_3 {
	    ($context:ident) => {
    		let path = gen_test_table_path();
			remove_file(&path);
			do_exec(&mut $context, "DROP DATABASE IF EXISTS test_db").unwrap();
			do_exec(&mut $context, "CREATE DATABASE test_db").unwrap();
			do_exec(&mut $context, "USE test_db").unwrap();
			do_exec(&mut $context, "CREATE TABLE test_table (id: I64 AUTO_INCREMENT, weight: F64, is_login: BOOL, name: CHAR[128])").unwrap();
			assert!(path.exists());
			let s = fs::read_to_string(&path).unwrap();
			assert!(s == "id: I64 AUTO_INCREMENT,weight: F64,is_login: BOOL,name: CHAR[128]\n");
			do_exec(&mut $context, "ADD weight = 60.2, is_login = true, name = \"hige\" OF test_table").unwrap();
			do_exec(&mut $context, "ADD weight = 60.2, is_login = false, name = \"hoge\" OF test_table").unwrap();
			let s = fs::read_to_string(&path).unwrap();
			assert!(s == "id: I64 AUTO_INCREMENT,weight: F64,is_login: BOOL,name: CHAR[128]\n1,60.2,true,hige\n2,60.2,false,hoge\n");
	    };
	}

	#[test]
	fn test_get_stmt_limit_0() {
		let path = gen_test_table_path();
		let mut context = Context::new();

		setup_records_2!(context);
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: I64,weight: F64,name: CHAR[128]\n1,3.14,hige\n2,3.14,hoge\n3,3.14,moge\n4,3.14,huge\n5,3.14,oge\n");

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
		assert!(s == "id: I64,weight: F64,name: CHAR[128]\n1,3.14,hige\n2,3.14,hoge\n3,3.14,moge\n4,3.14,huge\n5,3.14,oge\n");

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
		do_exec(&mut context, "CREATE TABLE test_table (id: I64, weight: F64, name: CHAR[128])").unwrap();
		assert!(path.exists());
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: I64,weight: F64,name: CHAR[128]\n");
		do_exec(&mut context, "ADD id = 1, weight = 3.14, name = \"hige\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 2, weight = 3.14, name = \"hoge\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 3, weight = 3.14, name = \"moge\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 4, weight = 3.14, name = \"hoge\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 5, weight = 3.14, name = \"oge\" OF test_table").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: I64,weight: F64,name: CHAR[128]\n1,3.14,hige\n2,3.14,hoge\n3,3.14,moge\n4,3.14,hoge\n5,3.14,oge\n");

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
		do_exec(&mut context, "CREATE TABLE test_table (id: I64, weight: F64, name: CHAR[128])").unwrap();
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
		do_exec(&mut context, "CREATE TABLE test_table (id: I64, weight: F64, name: CHAR[128])").unwrap();
		do_exec(&mut context, "ADD id = 1, weight = 3.14, name = \"hige\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 2, weight = 3.14, name = \"hoge\" OF test_table").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: I64,weight: F64,name: CHAR[128]\n1,3.14,hige\n2,3.14,hoge\n");
		do_exec(&mut context, "DEL ALL OF test_table").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: I64,weight: F64,name: CHAR[128]\n");
	}

	#[test]
	fn test_del_stmt_1() {
		let path = gen_test_table_path();
		remove_file(&path);
		let mut context = Context::new();
		do_exec(&mut context, "DROP DATABASE IF EXISTS test_db").unwrap();
		do_exec(&mut context, "CREATE DATABASE test_db").unwrap();
		do_exec(&mut context, "USE test_db").unwrap();
		do_exec(&mut context, "CREATE TABLE test_table (id: I64, weight: F64, name: CHAR[128])").unwrap();
		do_exec(&mut context, "ADD id = 1, weight = 3.14, name = \"hige\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 2, weight = 3.14, name = \"hoge\" OF test_table").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: I64,weight: F64,name: CHAR[128]\n1,3.14,hige\n2,3.14,hoge\n");
		do_exec(&mut context, "DEL ALL OF test_table WHERE id == 1").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: I64,weight: F64,name: CHAR[128]\n2,3.14,hoge\n");
	}

	#[test]
	fn test_del_stmt_2() {
		let path = gen_test_table_path();
		remove_file(&path);
		let mut context = Context::new();
		do_exec(&mut context, "DROP DATABASE IF EXISTS test_db").unwrap();
		do_exec(&mut context, "CREATE DATABASE test_db").unwrap();
		do_exec(&mut context, "USE test_db").unwrap();
		do_exec(&mut context, "CREATE TABLE test_table (id: I64, weight: F64, name: CHAR[128])").unwrap();
		do_exec(&mut context, "ADD id = 1, weight = 3.14, name = \"hige\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 2, weight = 3.14, name = \"hoge\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 3, weight = 3.14, name = \"moge\" OF test_table").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: I64,weight: F64,name: CHAR[128]\n1,3.14,hige\n2,3.14,hoge\n3,3.14,moge\n");
		do_exec(&mut context, "DEL ALL OF test_table WHERE id == 2").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: I64,weight: F64,name: CHAR[128]\n1,3.14,hige\n3,3.14,moge\n");
	}

	#[test]
	fn test_del_stmt_3() {
		let path = gen_test_table_path();
		let mut context = Context::new();

		setup_records_2!(context);
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: I64,weight: F64,name: CHAR[128]\n1,3.14,hige\n2,3.14,hoge\n3,3.14,moge\n4,3.14,huge\n5,3.14,oge\n");

		do_exec(&mut context, "DEL ALL OF test_table WHERE id == 1").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: I64,weight: F64,name: CHAR[128]\n2,3.14,hoge\n3,3.14,moge\n4,3.14,huge\n5,3.14,oge\n");

		do_exec(&mut context, "DEL ALL OF test_table WHERE name == \"oge\"").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: I64,weight: F64,name: CHAR[128]\n2,3.14,hoge\n3,3.14,moge\n4,3.14,huge\n");

		do_exec(&mut context, "DEL ALL OF test_table WHERE weight == 3.14").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: I64,weight: F64,name: CHAR[128]\n");
	}

	#[test]
	fn test_del_stmt_4() {
		let path = gen_test_table_path();
		let mut context = Context::new();

		setup_records_2!(context);
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: I64,weight: F64,name: CHAR[128]\n1,3.14,hige\n2,3.14,hoge\n3,3.14,moge\n4,3.14,huge\n5,3.14,oge\n");

		do_exec(&mut context, "DEL OF test_table").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: I64,weight: F64,name: CHAR[128]\n2,3.14,hoge\n3,3.14,moge\n4,3.14,huge\n5,3.14,oge\n");
	}

	#[test]
	fn test_del_stmt_5() {
		let path = gen_test_table_path();
		let mut context = Context::new();

		setup_records_2!(context);
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: I64,weight: F64,name: CHAR[128]\n1,3.14,hige\n2,3.14,hoge\n3,3.14,moge\n4,3.14,huge\n5,3.14,oge\n");

		do_exec(&mut context, "DEL OF test_table WHERE id == 2").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: I64,weight: F64,name: CHAR[128]\n1,3.14,hige\n3,3.14,moge\n4,3.14,huge\n5,3.14,oge\n");
	}

	#[test]
	fn test_del_stmt_limit_0() {
		let path = gen_test_table_path();
		let mut context = Context::new();

		setup_records_2!(context);
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: I64,weight: F64,name: CHAR[128]\n1,3.14,hige\n2,3.14,hoge\n3,3.14,moge\n4,3.14,huge\n5,3.14,oge\n");

		context.test_get_records = Some(vec![]);
		do_exec(&mut context, "DEL ALL OF test_table LIMIT 2").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: I64,weight: F64,name: CHAR[128]
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
		do_exec(&mut context, "CREATE TABLE test_table (id: I64, weight: F64, name: CHAR[128])").unwrap();
		assert!(path.exists());
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: I64,weight: F64,name: CHAR[128]\n");
		do_exec(&mut context, "ADD id = 1, weight = 3.14, name = \"hige\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 2, weight = 3.14, name = \"hoge\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 3, weight = 3.14, name = \"moge\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 4, weight = 3.14, name = \"hoge\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 5, weight = 3.14, name = \"oge\" OF test_table").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: I64,weight: F64,name: CHAR[128]
1,3.14,hige
2,3.14,hoge
3,3.14,moge
4,3.14,hoge
5,3.14,oge
");

		do_exec(&mut context, "DEL ALL OF test_table WHERE name == \"hoge\" LIMIT 2").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: I64,weight: F64,name: CHAR[128]
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
		do_exec(&mut context, "CREATE TABLE test_table (id: I64, weight: F64, name: CHAR[128])").unwrap();
		do_exec(&mut context, "ADD id = 1, weight = 3.14, name = \"hige\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 2, weight = 3.14, name = \"hoge\" OF test_table").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: I64,weight: F64,name: CHAR[128]\n1,3.14,hige\n2,3.14,hoge\n");
		do_exec(&mut context, "SET ALL id=10 OF test_table WHERE weight == 3.14").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: I64,weight: F64,name: CHAR[128]\n10,3.14,hige\n10,3.14,hoge\n");
	}

	#[test]
	fn test_set_stmt_0a() {
		let path = gen_test_table_path();
		remove_file(&path);
		let mut context = Context::new();
		do_exec(&mut context, "DROP DATABASE IF EXISTS test_db").unwrap();
		do_exec(&mut context, "CREATE DATABASE test_db").unwrap();
		do_exec(&mut context, "USE test_db").unwrap();
		do_exec(&mut context, "CREATE TABLE test_table (id: I64, weight: F64, name: CHAR[128])").unwrap();
		do_exec(&mut context, "ADD id = 1, weight = 3.14, name = \"hige\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 2, weight = 3.14, name = \"hoge\" OF test_table").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: I64,weight: F64,name: CHAR[128]\n1,3.14,hige\n2,3.14,hoge\n");
		do_exec(&mut context, "SET ALL id=10, name=\"HOGE\" OF test_table WHERE weight == 1234").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: I64,weight: F64,name: CHAR[128]\n1,3.14,hige\n2,3.14,hoge\n");
	}

	#[test]
	fn test_set_stmt_1() {
		let path = gen_test_table_path();
		let mut context = Context::new();
		setup_records!(context);
		do_exec(&mut context, "SET id=10, name=\"HOGE\" OF test_table WHERE weight == 3.14").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: I64,weight: F64,name: CHAR[128]\n10,3.14,HOGE\n2,3.14,hoge\n");
	}

	#[test]
	fn test_set_stmt_2() {
		let path = gen_test_table_path();
		let mut context = Context::new();
		setup_records!(context);
		do_exec(&mut context, "SET id=10, name=\"HOGE\" OF test_table WHERE name == \"hoge\"").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: I64,weight: F64,name: CHAR[128]\n1,3.14,hige\n10,3.14,HOGE\n");
	}

	#[test]
	fn test_set_stmt_3() {
		let path = gen_test_table_path();
		let mut context = Context::new();
		setup_records!(context);
		do_exec(&mut context, "SET ALL name=\"HOGE\" OF test_table WHERE weight == 3.14").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: I64,weight: F64,name: CHAR[128]\n1,3.14,HOGE\n2,3.14,HOGE\n");
	}

	#[test]
	fn test_set_stmt_4() {
		let path = gen_test_table_path();
		let mut context = Context::new();
		setup_records!(context);
		do_exec(&mut context, "SET ALL name=\"HOGE\" OF test_table").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: I64,weight: F64,name: CHAR[128]\n1,3.14,HOGE\n2,3.14,HOGE\n");
	}

	#[test]
	fn test_set_stmt_limit_0() {
		let path = gen_test_table_path();
		let mut context = Context::new();

		setup_records_2!(context);
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: I64,weight: F64,name: CHAR[128]\n1,3.14,hige\n2,3.14,hoge\n3,3.14,moge\n4,3.14,huge\n5,3.14,oge\n");

		context.test_get_records = Some(vec![]);
		do_exec(&mut context, "SET ALL weight = 1.23 OF test_table LIMIT 2").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: I64,weight: F64,name: CHAR[128]
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
		assert!(s == "id: I64,weight: F64,name: CHAR[128]\n1,3.14,hige\n2,3.14,hoge\n3,3.14,moge\n4,3.14,huge\n5,3.14,oge\n");

		context.test_get_records = Some(vec![]);
		do_exec(&mut context, "SET ALL weight = 1.23 OF test_table LIMIT 0").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: I64,weight: F64,name: CHAR[128]
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
		do_exec(&mut context, "CREATE TABLE test_table (id: I64, weight: F64, name: CHAR[128])").unwrap();
		assert!(path.exists());
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: I64,weight: F64,name: CHAR[128]\n");
		do_exec(&mut context, "ADD id = 1, weight = 3.14, name = \"hige\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 2, weight = 3.14, name = \"hoge\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 3, weight = 3.14, name = \"moge\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 4, weight = 3.14, name = \"hoge\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 5, weight = 3.14, name = \"oge\" OF test_table").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: I64,weight: F64,name: CHAR[128]\n1,3.14,hige\n2,3.14,hoge\n3,3.14,moge\n4,3.14,hoge\n5,3.14,oge\n");

		do_exec(&mut context, "SET ALL weight = 1.23 OF test_table WHERE name == \"hoge\" LIMIT 2").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: I64,weight: F64,name: CHAR[128]
1,3.14,hige
2,1.23,hoge
3,3.14,moge
4,1.23,hoge
5,3.14,oge
");
	}
	
	#[test]
	fn test_alter_add_column_0() {
		let path = gen_test_table_path();
		let mut context = Context::new();
		setup_records_2!(context);
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: I64,weight: F64,name: CHAR[128]\n1,3.14,hige\n2,3.14,hoge\n3,3.14,moge\n4,3.14,huge\n5,3.14,oge\n");
		do_exec(&mut context, "ALTER TABLE test_table ADD COLUMN uge I64").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: I64,weight: F64,name: CHAR[128],uge: I64\n1,3.14,hige,0\n2,3.14,hoge,0\n3,3.14,moge,0\n4,3.14,huge,0\n5,3.14,oge,0\n");
	}

	#[test]
	fn test_alter_add_column_1() {
		let path = gen_test_table_path();
		let mut context = Context::new();
		setup_records_2!(context);
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: I64,weight: F64,name: CHAR[128]\n1,3.14,hige\n2,3.14,hoge\n3,3.14,moge\n4,3.14,huge\n5,3.14,oge\n");
		do_exec(&mut context, "ALTER TABLE test_table ADD COLUMN uge I64 DEFAULT 100").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: I64,weight: F64,name: CHAR[128],uge: I64 DEFAULT 100\n1,3.14,hige,100\n2,3.14,hoge,100\n3,3.14,moge,100\n4,3.14,huge,100\n5,3.14,oge,100\n");
	}

	#[test]
	fn test_where_lt() {
		let path = gen_test_table_path();
		let mut context = Context::new();
		setup_records_2!(context);
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: I64,weight: F64,name: CHAR[128]\n1,3.14,hige\n2,3.14,hoge\n3,3.14,moge\n4,3.14,huge\n5,3.14,oge\n");
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
		assert!(s == "id: I64,weight: F64,name: CHAR[128]\n1,3.14,hige\n2,3.14,hoge\n3,3.14,moge\n4,3.14,huge\n5,3.14,oge\n");
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
		assert!(s == "id: I64,weight: F64,name: CHAR[128]\n1,3.14,hige\n2,3.14,hoge\n3,3.14,moge\n4,3.14,huge\n5,3.14,oge\n");
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
		assert!(s == "id: I64,weight: F64,name: CHAR[128]\n1,3.14,hige\n2,3.14,hoge\n3,3.14,moge\n4,3.14,huge\n5,3.14,oge\n");
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
		assert!(s == "id: I64 AUTO_INCREMENT,weight: F64,is_login: BOOL,name: CHAR[128]\n1,60.2,true,hige\n2,60.2,false,hoge\n");

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
		assert!(s == "id: I64 AUTO_INCREMENT,weight: F64,is_login: BOOL,name: CHAR[128]\n1,60.2,true,hige\n2,60.2,false,hoge\n");

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
		assert!(s == "id: I64 AUTO_INCREMENT,weight: F64,is_login: BOOL,name: CHAR[128]\n1,60.2,true,hige\n2,60.2,false,hoge\n");

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
		assert!(s == "id: I64 AUTO_INCREMENT,weight: F64,is_login: BOOL,name: CHAR[128]\n1,60.2,true,hige\n2,60.2,false,hoge\n");

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
		assert!(s == "id: I64,weight: F64,name: CHAR[128]\n1,3.14,hige\n2,3.14,hoge\n3,3.14,moge\n4,3.14,huge\n5,3.14,oge\n");

		do_exec(&mut context, "ALTER TABLE test_table DROP COLUMN id").unwrap();

		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "weight: F64,name: CHAR[128]\n3.14,hige\n3.14,hoge\n3.14,moge\n3.14,huge\n3.14,oge\n");
	}

	#[test]
	fn test_alter_drop_column_1() {
		let path = gen_test_table_path();
		let mut context = Context::new();

		setup_records_2!(context);
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: I64,weight: F64,name: CHAR[128]\n1,3.14,hige\n2,3.14,hoge\n3,3.14,moge\n4,3.14,huge\n5,3.14,oge\n");

		do_exec(&mut context, "ALTER TABLE test_table DROP COLUMN weight").unwrap();

		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: I64,name: CHAR[128]\n1,hige\n2,hoge\n3,moge\n4,huge\n5,oge\n");
	}

	#[test]
	fn test_alter_drop_column_2() {
		let path = gen_test_table_path();
		let mut context = Context::new();

		setup_records_2!(context);
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: I64,weight: F64,name: CHAR[128]\n1,3.14,hige\n2,3.14,hoge\n3,3.14,moge\n4,3.14,huge\n5,3.14,oge\n");

		do_exec(&mut context, "ALTER TABLE test_table DROP COLUMN name").unwrap();

		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: I64,weight: F64\n1,3.14\n2,3.14\n3,3.14\n4,3.14\n5,3.14\n");
	}

	#[test]
	fn test_alter_drop_column_3() {
		let path = gen_test_table_path();
		let mut context = Context::new();

		setup_records_2!(context);
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: I64,weight: F64,name: CHAR[128]\n1,3.14,hige\n2,3.14,hoge\n3,3.14,moge\n4,3.14,huge\n5,3.14,oge\n");

		match do_exec(&mut context, "ALTER TABLE test_table DROP COLUMN nothing") {
			Ok(_) => panic!("failed"),
			Err(_) => {}
		}
	}

}
