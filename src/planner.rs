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
	pub dir_list: Option<Box<DirListNode>>,
	pub dir_delete_all: Option<Box<DirDeleteAllNode>>,
	pub csv_file_create: Option<Box<CsvFileCreateNode>>,
	pub csv_file_append: Option<Box<CsvFileAppendNode>>,
	pub csv_file_delete: Option<Box<CsvFileDeleteNode>>,
}

impl PlanNode {
	pub fn new() -> Self {
		Self {
			use_db: None,
			project: None,
			dir_create: None,
			dir_list: None,
			dir_delete_all: None,
			csv_file_create: None,
			csv_file_append: None,
			csv_file_delete: None,
		}
	}
}

pub struct DirDeleteAllNode {
	pub db_name: Option<String>,
}

impl DirDeleteAllNode {
	pub fn new() -> Self {
		Self {
			db_name: None,
		}
	}
}

pub struct CsvFileDeleteNode {
	pub table_name: Option<String>,
}

impl CsvFileDeleteNode {
	pub fn new() -> Self {
		Self {
			table_name: None,
		}
	}
}

pub struct DirListNode {
	pub csv_file_grep: Option<Box<CsvFileGrepNode>>,
}

impl DirListNode {
	pub fn new() -> Self {
		Self {
			csv_file_grep: None,
		}
	}
}

pub struct CsvFileGrepNode {
	dummy: i32,
}

