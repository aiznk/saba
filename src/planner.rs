use std::fs::File;
use crate::parser::{QueryNode};
use crate::parser;
use crate::error::{Error, make_error, err_planning};

pub struct PlansNode {
	pub plans: Vec<PlanNode>,
}

impl PlansNode {
	pub fn new() -> Self {
		Self {
			plans: vec![],
		}
	}
}

pub struct PlanNode {
	pub use_db: Option<Box<UseDatabaseNode>>,
	pub project: Option<Box<ProjectNode>>,
	pub dir_create: Option<Box<DirCreateNode>>,
	pub csv_file_create: Option<Box<CsvFileCreateNode>>,
}

impl PlanNode {
	pub fn new() -> Self {
		Self {
			use_db: None,
			project: None,
			dir_create: None,
			csv_file_create: None,
		}
	}
}

pub struct UseDatabaseNode {
	pub db_name: String,
}

impl UseDatabaseNode {
	pub fn new() -> Self {
		Self {
			db_name: String::new(),
		}
	}
}

pub struct ProjectNode {
	pub get_stmt_idents: Vec<String>,
	pub csv_scan: Option<Box<CsvScan>>,
	pub filter: Option<Box<FilterNode>>,
}

impl ProjectNode {
	pub fn new() -> Self {
		Self {
			get_stmt_idents: vec![],
			csv_scan: None,
			filter: None,
		}
	}
}

pub struct FilterNode {
	pub where_clause: Option<Box<parser::WhereClauseNode>>,
}

impl FilterNode {
	pub fn new() -> Self {
		Self {
			where_clause: None,
		}
	}
}

pub struct CsvScan {
	file: Option<File>,
	fname: String,
}

impl CsvScan {
	pub fn new() -> Self {
		Self {
			file: None,
			fname: String::new(),
		}
	}
}

#[derive(Debug, Clone)]
pub struct DirCreateNode {
	pub dir_name: String,
}

impl DirCreateNode {
	pub fn new() -> Self {
		Self {
			dir_name: String::new(),
		}
	}
}

#[derive(Debug, Clone)]
pub struct CsvFileCreateNode {
	pub table_name: String,
	pub column_definitions: Vec<Box<parser::ColumnDefinitionNode>>,
}

impl CsvFileCreateNode {
	pub fn new() -> Self {
		Self {
			table_name: String::new(),
			column_definitions: vec![],
		}
	}
}

pub fn planning(node: &QueryNode) -> Result<PlansNode, Error> {
	let mut plans = PlansNode::new();
	plan_query(node, &mut plans)?;
	Ok(plans)
}

pub fn plan_query(node: &QueryNode, plans: &mut PlansNode) -> Result<(), Error> {
	for stmt in node.stmts.iter() {
		let mut plan = PlanNode::new();
		plan_stmt(&stmt, &mut plan)?;
		plans.plans.push(plan);
	}
	Ok(())
}

pub fn plan_stmt(node: &Box<parser::StmtNode>, plan: &mut PlanNode) -> Result<(), Error> {
	if !node.use_stmt.is_none() {
		if let Some(use_stmt) = &node.use_stmt {
			plan_use_stmt(&use_stmt, plan)?
		}
	} else if !node.create_stmt.is_none() {
		if let Some(create_stmt) = &node.create_stmt {
			plan_create_stmt(&create_stmt, plan)?
		}
	} else if !node.get_stmt.is_none() {
		if let Some(get_stmt) = &node.get_stmt {
			plan_get_stmt(&get_stmt, plan)?;
		}
	}
	Ok(())
}

pub fn plan_use_stmt(node: &Box<parser::UseStmtNode>, plan: &mut PlanNode) -> Result<(), Error> {
	let mut use_db = UseDatabaseNode::new();

	if let Some(db_name) = &node.db_name {
		use_db.db_name = unwrap_ident(&db_name)?;
	} else {
		return err_planning!("missing database name in use stmt");
	}

	plan.use_db = Some(Box::new(use_db));

	Ok(())
}

