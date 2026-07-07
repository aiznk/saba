use crate::parser::{FuncExprNode, QueryNode};
use crate::parser;
use crate::error::{Error, make_error, err_planning};
use crate::tokenizer::{TokenKind};
use crate::objects::{Object, ObjectKind};

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
	pub desc_table: Option<Box<DescTableNode>>,
	pub use_db: Option<Box<UseDatabaseNode>>,
	pub sort: Option<Box<SortNode>>,
	pub aggregate: Option<Box<AggregateNode>>,
	pub project: Option<Box<ProjectNode>>,
	pub database_create: Option<Box<DatabaseCreateNode>>,
	pub dir_list: Option<Box<DirListNode>>,
	pub dir_delete_all: Option<Box<DirDeleteAllNode>>,
	pub csv_file_create: Option<Box<CsvFileCreateNode>>,
	pub csv_file_append: Option<Box<CsvFileAppendNode>>,
	pub csv_file_delete: Option<Box<CsvFileDeleteNode>>,
	pub csv_file_rewrite: Option<Box<CsvFileRewriteNode>>,
	pub csv_file_rename: Option<Box<CsvFileRenameNode>>,
}

impl PlanNode {
	pub fn new() -> Self {
		Self {
			desc_table: None,
			use_db: None,
			sort: None,
			aggregate: None,
			project: None,
			database_create: None,
			dir_list: None,
			dir_delete_all: None,
			csv_file_create: None,
			csv_file_append: None,
			csv_file_delete: None,
			csv_file_rewrite: None,
			csv_file_rename: None,
		}
	}
}

pub struct CsvFileRewriteNode {
	pub table_name: Option<String>,
	pub row_delete: Option<Box<RowDeleteNode>>,
	pub row_update: Option<Box<RowUpdateNode>>,
	pub column_add: Option<Box<ColumnAddNode>>,
	pub column_drop: Option<Box<ColumnDropNode>>,
	pub column_rename: Option<Box<ColumnRenameNode>>,
	pub column_alter_type: Option<Box<ColumnAlterTypeNode>>,
}

impl CsvFileRewriteNode {
	pub fn new() -> Self {
		Self {
			table_name: None,
			row_delete: None,
			row_update: None,
			column_add: None,
			column_drop: None,
			column_rename: None,
			column_alter_type: None,
		}
	}
}

pub struct SortNode {
	pub expr: Option<Box<parser::ExprNode>>,
	pub project: Option<Box<ProjectNode>>,
	pub is_asc: bool,
	pub all: bool,
}

impl SortNode {
	pub fn new() -> Self {
		Self {
			expr: None,
			project: None,
			is_asc: true,
			all: false,
		}
	}
}

pub struct CsvFileRenameNode {
	pub table_name: Option<String>,
	pub to_ident: Option<String>,
}

impl CsvFileRenameNode {
	pub fn new() -> Self {
		Self {
			table_name: None,
			to_ident: None,
		}
	}
}

pub struct ColumnAlterTypeNode {
	pub project: Option<Box<ProjectNode>>,
	pub ident: Option<String>,
	pub column_types: Vec<parser::ColumnTypeNode>,
}

impl ColumnAlterTypeNode {
	pub fn new() -> Self {
		Self {
			project: None,
			ident: None,
			column_types: vec![],
		}
	}
}

pub struct ColumnRenameNode {
	pub project: Option<Box<ProjectNode>>,
	pub from_ident: Option<String>,
	pub to_ident: Option<String>,
}

impl ColumnRenameNode {
	pub fn new() -> Self {
		Self {
			project: None,
			from_ident: None,
			to_ident: None,
		}
	}
}

pub struct ColumnDropNode {
	pub ident: Option<String>,
	pub project: Option<Box<ProjectNode>>,
}

impl ColumnDropNode {
	pub fn new() -> Self {
		Self {
			ident: None,
			project: None,
		}
	}
}

