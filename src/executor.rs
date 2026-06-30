use crate::error::{Error, make_error, err_exec};
use crate::parser;
use crate::planner;
use crate::context::{Context};
use crate::objects::{Object, ObjectKind};
use std::path::{Path, PathBuf};
use std::fs;
use std::fs::{OpenOptions};
use std::io::{Write};
use csv::{Reader, Writer, StringRecord};

pub fn exec(context: &mut Context, node: &planner::PlansNode) -> Result<(), Error> {
	for plan in node.plans.iter() {
		exec_plan(context, &plan)?
	}
	Ok(())
}

pub fn exec_plan(context: &mut Context, node: &planner::PlanNode) -> Result<(), Error> {
	if let Some(use_db) = &node.use_db {
		exec_use_db(context, &use_db)?;
	} else if let Some(project) = &node.project {
		exec_project(context, &project)?;
	} else if let Some(dir_create) = &node.dir_create {
		exec_dir_create(context, &dir_create)?;
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

pub fn exec_csv_file_append(context: &mut Context, node: &planner::CsvFileAppendNode) -> Result<(), Error> {
	let path = context.gen_table_file_path(&node.table_name)?;
	let file = match OpenOptions::new()
        .append(true)
        .create(true)
        .open(path) {
    	Ok(v) => v,
    	Err(e) => return err_exec!("failed to open file on append: {}", e),
    };
    let mut writer = Writer::from_writer(file);

	let mut row: Vec<String> = vec![];

	if let Some(expr_list) = &node.expr_list {
		let objs = exec_expr_list(context, &expr_list)?;
		for obj in objs.iter() {
			if let Some(o) = context.vars.get(obj.to_string().as_str()) {
				row.push(o.to_string());
			} else {
				return err_exec!("failed to get value of vars");
			}
		}
	}

	match writer.write_record(&row) {
		Ok(_) => {},
		Err(e) => return err_exec!("failed to write CSV record {}", e),
	}

	Ok(())
}

pub fn exec_use_db(context: &mut Context, node: &planner::UseDatabaseNode) -> Result<(), Error> {
	context.using_db_name = node.db_name.clone();
	Ok(())
}

pub fn print_selected_columns(context: &mut Context) -> Result<(), Error> {
	for col in context.selected_csv_columns.iter() {
		print!("{} ", col);
	}
	println!("");
	Ok(())
}

pub fn exec_project(context: &mut Context, node: &planner::ProjectNode) -> Result<bool, Error> {
	if node.filter.is_none() {
		if node.csv_scan.is_none() {
			return err_exec!("csv scan is none in project");
		}
		if let Some(csv_scan) = &node.csv_scan {
			while exec_csv_scan(context, csv_scan)? {
				select_get_columns(context, node)?;
				context.matched_csv_record = context.csv_record.clone();
				if context.is_cli {
					print_selected_columns(context)?;
				}
				if let Some(records) = context.test_get_records.as_mut() {
					records.push(context.csv_record.clone());
				}
				if !csv_scan.all {
					context.table_csv_reader = None;
					return Ok(false);
				}
				if context.is_sequential {
					return Ok(true);
				}
			}

			return Ok(false);
		}
	} else {
		if node.csv_scan.is_none() {
			return err_exec!("csv scan is none in project (2)");
		}
		if let Some(csv_scan) = &node.csv_scan {
			if let Some(filter) = &node.filter {
				context.counter_selected = 0;
				while exec_csv_scan(context, csv_scan)? {
					if exec_filter(context, filter)? {
						select_get_columns(context, node)?;
						if context.is_cli {
							print_selected_columns(context)?;
						}
						context.counter_selected += 1;
						if let Some(records) = context.test_get_records.as_mut() {
							records.push(context.csv_record.clone());
						}
					}
					if !csv_scan.all && context.counter_selected >= 1 {
						context.table_csv_reader = None;
						break;
					}
					if context.is_sequential {
						return Ok(true);
					}
				}

				return Ok(false);
			}
		}		
	}

	Ok(false)
}

pub fn exec_filter(context: &mut Context, node: &planner::FilterNode) -> Result<bool, Error> {
	if let Some(where_clause) = &node.where_clause {
		let o = exec_where_clause(context, where_clause)?;
		if o.bool_value {
			context.matched_csv_record = context.csv_record.clone();
			context.unmatched_csv_record.clear();
		} else {
			context.matched_csv_record.clear();
			context.unmatched_csv_record = context.csv_record.clone();
		}
		Ok(o.bool_value)
	} else {
		Ok(false)
	}
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
		let col = &context.csv_record[index];
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
		parser::CompareOpNode::Lte => {
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
						_ => err_exec!("can't compare f64 and other: a < b"),						
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
						_ => err_exec!("can't compare f64 and other: a < b"),						
					}
				}
				_ => err_exec!("can't compare"),
			}
		},
		parser::CompareOpNode::Gte => {
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
						_ => err_exec!("can't compare f64 and other: a < b"),						
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
	}
}