impl CsvFileGrepNode {
	pub fn new() -> Self {
		Self {
			dummy: 0,
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

pub struct CsvFileAppendNode {
	pub table_name: String,
	pub expr_list: Option<Box<parser::ExprListNode>>,
}

impl CsvFileAppendNode {
	pub fn new() -> Self {
		Self {
			table_name: String::new(),
			expr_list: None,
		}
	}
}

pub struct ProjectNode {
	pub get_stmt_idents: Vec<String>,
	pub csv_scan: Option<Box<CsvScanNode>>,
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

pub struct CsvScanNode {
	pub table_name: String,
	pub all: bool,
}

impl CsvScanNode {
	pub fn new() -> Self {
		Self {
			table_name: String::new(),
			all: false,
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
	pub csv_head_row: String,
}

impl CsvFileCreateNode {
	pub fn new() -> Self {
		Self {
			table_name: String::new(),
			csv_head_row: String::new(),
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
	if node.show_stmt.is_some() {
		if let Some(show_stmt) = &node.show_stmt {
			plan_show_stmt(&show_stmt, plan)?
		}		
	} else if node.drop_stmt.is_some() {
		if let Some(drop_stmt) = &node.drop_stmt {
			plan_drop_stmt(&drop_stmt, plan)?
		}	
	} else if node.use_stmt.is_some() {
		if let Some(use_stmt) = &node.use_stmt {
			plan_use_stmt(&use_stmt, plan)?
		}
	} else if node.create_stmt.is_some() {
		if let Some(create_stmt) = &node.create_stmt {
			plan_create_stmt(&create_stmt, plan)?
		}
	} else if node.get_stmt.is_some() {
		if let Some(get_stmt) = &node.get_stmt {
			plan_get_stmt(&get_stmt, plan)?;
		}
	} else if node.add_stmt.is_some() {
		if let Some(add_stmt) = &node.add_stmt {
			plan_add_stmt(&add_stmt, plan)?;
		}
	}
	Ok(())
}

pub fn plan_drop_stmt(node: &Box<parser::DropStmtNode>, plan: &mut PlanNode) -> Result<(), Error> {
	if node.table_name.is_some() {
		let mut csv_file_delete = CsvFileDeleteNode::new();
		csv_file_delete.table_name = Some(unwrap_ident(&node.table_name.clone().unwrap())?);
		plan.csv_file_delete = Some(Box::new(csv_file_delete));
	} else if node.db_name.is_some() {
		let mut dir_delete_all = DirDeleteAllNode::new();
		dir_delete_all.db_name = Some(unwrap_ident(&node.db_name.clone().unwrap())?);
		plan.dir_delete_all = Some(Box::new(dir_delete_all));
	} else {
		return err_planning!("invalid state: drop stmt");
	}

	Ok(())
}

pub fn plan_show_stmt(node: &Box<parser::ShowStmtNode>, plan: &mut PlanNode) -> Result<(), Error> {
	let mut dir_list = DirListNode::new();

	if node.tables {
		dir_list.csv_file_grep = Some(Box::new(CsvFileGrepNode::new()));
	}

	plan.dir_list = Some(Box::new(dir_list));

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

	csv_file_create.csv_head_row = gen_csv_head_row_by_column_defs(&node.column_definitions)?;

	plan.csv_file_create = Some(Box::new(csv_file_create));

	Ok(())
}

pub fn gen_csv_head_row_by_column_defs(column_defs: &Vec<Box<parser::ColumnDefinitionNode>>) -> Result<String, Error> {
	let mut s = String::new();

	for column_def in column_defs.iter() {
		let col = gen_csv_head_col_by_column_def(column_def)?;
		s.push_str(col.as_str());
		s.push_str(",");
	}

	s.pop();
	s.push_str("\n");

	Ok(s)
}

pub fn gen_csv_head_col_by_column_def(column_def: &Box<parser::ColumnDefinitionNode>) -> Result<String, Error> {
	let mut s = String::new();

	if let Some(ident) = &column_def.ident {
		s.push_str(&unwrap_ident(&ident)?);	
	} else {
		return err_planning!("missing ident");
	}

	s.push_str(": ");

	for column_type in column_def.column_types.iter() {
		let t = gen_csv_head_col_type_by_column_type(column_type)?;
		s.push_str(&t);
		s.push_str(" ");
	}

	s.pop();

	Ok(s)
}

pub fn gen_csv_head_col_type_by_column_type(column_type: &parser::ColumnTypeNode) -> Result<String, Error> {
	match column_type {
		parser::ColumnTypeNode::PrimaryKey => Ok(String::from("PRIMARY_KEY")),
		parser::ColumnTypeNode::AutoIncrement => Ok(String::from("AUTO_INCREMENT")),
		parser::ColumnTypeNode::I64 => Ok(String::from("I64")),
		parser::ColumnTypeNode::F64 => Ok(String::from("F64")),
		parser::ColumnTypeNode::Char(nelems) => Ok(format!("CHAR[{}]", nelems)),
	}
}

pub fn plan_add_stmt(node: &Box<parser::AddStmtNode>, plan: &mut PlanNode) -> Result<(), Error> {
	let mut append = CsvFileAppendNode::new();

	if let Some(expr_list) = &node.expr_list {
		append.expr_list = Some(expr_list.clone());
	}	

	if let Some(table) = &node.table {
		append.table_name = unwrap_ident(&table)?;
	}
	
	plan.csv_file_append = Some(Box::new(append));

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
		let mut csv_scan = CsvScanNode::new();
		csv_scan.table_name = unwrap_ident(&table)?;
		csv_scan.all = node.all;
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
	if node.right_logic_expr.is_some() {
		return err_planning!("can't assign in ass expr: a = b");
	}
	if let Some(logic_expr) = &node.left_logic_expr {
		return Ok(unwrap_logic_expr_ident(&logic_expr)?);
	}
	err_planning!("failed")
}

fn unwrap_compare_expr_ident(node: &Box<parser::CompareExprNode>) -> Result<String, Error> {
	if node.nodes.len() == 0 {
		return err_planning!("nodes len is 0 in unwrap compare expr ident");
	} else if node.nodes.len() >= 2 {
		return err_planning!("over nodes len in unwrap compare expr ident");
	}
	let item: &parser::CompareExprItemNode = &node.nodes[0];
	if let parser::CompareExprItemNode::Left(operand) = item {
		return Ok(unwrap_operand_ident(operand)?);
	}
	err_planning!("failed")
}

fn unwrap_logic_expr_ident(node: &Box<parser::LogicExprNode>) -> Result<String, Error> {
	if node.nodes.len() == 0 {
		return err_planning!("nodes len is 0 in unwrap logic expr ident");
	} else if node.nodes.len() >= 2 {
		return err_planning!("over nodes len in unwrap logic expr ident");
	}
	let item: &parser::LogicExprItemNode = &node.nodes[0];
	if let parser::LogicExprItemNode::Left(compare_expr) = item {
		return Ok(unwrap_compare_expr_ident(&compare_expr)?);
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