pub struct ColumnAddNode {
	pub ident: Option<String>,
	pub column_types_string: Option<String>,
	pub column_definition_string: Option<String>,
	pub project: Option<Box<ProjectNode>>,
}

impl ColumnAddNode {
	pub fn new() -> Self {
		Self {
			ident: None,
			column_types_string: None,
			column_definition_string: None,
			project: None,
		}
	}
}

pub struct DescTableNode {
	pub table_name: Option<String>,
}

impl DescTableNode {
	pub fn new() -> Self {
		Self {
			table_name: None,
		}
	}
}

pub struct RowDeleteNode {
	pub all: bool,
	pub project: Option<Box<ProjectNode>>,
}

impl RowDeleteNode {
	pub fn new() -> Self {
		Self {
			all: false,
			project: None,
		}
	}
}

pub struct RowUpdateNode {
	pub project: Option<Box<ProjectNode>>,
	pub expr_list: Option<Box<parser::ExprListNode>>,
	pub all: bool,
}

impl RowUpdateNode {
	pub fn new() -> Self {
		Self {
			project: None,
			expr_list: None,
			all: false,
		}
	}
}

pub struct DirDeleteAllNode {
	pub db_name: Option<String>,
	pub if_exists: bool,
}

impl DirDeleteAllNode {
	pub fn new() -> Self {
		Self {
			db_name: None,
			if_exists: false,
		}
	}
}

pub struct CsvFileDeleteNode {
	pub table_name: Option<String>,
	pub if_exists: bool,
}