pub fn plan_create_stmt(node: &Box<parser::CreateStmtNode>, plan: &mut PlanNode) -> Result<(), Error> {
	if !node.create_database.is_none() {
		if let Some(create_database) = &node.create_database {
			return plan_create_database(&create_database, plan);
		} else {
			return err_planning!("missing create database");
		}
	} else if !node.create_table.is_none() {
		if let Some(create_table) = &node.create_table {
			return plan_create_table(&create_table, plan);
		} else {
			return err_planning!("missing create table");
		}
	} else {
		return err_planning!("invalid state in create stmt");
	}
}

pub fn plan_create_database(node: &Box<parser::CreateDatabaseNode>, plan: &mut PlanNode) -> Result<(), Error> {
	let mut dir_create = DirCreateNode::new();

	if let Some(ident) = &node.ident {
		dir_create.dir_name = unwrap_ident(&ident)?;
	}

	plan.dir_create = Some(Box::new(dir_create));

	Ok(())
}

pub fn plan_create_table(node: &Box<parser::CreateTableNode>, plan: &mut PlanNode) -> Result<(), Error> {
	let mut csv_file_create = CsvFileCreateNode::new();

	if let Some(ident) = &node.ident {
		csv_file_create.table_name = unwrap_ident(&ident)?;
	}

	csv_file_create.column_definitions = node.column_definitions.clone();

	plan.csv_file_create = Some(Box::new(csv_file_create));

	Ok(())
}

pub fn plan_get_stmt(node: &Box<parser::GetStmtNode>, plan: &mut PlanNode) -> Result<(), Error> {
	let mut project = ProjectNode::new();

	if let Some(expr_list) = &node.expr_list {
		for expr in expr_list.exprs.iter() {
			let ident: String = unwrap_expr_ident(&expr)?;
			project.get_stmt_idents.push(ident);
		}
	}

	if let Some(table) = &node.table {
		let mut csv_scan = CsvScan::new();
		csv_scan.fname = unwrap_ident(&table)?;
		project.csv_scan = Some(Box::new(csv_scan));
	}

	if let Some(where_clause) = &node.where_clause {
		let mut filter = FilterNode::new();
		filter.where_clause = Some((*where_clause).clone());
		project.filter = Some(Box::new(filter));
	} 

	plan.project = Some(Box::new(project));

	Ok(())
}

fn unwrap_expr_ident(node: &Box<parser::ExprNode>) -> Result<String, Error> {
	if let Some(ass_expr) = &node.ass_expr {
		return Ok(unwrap_ass_expr_ident(&ass_expr)?);
	}
	err_planning!("failed")
}

fn unwrap_ass_expr_ident(node: &Box<parser::AssExprNode>) -> Result<String, Error> {
	if let Some(compare_expr) = &node.left_compare_expr {
		return Ok(unwrap_compare_expr_ident(&compare_expr)?);
	}
	err_planning!("failed")
}

fn unwrap_compare_expr_ident(node: &Box<parser::CompareExprNode>) -> Result<String, Error> {
	let item: &parser::CompareExprItemNode = &node.nodes[0];
	if let parser::CompareExprItemNode::Left(logic_expr) = item {
		return Ok(unwrap_logic_expr_ident(logic_expr)?);
	}
	err_planning!("failed")
}

fn unwrap_logic_expr_ident(node: &Box<parser::LogicExprNode>) -> Result<String, Error> {
	let item: &parser::LogicExprItemNode = &node.nodes[0];
	if let parser::LogicExprItemNode::Left(operand) = item {
		return Ok(unwrap_operand_ident(&operand)?);
	}
	err_planning!("failed")
}

fn unwrap_operand_ident(node: &Box<parser::OperandNode>) -> Result<String, Error> {
	if let Some(ident) = &node.ident {
		return Ok(unwrap_ident(&ident)?);
	}
	err_planning!("failed")
}

fn unwrap_ident(node: &Box<parser::IdentNode>) -> Result<String, Error> {
	Ok(node.value.clone())
}

