use crate::error::{Error, make_error, err_exec};
use crate::parser;
use crate::planner;
use crate::context::{Context};
use std::path::{Path};
use std::fs;
use std::io::{Write};
use csv::Reader;

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
	if node.filter.is_none() {
		if node.csv_scan.is_none() {
			return err_exec!("csv scan is none in project");
		}
		if let Some(csv_scan) = &node.csv_scan {
			while exec_csv_scan(context, csv_scan)? {
				select_get_columns(context, node)?;
			}
		}
	} else {
		if node.csv_scan.is_none() {
			return err_exec!("csv scan is none in project (2)");
		}
		if let Some(csv_scan) = &node.csv_scan {
			if let Some(filter) = &node.filter {
				while exec_csv_scan(context, csv_scan)? {
					if exec_filter(context, filter)? {
						select_get_columns(context, node)?;
					}
				}
			}
		}		
	}
	Ok(())
}

pub fn exec_filter(context: &mut Context, node: &planner::FilterNode) -> Result<bool, Error> {
	if let Some(where_clause) = &node.where_clause {
		let o = exec_where_clause(context, where_clause)?;
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

pub fn exec_expr(context: &mut Context, node: &parser::ExprNode) -> Result<Object, Error> {
	if let Some(ass_expr) = &node.ass_expr {
		Ok(exec_ass_expr(context, ass_expr)?)
	} else {
		err_exec!("impossible")
	}
}

pub fn exec_ass_expr(context: &mut Context, node: &parser::AssExprNode) -> Result<Object, Error> {
	if !node.right_logic_expr.is_none() {
		return err_exec!("can not use assign expr in filter");
	}
	if let Some(logic_expr) = &node.left_logic_expr {
		Ok(exec_logic_expr(context, logic_expr)?)
	} else {
		err_exec!("impossible")
	}
}

pub fn exec_logic_expr(context: &mut Context, node: &parser::LogicExprNode) -> Result<Object, Error> {
	let mut a;
	let mut b;
	let mut c = Object::new();

	if let parser::LogicExprItemNode::Left(compare_expr) = &node.nodes[0] {
		a = exec_compare_expr(context, &*compare_expr)?;	
	} else {
		return err_exec!("impossible");
	}

	for i in (1..node.nodes.len()).step_by(2) {
		let op = &node.nodes[i];
		let rhs = &node.nodes[i+1];

		if let parser::LogicExprItemNode::Right(compare_expr) = rhs {
			b = exec_compare_expr(context, &*compare_expr)?;
		} else {
			return err_exec!("impossible");
		}

		if let parser::LogicExprItemNode::Op(logic_op) = op {
			c = logical_objects(context, &a, &logic_op, &b)?;
			a = c.clone();
		}		
	}

	Ok(c)
}

pub fn exec_compare_expr(context: &mut Context, node: &parser::CompareExprNode) -> Result<Object, Error> {
	let mut a;
	let mut b;
	let mut c = Object::new();

	if let parser::CompareExprItemNode::Left(operand) = &node.nodes[0] {
		a = exec_operand(context, &*operand)?;	
	} else {
		return err_exec!("impossible");
	}

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

pub fn logical_objects(context: &mut Context, lhs: &Object, op: &parser::LogicOpNode, rhs: &Object) -> Result<Object, Error> {
	match op {
		parser::LogicOpNode::And => {
			match lhs.kind {
				ObjectKind::Bool => {
					match rhs.kind {
						ObjectKind::Bool => {
							let b = lhs.bool_value && rhs.bool_value;
							Ok(Object::from_bool(b))
						},
						ObjectKind::Ident => {
							let ro = refer_ident(context, &rhs.ident)?;
							Ok(logical_objects(context, lhs, op, &ro)?)
						},
						_ => err_exec!("can't compare: a && b"),
					}
				},
				ObjectKind::Ident => {
					match rhs.kind {
						ObjectKind::Bool => {
							let lo = refer_ident(context, &lhs.ident)?;
							Ok(logical_objects(context, &lo, op, rhs)?)
						},
						ObjectKind::Ident => {
							let lo = refer_ident(context, &lhs.ident)?;
							let ro = refer_ident(context, &rhs.ident)?;
							Ok(logical_objects(context, &lo, op, &ro)?)
						},
						_ => err_exec!("can't compare: a && b (2)")
					}
				}
				_ => err_exec!("can't compare: a && b (3)")
			}
		},
		parser::LogicOpNode::Or => {
			match lhs.kind {
				ObjectKind::Bool => {
					match rhs.kind {
						ObjectKind::Bool => {
							let b = lhs.bool_value || rhs.bool_value;
							Ok(Object::from_bool(b))
						}
						ObjectKind::Ident => {
							let ro = refer_ident(context, &rhs.ident)?;
							Ok(logical_objects(context, lhs, op, &ro)?)
						},
						_ => err_exec!("can't compare: a || b"),
					}
				},
				ObjectKind::Ident => {
					match rhs.kind {
						ObjectKind::Bool => {
							let lo = refer_ident(context, &lhs.ident)?;
							Ok(logical_objects(context, &lo, op, rhs)?)
						},
						ObjectKind::Ident => {
							let lo = refer_ident(context, &lhs.ident)?;
							let ro = refer_ident(context, &rhs.ident)?;
							Ok(logical_objects(context, &lo, op, &ro)?)
						},
						_ => err_exec!("can't compare: a && b (2)")
					}
				}
				_ => err_exec!("can't compare: a || b (2)")
			}
		},
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
						_ => err_exec!("can't compare i32 and other: a < b"),
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
						_ => err_exec!("can't compare f32 and other: a < b"),
					}
				}
				ObjectKind::Ident => {
					match rhs.kind {
						ObjectKind::I64 => {
							let lo = refer_ident(context, &lhs.ident)?;
							Ok(compare_objects(context, &lo, op, rhs)?)
						},
						ObjectKind::F64 => {
							let lo = refer_ident(context, &lhs.ident)?;
							Ok(compare_objects(context, &lo, op, rhs)?)
						},
						ObjectKind::Ident => {
							let lo = refer_ident(context, &lhs.ident)?;
							let ro = refer_ident(context, &rhs.ident)?;
							Ok(compare_objects(context, &lo, op, &ro)?)
						}
						_ => err_exec!("can't compare f32 and other: a < b"),						
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
						_ => err_exec!("can't compare i32 and other: a <= b"),
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
						_ => err_exec!("can't compare f32 and other: a <= b"),
					}
				}
				ObjectKind::Ident => {
					match rhs.kind {
						ObjectKind::I64 => {
							let lo = refer_ident(context, &lhs.ident)?;
							Ok(compare_objects(context, &lo, op, rhs)?)
						},
						ObjectKind::F64 => {
							let lo = refer_ident(context, &lhs.ident)?;
							Ok(compare_objects(context, &lo, op, rhs)?)
						},
						ObjectKind::Ident => {
							let lo = refer_ident(context, &lhs.ident)?;
							let ro = refer_ident(context, &rhs.ident)?;
							Ok(compare_objects(context, &lo, op, &ro)?)
						}
						_ => err_exec!("can't compare f32 and other: a < b"),						
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
						_ => err_exec!("can't compare i32 and other: a > b"),
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
						_ => err_exec!("can't compare f32 and other: a > b"),
					}
				}
				ObjectKind::Ident => {
					match rhs.kind {
						ObjectKind::I64 => {
							let lo = refer_ident(context, &lhs.ident)?;
							Ok(compare_objects(context, &lo, op, rhs)?)
						},
						ObjectKind::F64 => {
							let lo = refer_ident(context, &lhs.ident)?;
							Ok(compare_objects(context, &lo, op, rhs)?)
						},
						ObjectKind::Ident => {
							let lo = refer_ident(context, &lhs.ident)?;
							let ro = refer_ident(context, &rhs.ident)?;
							Ok(compare_objects(context, &lo, op, &ro)?)
						}
						_ => err_exec!("can't compare f32 and other: a < b"),						
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
						_ => err_exec!("can't compare i32 and other: a >= b"),
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
						_ => err_exec!("can't compare f32 and other: a >= b"),
					}
				}
				ObjectKind::Ident => {
					match rhs.kind {
						ObjectKind::I64 => {
							let lo = refer_ident(context, &lhs.ident)?;
							Ok(compare_objects(context, &lo, op, rhs)?)
						},
						ObjectKind::F64 => {
							let lo = refer_ident(context, &lhs.ident)?;
							Ok(compare_objects(context, &lo, op, rhs)?)
						},
						ObjectKind::Ident => {
							let lo = refer_ident(context, &lhs.ident)?;
							let ro = refer_ident(context, &rhs.ident)?;
							Ok(compare_objects(context, &lo, op, &ro)?)
						}
						_ => err_exec!("can't compare f32 and other: a < b"),						
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
						_ => err_exec!("can't compare i32 and other: a == b"),
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
						_ => err_exec!("can't compare f32 and other: a == b"),
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
						ObjectKind::I64 => {
							let lo = refer_ident(context, &lhs.ident)?;
							Ok(compare_objects(context, &lo, op, rhs)?)
						},
						ObjectKind::F64 => {
							let lo = refer_ident(context, &lhs.ident)?;
							Ok(compare_objects(context, &lo, op, rhs)?)
						},
						ObjectKind::Ident => {
							let lo = refer_ident(context, &lhs.ident)?;
							let ro = refer_ident(context, &rhs.ident)?;
							Ok(compare_objects(context, &lo, op, &ro)?)
						}
						_ => err_exec!("can't compare f32 and other: a < b"),						
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
						_ => err_exec!("can't compare i32 and other: a != b"),
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
						_ => err_exec!("can't compare f32 and other: a != b"),
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
						ObjectKind::I64 => {
							let lo = refer_ident(context, &lhs.ident)?;
							Ok(compare_objects(context, &lo, op, rhs)?)
						},
						ObjectKind::F64 => {
							let lo = refer_ident(context, &lhs.ident)?;
							Ok(compare_objects(context, &lo, op, rhs)?)
						},
						ObjectKind::Ident => {
							let lo = refer_ident(context, &lhs.ident)?;
							let ro = refer_ident(context, &rhs.ident)?;
							Ok(compare_objects(context, &lo, op, &ro)?)
						}
						_ => err_exec!("can't compare f32 and other: a < b"),						
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
		if let Some(index) = context.csv_header_idents.iter().position(|s| *s == *get_ident) {
			indices.push(index);
		}	
	}

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
			match reader.read_record(&mut context.csv_header) {
				Ok(_) => {},
				Err(_) => {
					context.table_csv_reader = None;
					return Ok(false);
				}
			};
		}

		parse_csv_header_idents(context)?;
	}	
	if let Some(reader) = context.table_csv_reader.as_mut() {
		match reader.read_record(&mut context.csv_record) {
			Ok(_) => return Ok(true),
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
			context.csv_header_idents.push(left.trim().to_string());
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