impl CsvFileDeleteNode {
	pub fn new() -> Self {
		Self {
			table_name: None,
			if_exists: false,
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
	#[allow(dead_code)]
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
	pub paren_idents: Option<Box<parser::ParenIdentsNode>>,
	pub paren_values_list: Vec<Box<parser::ParenValuesNode>>,
}

impl CsvFileAppendNode {
	pub fn new() -> Self {
		Self {
			table_name: String::new(),
			expr_list: None,
			paren_idents: None,
			paren_values_list: vec![],
		}
	}
}

pub struct AggregateNode {
	pub filter: Option<Box<FilterNode>>,
	pub limit: Option<Box<parser::LimitNode>>,
	pub all: bool,
	pub expr_list: Option<Box<parser::ExprListNode>>,
}

impl AggregateNode {
	pub fn new() -> Self {
		Self {
			filter: None,
			limit: None,
			all: false,
			expr_list: None,
		}
	}
}

pub struct ProjectNode {
	pub method: TokenKind,
	pub filter: Option<Box<FilterNode>>,
	pub limit: Option<Box<parser::LimitNode>>,
	pub all: bool,
	pub expr_list: Option<Box<parser::ExprListNode>>,
}

impl ProjectNode {
	pub fn new() -> Self {
		Self {
			method: TokenKind::Nil,
			filter: None,
			limit: None,
			all: false,
			expr_list: None,
		}
	}
}

pub struct FilterNode {
	pub where_clause: Option<Box<parser::WhereClauseNode>>,
	pub csv_file_scan: Option<Box<CsvFileScanNode>>,
}

impl FilterNode {
	pub fn new() -> Self {
		Self {
			where_clause: None,
			csv_file_scan: None,
		}
	}
}

pub struct CsvFileScanNode {
	pub table_name: String,
	pub all: bool,
}

impl CsvFileScanNode {
	pub fn new() -> Self {
		Self {
			table_name: String::new(),
			all: false,
		}
	}
}

#[derive(Debug, Clone)]
pub struct DatabaseCreateNode {
	pub db_name: String,
}

impl DatabaseCreateNode {
	pub fn new() -> Self {
		Self {
			db_name: String::new(),
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
	if let Some(show_stmt) = &node.show_stmt {
		plan_show_stmt(&show_stmt, plan)?
	} else if let Some(desc_stmt) = &node.desc_stmt {
		plan_desc_stmt(&desc_stmt, plan)?
	} else if let Some(alter_stmt) = &node.alter_stmt {
		plan_alter_stmt(&alter_stmt, plan)?
	} else if let Some(drop_stmt) = &node.drop_stmt {
		plan_drop_stmt(&drop_stmt, plan)?
	} else if let Some(use_stmt) = &node.use_stmt {
		plan_use_stmt(&use_stmt, plan)?
	} else if let Some(create_stmt) = &node.create_stmt {
		plan_create_stmt(&create_stmt, plan)?
	} else if let Some(get_stmt) = &node.get_stmt {
		plan_get_stmt(&get_stmt, plan)?;
	} else if let Some(set_stmt) = &node.set_stmt {
		plan_set_stmt(&set_stmt, plan)?;
	} else if let Some(add_stmt) = &node.add_stmt {
		plan_add_stmt(&add_stmt, plan)?;
	} else if let Some(del_stmt) = &node.del_stmt {
		plan_del_stmt(&del_stmt, plan)?;
	}
	Ok(())
}

pub fn plan_desc_stmt(node: &Box<parser::DescStmtNode>, plan: &mut PlanNode) -> Result<(), Error> {
	let mut desc_table = DescTableNode::new();

	if let Some(table_name) = &node.table_name {
		let table_name = unwrap_ident_object(table_name)?.to_string();
		desc_table.table_name = Some(table_name);
		plan.desc_table = Some(Box::new(desc_table));
		Ok(())
	} else {
		err_planning!("invalid state: desc stmt")
	}
}

pub fn plan_alter_stmt(node: &Box<parser::AlterStmtNode>, plan: &mut PlanNode) -> Result<(), Error> {
	if let Some(alter_table) = &node.alter_table {
		plan_alter_table(alter_table, plan)?;
		Ok(())
	} else {
		err_planning!("invalid state: alter stmt")
	}
}

fn gen_column_types_string(column_types: &Vec<parser::ColumnTypeNode>) -> Result<String, Error> {
	let mut scolumn_type = String::new();

	for column_type in column_types.iter() {
		let stype = gen_csv_head_col_type_by_column_type(&column_type)?;
		scolumn_type.push_str(stype.as_str());
		scolumn_type.push(' ');
	}

	scolumn_type.pop();
	Ok(scolumn_type)
}

pub fn plan_alter_table(node: &Box<parser::AlterTableNode>, plan: &mut PlanNode) -> Result<(), Error> {
	if let Some(alter_add_column) = &node.alter_add_column {
		let mut n = CsvFileRewriteNode::new();
		let mut column_add = ColumnAddNode::new();
		let mut project = ProjectNode::new();
		let mut filter = FilterNode::new();
		let mut csv_file_scan = CsvFileScanNode::new();

		project.method = TokenKind::Alter;
		project.all = true; // always true

		if let Some(table_name) = &node.table_name {
			let table_name = unwrap_ident_object(table_name)?.to_string();
			n.table_name = Some(table_name.clone());
			csv_file_scan.table_name = table_name.clone();
			csv_file_scan.all = true; // always true
		} else {
			return err_planning!("missing table name in plan alter table");
		}

		if let Some(ident) = &alter_add_column.ident {
			let ident = unwrap_ident_object(ident)?.to_string();
			column_add.ident = Some(ident);
		} else {
			return err_planning!("missing column ident in plan alter table");
		}
		let column_types = alter_add_column.column_types.clone();
		column_add.column_types_string = Some(gen_column_types_string(&column_types)?);

		let mut new_type = String::new();
		if let Some(ident) = &column_add.ident {
			new_type.push_str(ident.as_str());
		}
		new_type.push_str(": ");
		if let Some(column_type_string) = &column_add.column_types_string {
			new_type.push_str(column_type_string.as_str());
		}
		column_add.column_definition_string = Some(new_type);

		filter.csv_file_scan = Some(Box::new(csv_file_scan));
		project.filter = Some(Box::new(filter));
		column_add.project = Some(Box::new(project));
		n.column_add = Some(Box::new(column_add));
		plan.csv_file_rewrite = Some(Box::new(n));

	} else if let Some(alter_drop_column) = &node.alter_drop_column {
		let mut n = CsvFileRewriteNode::new();
		let mut column_drop = ColumnDropNode::new();
		let mut project = ProjectNode::new();
		let mut filter = FilterNode::new();
		let mut csv_file_scan = CsvFileScanNode::new();

		project.all = true; // always true

		if let Some(table_name) = &node.table_name {
			let table_name = unwrap_ident_object(table_name)?.to_string();
			n.table_name = Some(table_name.clone());
			csv_file_scan.table_name = table_name.clone();
			csv_file_scan.all = true; // always true
		} else {
			return err_planning!("missing table name in plan alter table");
		}

		if let Some(ident) = &alter_drop_column.ident {
			let ident = unwrap_ident_object(ident)?.to_string();
			column_drop.ident = Some(ident);
		}

		filter.csv_file_scan = Some(Box::new(csv_file_scan));
		project.filter = Some(Box::new(filter));
		column_drop.project = Some(Box::new(project));
		n.column_drop = Some(Box::new(column_drop));
		plan.csv_file_rewrite = Some(Box::new(n));

	} else if let Some(alter_rename_column) = &node.alter_rename_column {
		let mut n = CsvFileRewriteNode::new();
		let mut column_rename = ColumnRenameNode::new();
		let mut project = ProjectNode::new();
		let mut filter = FilterNode::new();
		let mut csv_file_scan = CsvFileScanNode::new();

		project.all = true; // always true

		if let Some(table_name) = &node.table_name {
			let table_name = unwrap_ident_object(table_name)?.to_string();
			n.table_name = Some(table_name.clone());
			csv_file_scan.table_name = table_name.clone();
			csv_file_scan.all = true; // always true
		} else {
			return err_planning!("missing table name in plan alter table");
		}

		if let Some(ident) = &alter_rename_column.from_ident {
			let ident = unwrap_ident_object(ident)?.to_string();
			column_rename.from_ident = Some(ident);
		}
		if let Some(ident) = &alter_rename_column.to_ident {
			let ident = unwrap_ident_object(ident)?.to_string();
			column_rename.to_ident = Some(ident);
		}

		filter.csv_file_scan = Some(Box::new(csv_file_scan));
		project.filter = Some(Box::new(filter));
		column_rename.project = Some(Box::new(project));
		n.column_rename = Some(Box::new(column_rename));
		plan.csv_file_rewrite = Some(Box::new(n));

	} else if let Some(alter_rename_table) = &node.alter_rename_table {
		let mut csv_file_rename = CsvFileRenameNode::new();

		if let Some(table_name) = &node.table_name {
			let table_name = unwrap_ident_object(table_name)?.to_string();
			csv_file_rename.table_name = Some(table_name.clone());
		} else {
			return err_planning!("missing table name in plan alter table");
		}

		if let Some(ident) = &alter_rename_table.to_ident {
			csv_file_rename.to_ident = Some(unwrap_ident_object(ident)?.to_string());
		}

		plan.csv_file_rename = Some(Box::new(csv_file_rename));

	} else if let Some(alter_column_type) = &node.alter_column_type {
		let mut rewrite = CsvFileRewriteNode::new();
		let mut column_alter_type = ColumnAlterTypeNode::new();
		let mut project = ProjectNode::new();
		let mut filter = FilterNode::new();
		let mut csv_file_scan = CsvFileScanNode::new();

		project.all = true; // always true

		if let Some(table_name) = &node.table_name {
			let table_name = unwrap_ident_object(table_name)?.to_string();
			rewrite.table_name = Some(table_name.clone());
			csv_file_scan.table_name = table_name.clone();
			csv_file_scan.all = true; // always true
		} else {
			return err_planning!("missing table name in plan alter table");
		}

		if let Some(ident) = &alter_column_type.ident {
			let ident = unwrap_ident_object(ident)?.to_string();
			column_alter_type.ident = Some(ident);
		}

		column_alter_type.column_types = alter_column_type.column_types.clone();

		filter.csv_file_scan = Some(Box::new(csv_file_scan));
		project.filter = Some(Box::new(filter));
		column_alter_type.project = Some(Box::new(project));
		rewrite.column_alter_type = Some(Box::new(column_alter_type));
		plan.csv_file_rewrite = Some(Box::new(rewrite));
	} else {
		return err_planning!("invalid state: plan alter table");
	}

	Ok(())
}

pub fn plan_drop_stmt(node: &Box<parser::DropStmtNode>, plan: &mut PlanNode) -> Result<(), Error> {
	if node.table_name.is_some() {
		let mut csv_file_delete = CsvFileDeleteNode::new();
		csv_file_delete.table_name = Some(unwrap_ident_object(&node.table_name.clone().unwrap())?.to_string());
		csv_file_delete.if_exists = node.if_exists;
		plan.csv_file_delete = Some(Box::new(csv_file_delete));
	} else if node.db_name.is_some() {
		let mut dir_delete_all = DirDeleteAllNode::new();
		dir_delete_all.db_name = Some(unwrap_ident_object(&node.db_name.clone().unwrap())?.to_string());
		dir_delete_all.if_exists = node.if_exists;
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
		use_db.db_name = unwrap_ident_object(&db_name)?.to_string();
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
	let mut db_create = DatabaseCreateNode::new();

	if let Some(ident) = &node.ident {
		db_create.db_name = unwrap_ident_object(&ident)?.to_string();
	}

	plan.database_create = Some(Box::new(db_create));

	Ok(())
}

pub fn plan_create_table(node: &Box<parser::CreateTableNode>, plan: &mut PlanNode) -> Result<(), Error> {
	let mut csv_file_create = CsvFileCreateNode::new();

	if let Some(ident) = &node.ident {
		csv_file_create.table_name = unwrap_ident_object(&ident)?.to_string();
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
		s.push_str(&unwrap_ident_object(&ident)?.to_string());	
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
		parser::ColumnTypeNode::Bool => Ok(String::from("BOOL")),
		parser::ColumnTypeNode::Default(value) => {
			let obj = unwrap_value(&*value)?;
			let sobj;
			if obj.kind == ObjectKind::String {
				sobj = format!("\"{}\"", obj.to_string());
			} else {
				sobj = obj.to_string();
			}
			return Ok(format!("DEFAULT {}", sobj));
		}
	}
}

fn unwrap_value(node: &parser::ValueNode) -> Result<Object, Error> {
	if let Some(i64_value) = &node.i64_value {
		return Ok(unwrap_i64_value(&i64_value)?);
	} else if let Some(f64_value) = &node.f64_value {
		return Ok(unwrap_f64_value(&f64_value)?);
	} else if let Some(bool_value) = &node.bool_value {
		return Ok(unwrap_bool_value(&bool_value)?);		
	} else if let Some(string) = &node.string {
		return Ok(unwrap_string(&string)?);
	} else {
		return err_planning!("invalid state: unwrap value");
	}
}

fn unwrap_i64_value(node: &parser::I64ValueNode) -> Result<Object, Error> {
	Ok(Object::from_i64(node.value))
}

fn unwrap_f64_value(node: &parser::F64ValueNode) -> Result<Object, Error> {
	Ok(Object::from_f64(node.value))
}

fn unwrap_bool_value(node: &parser::BoolValueNode) -> Result<Object, Error> {
	Ok(Object::from_bool(node.value))
}

fn unwrap_string(node: &parser::StringNode) -> Result<Object, Error> {
	Ok(Object::from_string(node.value.clone()))
}

pub fn plan_add_stmt(node: &Box<parser::AddStmtNode>, plan: &mut PlanNode) -> Result<(), Error> {
	let mut append = CsvFileAppendNode::new();

	if let Some(expr_list) = &node.expr_list {
		append.expr_list = Some(expr_list.clone());
	}	

	if let Some(table) = &node.table {
		append.table_name = unwrap_ident_object(&table)?.to_string();
	}

	if node.expr_list.is_none() {
		if let Some(paren_idents) = &node.paren_idents {
			append.paren_idents = Some(paren_idents.clone());
		}

		append.paren_values_list = node.paren_values_list.clone();
	}
	
	plan.csv_file_append = Some(Box::new(append));

	Ok(())
}

pub fn plan_del_stmt(node: &Box<parser::DelStmtNode>, plan: &mut PlanNode) -> Result<(), Error> {
	let mut rewrite = CsvFileRewriteNode::new();
	let mut row_delete = RowDeleteNode::new();
	let mut project = ProjectNode::new();
	let mut filter = FilterNode::new();
	let mut csv_file_scan = CsvFileScanNode::new();

	project.method = TokenKind::Del;
	project.all = true;
	row_delete.all = node.all;

	if let Some(table) = &node.table {
		let ident = unwrap_ident_object(&table)?.to_string();
		csv_file_scan.table_name = ident.clone();
		csv_file_scan.all = true; // always true
		filter.csv_file_scan = Some(Box::new(csv_file_scan));
		rewrite.table_name = Some(ident.clone());
	}

	if let Some(where_clause) = &node.where_clause {
		filter.where_clause = Some((*where_clause).clone());
	}

	if let Some(limit) = &node.limit {
		project.limit = Some(limit.clone());
	}

	project.filter = Some(Box::new(filter));
	row_delete.project = Some(Box::new(project));
	rewrite.row_delete = Some(Box::new(row_delete));
	plan.csv_file_rewrite = Some(Box::new(rewrite));

	Ok(())
}

pub fn plan_set_stmt(node: &Box<parser::SetStmtNode>, plan: &mut PlanNode) -> Result<(), Error> {
	let mut rewrite = CsvFileRewriteNode::new();
	let mut row_update = RowUpdateNode::new();
	let mut project = ProjectNode::new();
	let mut filter = FilterNode::new();
	let mut csv_file_scan = CsvFileScanNode::new();

	project.method = TokenKind::Set;
	project.all = true; // always true
	
	if let Some(expr_list) = &node.expr_list {
		row_update.expr_list = Some(expr_list.clone());
	} else {
		return err_planning!("missing expr list in set stmt");
	}

	if let Some(table) = &node.table {
		let table_name = unwrap_ident_object(&table)?.to_string();
		csv_file_scan.table_name = table_name.clone();
		csv_file_scan.all = true; // the set stmt always needs all on csv_file_scan
		rewrite.table_name = Some(table_name);
	} else {
		return err_planning!("missing table name in set stmt");
	}

	if let Some(where_clause) = &node.where_clause {
		filter.where_clause = Some((*where_clause).clone());
	}

	if let Some(limit) = &node.limit {
		project.limit = Some(limit.clone());
	}

	filter.csv_file_scan = Some(Box::new(csv_file_scan));
	project.filter = Some(Box::new(filter));
	row_update.all = node.all;
	row_update.project = Some(Box::new(project));	
	rewrite.row_update = Some(Box::new(row_update));
	plan.csv_file_rewrite = Some(Box::new(rewrite));

	Ok(())
}

fn needs_aggregate(node: &Box<parser::GetStmtNode>) -> Result<bool, Error> {
	if let Some(expr_list) = &node.expr_list {
		return Ok(needs_aggregate_expr_list(expr_list)?);
	}
	return err_planning!("invalid state: needs aggregate");
}

fn needs_aggregate_expr_list(node: &Box<parser::ExprListNode>) -> Result<bool, Error> {
	for expr in node.exprs.iter() {
		if needs_aggregate_expr(expr)? {
			return Ok(true);
		}
	}
	return Ok(false);
}

fn needs_aggregate_expr(node: &Box<parser::ExprNode>) -> Result<bool, Error> {
	if let Some(ass_expr) = &node.ass_expr {
		return Ok(needs_aggregate_ass_expr(ass_expr)?);
	}
	return err_planning!("invalid state: needs aggregate expr");
}

fn needs_aggregate_ass_expr(node: &Box<parser::AssExprNode>) -> Result<bool, Error> {
	if let Some(func_expr) = &node.left_func_expr {
		return Ok(needs_aggregate_func_expr(func_expr)?);
	}
	if let Some(func_expr) = &node.right_func_expr {
		return Ok(needs_aggregate_func_expr(func_expr)?);
	}
	return err_planning!("invalid state: needs aggregate ass expr");
}

fn needs_aggregate_func_expr(node: &Box<parser::FuncExprNode>) -> Result<bool, Error> {
	if let Some(ident) = &node.ident {
		let func_name = unwrap_ident_object(ident)?.to_string().to_lowercase();
		match func_name.as_str() {
			"count" => return Ok(true),
			&_ => return Ok(false),
		}
	}
	return Ok(false);
}

pub fn plan_get_stmt(node: &Box<parser::GetStmtNode>, plan: &mut PlanNode) -> Result<(), Error> {
	let mut filter = FilterNode::new();
	let mut csv_file_scan = CsvFileScanNode::new();
	let mut sort = SortNode::new();

	if needs_aggregate(node)? {
		let mut aggregate = AggregateNode::new();
		aggregate.all = node.all;
		if let Some(expr_list) = &node.expr_list {
			aggregate.expr_list = Some(expr_list.clone());
		}		
		if let Some(table) = &node.table {
			csv_file_scan.table_name = unwrap_ident_object(&table)?.to_string();
			csv_file_scan.all = node.all;
		}
		if let Some(where_clause) = &node.where_clause {
			filter.where_clause = Some((*where_clause).clone());
		}
		if let Some(limit) = &node.limit {
			aggregate.limit = Some(limit.clone());
		}

		filter.csv_file_scan = Some(Box::new(csv_file_scan));
		aggregate.filter = Some(Box::new(filter));
		aggregate.all = node.all;
		plan.aggregate = Some(Box::new(aggregate));		

	} else {
		let mut project = ProjectNode::new();
		project.method = TokenKind::Get;

		if let Some(expr_list) = &node.expr_list {
			project.expr_list = Some(expr_list.clone());
		}

		if let Some(table) = &node.table {
			csv_file_scan.table_name = unwrap_ident_object(&table)?.to_string();
			csv_file_scan.all = node.all;
		}

		if let Some(where_clause) = &node.where_clause {
			filter.where_clause = Some((*where_clause).clone());
		}

		if let Some(limit) = &node.limit {
			project.limit = Some(limit.clone());
		}

		if let Some(order_by) = &node.order_by {
			if let Some(expr) = &order_by.expr {
				filter.csv_file_scan = Some(Box::new(csv_file_scan));
				project.filter = Some(Box::new(filter));
				project.all = true; // always true
				sort.expr = Some(expr.clone());	
				sort.project = Some(Box::new(project));
				sort.is_asc = order_by.is_asc;
				sort.all = node.all;
				plan.sort = Some(Box::new(sort));
			}
		} else {
			filter.csv_file_scan = Some(Box::new(csv_file_scan));
			project.filter = Some(Box::new(filter));
			project.all = node.all;
			plan.project = Some(Box::new(project));		
		}
	}

	Ok(())
}

fn unwrap_ident_object(node: &Box<parser::IdentNode>) -> Result<Object, Error> {
	Ok(Object::from_ident(&node.value))
}