pub fn exec_operand(context: &mut Context, node: &parser::OperandNode) -> Result<Object, Error> {
	if node.i64_value.is_some() {
		if let Some(i64_value) = &node.i64_value {
			return Ok(exec_i64_value(context, i64_value)?);
		}
	} else if node.f64_value.is_some() {
		if let Some(f64_value) = &node.f64_value {
			return Ok(exec_f64_value(context, f64_value)?);
		}
	} else if node.string.is_some() {
		if let Some(string) = &node.string {
			return Ok(exec_string(context, string)?);
		}
	} else if node.ident.is_some() {
		if let Some(ident) = &node.ident {
			return Ok(exec_ident(context, ident)?);
		}
	} else if node.expr.is_some() {
		if let Some(expr) = &node.expr {
			return Ok(exec_expr(context, expr)?);
		}
	}
	err_exec!("invalid state of operand in exec")
}

pub fn exec_i64_value(_: &mut Context, node: &parser::IntValueNode) -> Result<Object, Error> {
	let mut o = Object::new();
	o.kind = ObjectKind::I64;
	o.i64_value = node.value;
	Ok(o)
}

pub fn exec_f64_value(_: &mut Context, node: &parser::FloatValueNode) -> Result<Object, Error> {
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

pub fn select_get_columns(context: &mut Context, node: &planner::ProjectNode) -> Result<(), Error> {
	let mut indices: Vec<usize> = vec![];

	for get_ident in node.get_stmt_idents.iter() {
		if let Some(index) = context.csv_header_idents.iter().position(|s| {
				return *s == *get_ident;
			}) {
			indices.push(index);
		}	
	}

	context.selected_csv_columns.clear();

	for index in indices {
		let col = &context.csv_record[index];
		context.selected_csv_columns.push(col.to_string());
	}

	Ok(())
}

pub fn exec_csv_scan(context: &mut Context, node: &planner::CsvScanNode) -> Result<bool, Error> {
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
		match reader.read_record(&mut context.csv_record) {
			Ok(_) => {
				if context.csv_record.len() == 0 {
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
	let path = context.gen_db_dir_path(&db_name)?;
	if path.as_os_str().is_empty() {
		return err_exec!("invalid path in dir delete all");
	}
	fs::remove_dir_all(&path).unwrap();
	Ok(())
}

pub fn exec_dir_list(context: &mut Context, node: &planner::DirListNode) -> Result<(), Error> {
	if node.csv_file_grep.is_some() {
		// show tables
		let path = context.gen_using_db_dir_path()?;
		let dir = match fs::read_dir(&path) {
			Ok(v) => v,
			Err(e) => return err_exec!("failed to read dir: {}", e),
		};
		for entry in dir {
			let entry = entry.unwrap();
			let path = entry.path();
			if path.extension().and_then(|s| s.to_str()) == Some("csv") {
				println!("{}", path.display());
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
			println!("{}", path.display());
		}
	}

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

pub fn exec_row_delete(context: &mut Context, node: &planner::RowDeleteNode, writer: &mut Writer<fs::File>) -> Result<(), Error> {
	if let Some(project) = &node.project {
		let seq = context.is_sequential;
		context.is_sequential = true;

		while exec_project(context, project)? {
			let cols = &context.unmatched_csv_record;
			if cols.len() > 0 {
				writer.write_record(cols).unwrap();
			}
		}

		context.is_sequential = seq;
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

pub fn exec_row_update(context: &mut Context, node: &planner::RowUpdateNode, writer: &mut Writer<fs::File>) -> Result<(), Error> {
	if let Some(project) = &node.project {
		if let Some(expr_list) = &node.expr_list {
			let update_expr_list_objs = exec_expr_list(context, expr_list)?;

			let seq = context.is_sequential;
			context.is_sequential = true;

			if node.all {
				while exec_project(context, project)? {
					if context.matched_csv_record.len() > 0 {
						let cols = context.matched_csv_record.clone();
						let cols = replace_columns_by_objs(context, &cols, &update_expr_list_objs)?;
						writer.write_record(&cols).unwrap();
					} else if context.unmatched_csv_record.len() > 0 {
						let cols = context.unmatched_csv_record.clone();
						writer.write_record(&cols).unwrap();
					} else {
						return err_exec!("invalid state: row update");
					}
				}
			} else {
				let mut writted = false;

				while exec_project(context, project)? {
					if writted {
						let cols = context.csv_record.clone();
						writer.write_record(&cols).unwrap();
					} else {
						if context.matched_csv_record.len() > 0 {
							let cols = context.matched_csv_record.clone();
							let cols = replace_columns_by_objs(context, &cols, &update_expr_list_objs)?;
							writer.write_record(&cols).unwrap();
							writted = true;
						} else if context.unmatched_csv_record.len() > 0 {
							let cols = context.unmatched_csv_record.clone();
							writer.write_record(&cols).unwrap();
						} else {
							return err_exec!("invalid state: row update");
						}
					}
				}
			}

			context.is_sequential = seq;

			return Ok(());
		}
	}

	return err_exec!("failed to row update");
}

pub fn exec_csv_file_rewrite(context: &mut Context, node: &planner::CsvFileRewriteNode) -> Result<(), Error> {
	if let Some(table_name) = &node.table_name {
		let org_path = context.gen_table_file_path(&table_name)?;
		let tmp_path = context.gen_tmp_table_file_path(&table_name)?;
		let headers = read_table_headers(context, &table_name)?;
		let mut writer = match Writer::from_path(&tmp_path) {
			Ok(v) => v,
			Err(e) => return err_exec!("failed to open CSV writer: {}", e),
		};
		writer.write_record(&headers).unwrap();

		if let Some(row_delete) = &node.row_delete {
			exec_row_delete(context, row_delete, &mut writer)?;
		} else if let Some(row_update) = &node.row_update {
			exec_row_update(context, row_update, &mut writer)?;
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
	if path.exists() {
		fs::remove_file(&path).unwrap();
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
		let node: QueryNode = parse(&mut tok_strm).unwrap();
		let node: PlansNode = planning(&node).unwrap();
		return exec(context, &node);
	}

	#[test]
	fn test_use_db() {
		let mut context = Context::new();
		do_exec(&mut context, "USE hige").unwrap();
		assert!(context.using_db_name == "hige");
	}

	#[test]
	fn test_dir_create() {
		let path = Path::new("test_env").join("test_db");
		if path.exists() {
			fs::remove_dir_all(&path).unwrap();
		}
		let mut context = Context::new();
		do_exec(&mut context, "CREATE DATABASE test_db").unwrap();
		assert!(path.exists());
	}

	#[test]
	fn test_csv_file_create() {
		let path = Path::new("test_env").join("test_db").join("test_table.csv");
		remove_file(&path);
		let mut context = Context::new();
		do_exec(&mut context, "CREATE DATABASE test_db").unwrap();
		do_exec(&mut context, "USE test_db").unwrap();
		do_exec(&mut context, "CREATE TABLE test_table (id: I64, weight: F64)").unwrap();
		assert!(path.exists());
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: I64,weight: F64\n");
	}

	#[test]
	fn test_csv_file_append() {
		let path = Path::new("test_env").join("test_db").join("test_table.csv");
		remove_file(&path);
		let mut context = Context::new();
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
	fn test_get_stmt_0() {
		let path = Path::new("test_env").join("test_db").join("test_table.csv");
		remove_file(&path);
		let mut context = Context::new();
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
		do_exec(&mut context, "GET ALL id, name OF test_table").unwrap();
		assert!(context.selected_csv_columns.len() == 2);
		assert!(context.selected_csv_columns[0] == "2");
		assert!(context.selected_csv_columns[1] == "hoge");
		do_exec(&mut context, "GET id, name OF test_table WHERE id == 2").unwrap();
		assert!(context.selected_csv_columns.len() == 2);
		assert!(context.selected_csv_columns[0] == "2");
		assert!(context.selected_csv_columns[1] == "hoge");
		do_exec(&mut context, "GET id, name OF test_table WHERE name == \"hoge\"").unwrap();
		assert!(context.selected_csv_columns.len() == 2);
		assert!(context.selected_csv_columns[0] == "2");
		assert!(context.selected_csv_columns[1] == "hoge");
		do_exec(&mut context, "GET id, name OF test_table WHERE id == 1 AND name == \"hige\"").unwrap();
		assert!(context.selected_csv_columns.len() == 2);
		assert!(context.selected_csv_columns[0] == "1");
		assert!(context.selected_csv_columns[1] == "hige");
	}

	fn csv_records_to_string(context: &mut Context) -> String {
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
		let path = Path::new("test_env").join("test_db").join("test_table.csv");
		remove_file(&path);
		let mut context = Context::new();
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
		let s = csv_records_to_string(&mut context);
		assert!(s == "1,3.14,hige\n2,3.14,hoge\n");
	}

	#[test]
	fn test_get_stmt_and_0() {
		let path = Path::new("test_env").join("test_db").join("test_table.csv");
		remove_file(&path);
		let mut context = Context::new();
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
		let s = csv_records_to_string(&mut context);
		assert!(s == "1,3.14,hige\n");
	}

	macro_rules! setup_records {
	    ($context:ident) => {
    		let path = Path::new("test_env").join("test_db").join("test_table.csv");
			remove_file(&path);
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

	#[test]
	fn test_get_stmt_or_and_0() {
		let mut context = Context::new();
		setup_records!(context);
		context.test_get_records = Some(vec![]);
		do_exec(&mut context, "GET ALL id, name OF test_table WHERE id == 1 OR weight == 3.14 AND name == \"hige\"").unwrap();
		let s = csv_records_to_string(&mut context);
		assert!(s == "1,3.14,hige\n");
	}

	#[test]
	fn test_get_stmt_or_and_1() {
		let mut context = Context::new();
		setup_records!(context);
		context.test_get_records = Some(vec![]);
		do_exec(&mut context, "GET ALL id, name OF test_table WHERE id == 1 OR weight == 100.0 AND name == \"hige\"").unwrap();
		let s = csv_records_to_string(&mut context);
		assert!(s == "1,3.14,hige\n");
	}

	#[test]
	fn test_get_stmt_or_and_2() {
		let mut context = Context::new();
		setup_records!(context);
		context.test_get_records = Some(vec![]);
		do_exec(&mut context, "GET ALL id, name OF test_table WHERE id == 0 OR weight == 100.0 AND name == \"hige\"").unwrap();
		let s = csv_records_to_string(&mut context);
		assert!(s == "");
	}

	#[test]
	fn test_get_stmt_or_and_3() {
		let mut context = Context::new();
		setup_records!(context);
		context.test_get_records = Some(vec![]);
		do_exec(&mut context, "GET ALL id, name OF test_table WHERE id == 0 OR weight == 3.14 AND name == \"hige\"").unwrap();
		let s = csv_records_to_string(&mut context);
		assert!(s == "1,3.14,hige\n");
	}

	#[test]
	fn test_get_stmt_or_and_4() {
		let mut context = Context::new();
		setup_records!(context);
		context.test_get_records = Some(vec![]);
		do_exec(&mut context, "GET ALL id, name OF test_table WHERE (id == 0 OR weight == 3.14) AND name == \"hige\"").unwrap();
		let s = csv_records_to_string(&mut context);
		assert!(s == "1,3.14,hige\n");
	}

	#[test]
	fn test_get_stmt_or_and_4a() {
		let mut context = Context::new();
		setup_records!(context);
		context.test_get_records = Some(vec![]);
		do_exec(&mut context, "GET ALL id, name OF test_table WHERE (id == 1 OR weight == 60) AND name == \"hige\"").unwrap();
		let s = csv_records_to_string(&mut context);
		println!("[{}]", s);
		assert!(s == "1,3.14,hige\n");
	}

	#[test]
	fn test_get_stmt_or_and_5() {
		let mut context = Context::new();
		setup_records!(context);
		context.test_get_records = Some(vec![]);
		do_exec(&mut context, "GET ALL id, name OF test_table WHERE (id == 0 OR weight == 3.14) AND name == \"moge\"").unwrap();
		let s = csv_records_to_string(&mut context);
		assert!(s == "");
	}

	#[test]
	fn test_drop_db() {
		let mut context = Context::new();
		do_exec(&mut context, "CREATE DATABASE test_db").unwrap();
		assert!(Path::new("test_env").join("test_db").exists());
		do_exec(&mut context, "DROP DATABASE test_db").unwrap();
		assert!(!Path::new("test_env").join("test_db").exists());
	}

	#[test]
	fn test_drop_table() {
		let path = Path::new("test_env").join("test_db").join("test_table.csv");
		remove_file(&path);
		let mut context = Context::new();
		do_exec(&mut context, "CREATE DATABASE test_db").unwrap();
		do_exec(&mut context, "USE test_db").unwrap();
		do_exec(&mut context, "CREATE TABLE test_table (id: I64, weight: F64, name: CHAR[128])").unwrap();
		assert!(path.exists());
		do_exec(&mut context, "DROP TABLE test_table").unwrap();
		assert!(!path.exists());
	}

	#[test]
	fn test_del_stmt_0() {
		let path = Path::new("test_env").join("test_db").join("test_table.csv");
		remove_file(&path);
		let mut context = Context::new();
		do_exec(&mut context, "CREATE DATABASE test_db").unwrap();
		do_exec(&mut context, "USE test_db").unwrap();
		do_exec(&mut context, "CREATE TABLE test_table (id: I64, weight: F64, name: CHAR[128])").unwrap();
		do_exec(&mut context, "ADD id = 1, weight = 3.14, name = \"hige\" OF test_table").unwrap();
		do_exec(&mut context, "ADD id = 2, weight = 3.14, name = \"hoge\" OF test_table").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: I64,weight: F64,name: CHAR[128]\n1,3.14,hige\n2,3.14,hoge\n");
		do_exec(&mut context, "DEL ALL OF test_table").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		println!("[{}]", s);
		assert!(s == "id: I64,weight: F64,name: CHAR[128]\n");
	}

	#[test]
	fn test_del_stmt_1() {
		let path = Path::new("test_env").join("test_db").join("test_table.csv");
		remove_file(&path);
		let mut context = Context::new();
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
	fn test_set_stmt_0() {
		let path = Path::new("test_env").join("test_db").join("test_table.csv");
		remove_file(&path);
		let mut context = Context::new();
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
		let path = Path::new("test_env").join("test_db").join("test_table.csv");
		remove_file(&path);
		let mut context = Context::new();
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
		let path = Path::new("test_env").join("test_db").join("test_table.csv");
		let mut context = Context::new();
		setup_records!(context);
		do_exec(&mut context, "SET id=10, name=\"HOGE\" OF test_table WHERE weight == 3.14").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: I64,weight: F64,name: CHAR[128]\n10,3.14,HOGE\n2,3.14,hoge\n");
	}

	#[test]
	fn test_set_stmt_2() {
		let path = Path::new("test_env").join("test_db").join("test_table.csv");
		let mut context = Context::new();
		setup_records!(context);
		do_exec(&mut context, "SET id=10, name=\"HOGE\" OF test_table WHERE name == \"hoge\"").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: I64,weight: F64,name: CHAR[128]\n1,3.14,hige\n10,3.14,HOGE\n");
	}

	#[test]
	fn test_set_stmt_3() {
		let path = Path::new("test_env").join("test_db").join("test_table.csv");
		let mut context = Context::new();
		setup_records!(context);
		do_exec(&mut context, "SET ALL name=\"HOGE\" OF test_table WHERE weight == 3.14").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: I64,weight: F64,name: CHAR[128]\n1,3.14,HOGE\n2,3.14,HOGE\n");
	}
	
	#[test]
	fn test_set_stmt_4() {
		let path = Path::new("test_env").join("test_db").join("test_table.csv");
		let mut context = Context::new();
		setup_records!(context);
		do_exec(&mut context, "SET ALL name=\"HOGE\" OF test_table").unwrap();
		let s = fs::read_to_string(&path).unwrap();
		assert!(s == "id: I64,weight: F64,name: CHAR[128]\n1,3.14,HOGE\n2,3.14,HOGE\n");
	}
}
