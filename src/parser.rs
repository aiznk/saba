use crate::tokenizer::{Token, TokenKind, TokenStream, tokenize};
use crate::error::{Error, make_error, err_parse};

#[derive(Debug, Clone)]
pub struct QueryNode {
	pub stmts: Vec<Box<StmtNode>>,
}

impl QueryNode {
	pub fn new() -> Self {
		Self {
			stmts: vec![],
		}
	}
}

#[derive(Debug, Clone)]
pub struct StmtNode {
	pub desc_stmt: Option<Box<DescStmtNode>>,
	pub alter_stmt: Option<Box<AlterStmtNode>>,
	pub drop_stmt: Option<Box<DropStmtNode>>,
	pub show_stmt: Option<Box<ShowStmtNode>>,
	pub use_stmt: Option<Box<UseStmtNode>>,
	pub create_stmt: Option<Box<CreateStmtNode>>,
	pub get_stmt: Option<Box<GetStmtNode>>,
	pub set_stmt: Option<Box<SetStmtNode>>,
	pub add_stmt: Option<Box<AddStmtNode>>,
	pub del_stmt: Option<Box<DelStmtNode>>,
}

impl StmtNode {
	pub fn new() -> Self {
		Self {
			desc_stmt: None,
			alter_stmt: None,
			drop_stmt: None,
			show_stmt: None,
			use_stmt: None,
			create_stmt: None,
			get_stmt: None,
			set_stmt: None,
			add_stmt: None,
			del_stmt: None,
		}
	}
}

#[derive(Debug, Clone)]
pub struct LimitNode {
	pub expr: Option<Box<ExprNode>>,
}

impl LimitNode {
	pub fn new() -> Self {
		Self {
			expr: None,
		}
	}
}

#[derive(Debug, Clone)]
pub struct DescStmtNode {
	pub table_name: Option<Box<IdentNode>>,
}

impl DescStmtNode {
	pub fn new() -> Self {
		Self {
			table_name: None,
		}
	}
}

#[derive(Debug, Clone)]
pub struct AlterAddColumnNode {
	pub ident: Option<Box<IdentNode>>,
	pub column_types: Vec<ColumnTypeNode>,
}

impl AlterAddColumnNode {
	pub fn new() -> Self {
		Self {
			ident: None,
			column_types: vec![],
		}
	}
}

#[derive(Debug, Clone)]
pub struct AlterDropColumnNode {
	pub ident: Option<Box<IdentNode>>,
}

impl AlterDropColumnNode {
	pub fn new() -> Self {
		Self {
			ident: None,
		}
	}
}

#[derive(Debug, Clone)]
pub struct AlterTableNode {
	pub table_name: Option<Box<IdentNode>>,
	pub alter_add_column: Option<Box<AlterAddColumnNode>>,
	pub alter_drop_column: Option<Box<AlterDropColumnNode>>,
}

impl AlterTableNode {
	pub fn new() -> Self {
		Self {
			table_name: None,
			alter_add_column: None,
			alter_drop_column: None,
		}
	}
}

#[derive(Debug, Clone)]
pub struct AlterStmtNode {
	pub alter_table: Option<Box<AlterTableNode>>,
}

impl AlterStmtNode {
	pub fn new() -> Self {
		Self {
			alter_table: None,
		}
	}
}

#[derive(Debug, Clone)]
pub struct DropStmtNode {
	pub table_name: Option<Box<IdentNode>>,
	pub db_name: Option<Box<IdentNode>>,
	pub if_exists: bool,
}

impl DropStmtNode {
	pub fn new() -> Self {
		Self {
			table_name: None,
			db_name: None,
			if_exists: false,
		}
	}
}

#[derive(Debug, Clone)]
pub struct ShowStmtNode {
	pub tables: bool,
	pub databases: bool,
}

impl ShowStmtNode {
	pub fn new() -> Self {
		Self {
			tables: false,
			databases: false,
		}
	}
}

#[derive(Debug, Clone)]
pub struct UseStmtNode {
	pub db_name: Option<Box<IdentNode>>,
}

impl UseStmtNode {
	pub fn new() -> Self {
		Self {
			db_name: None,
		}
	}
}

#[derive(Debug, Clone)]
pub struct CreateStmtNode {
	pub create_database: Option<Box<CreateDatabaseNode>>,
	pub create_table: Option<Box<CreateTableNode>>,
}

impl CreateStmtNode {
	pub fn new() -> Self {
		Self {
			create_database: None,
			create_table: None,
		}
	}
}

#[derive(Debug, Clone)]
pub struct CreateDatabaseNode {
	pub ident: Option<Box<IdentNode>>,
}

impl CreateDatabaseNode {
	pub fn new() -> Self {
		Self {
			ident: None,
		}
	}
}

#[derive(Debug, Clone)]
pub struct CreateTableNode {
	pub ident: Option<Box<IdentNode>>,
	pub column_definitions: Vec<Box<ColumnDefinitionNode>>,
}

impl CreateTableNode {
	pub fn new() -> Self {
		Self {
			ident: None,
			column_definitions: vec![],
		}
	}
}

#[derive(Debug, Clone)]
pub struct ColumnDefinitionNode {
	pub ident: Option<Box<IdentNode>>,
	pub column_types: Vec<ColumnTypeNode>,
}

impl ColumnDefinitionNode {
	pub fn new() -> Self {
		Self {
			ident: None,
			column_types: vec![],
		}
	}
}

#[derive(Debug, Clone)]
pub struct ValueNode {
	pub i64_value: Option<Box<I64ValueNode>>,
	pub f64_value: Option<Box<F64ValueNode>>,
	pub bool_value: Option<Box<BoolValueNode>>,
	pub string: Option<Box<StringNode>>,
}

impl ValueNode {
	pub fn new() -> Self {
		Self {
			i64_value: None,
			f64_value: None,
			bool_value: None,
			string: None,
		}
	}
}

#[derive(Debug, Clone)]
pub enum ColumnTypeNode {
	PrimaryKey,
	AutoIncrement,
	I64,
	F64,
	Bool,
	Char(usize),
	Default(Box<ValueNode>),
}

#[derive(Debug, Clone)]
pub struct GetStmtNode {
	pub all: bool,
	pub expr_list: Option<Box<ExprListNode>>,
	pub table: Option<Box<IdentNode>>,
	pub where_clause: Option<Box<WhereClauseNode>>,
	pub limit: Option<Box<LimitNode>>,
}

impl GetStmtNode {
	pub fn new() -> Self {
		Self {
			all: false,
			expr_list: None,
			table: None,
			where_clause: None,
			limit: None,
		}
	}
}

#[derive(Debug, Clone)]
pub struct SetStmtNode {
	pub all: bool,
	pub expr_list: Option<Box<ExprListNode>>,
	pub table: Option<Box<IdentNode>>,
	pub where_clause: Option<Box<WhereClauseNode>>,
	pub limit: Option<Box<LimitNode>>,
}

impl SetStmtNode {
	pub fn new() -> Self {
		Self {
			all: false,
			expr_list: None,
			table: None,
			where_clause: None,
			limit: None,
		}
	}
}

#[derive(Debug, Clone)]
pub struct AddStmtNode {
	pub expr_list: Option<Box<ExprListNode>>,
	pub table: Option<Box<IdentNode>>,
}

impl AddStmtNode {
	pub fn new() -> Self {
		Self {
			expr_list: None,
			table: None,
		}
	}
}

#[derive(Debug, Clone)]
pub struct DelStmtNode {
	pub all: bool,
	pub table: Option<Box<IdentNode>>,
	pub where_clause: Option<Box<WhereClauseNode>>,
	pub limit: Option<Box<LimitNode>>,
}

impl DelStmtNode {
	pub fn new() -> Self {
		Self {
			all: false,
			table: None,
			where_clause: None,
			limit: None,
		}
	}
}

#[derive(Debug, Clone)]
pub struct WhereClauseNode {
	pub expr: Option<Box<ExprNode>>,
}

impl WhereClauseNode {
	pub fn new() -> Self {
		Self {
			expr: None,
		}
	}
}

#[derive(Debug, Clone)]
pub struct ExprListNode {
	pub exprs: Vec<Box<ExprNode>>,
}

impl ExprListNode {
	pub fn new() -> Self {
		Self {
			exprs: vec![],
		}
	}
}

#[derive(Debug, Clone)]
pub struct ExprNode {
	pub ass_expr: Option<Box<AssExprNode>>,
}

impl ExprNode {
	pub fn new() -> Self {
		Self {
			ass_expr: None,
		}
	}
}

#[derive(Debug, Clone)]
pub struct AssExprNode {
	pub left_or_expr: Option<Box<OrExprNode>>,
	pub right_or_expr: Option<Box<OrExprNode>>,
}

impl AssExprNode {
	pub fn new() -> Self {
		Self {
			left_or_expr: None,
			right_or_expr: None,
		}
	}
}

#[derive(Debug, Clone)]
pub struct OrExprNode {
	pub nodes: Vec<Box<AndExprNode>>,
}

impl OrExprNode {
	pub fn new() -> Self {
		Self {
			nodes: vec![],
		}
	}
}

#[derive(Debug, Clone)]
pub struct AndExprNode {
	pub nodes: Vec<Box<CompareExprNode>>,
}

impl AndExprNode {
	pub fn new() -> Self {
		Self {
			nodes: vec![],
		}
	}
}

#[derive(Debug, Clone)]
pub enum CompareExprItemNode {
	Left(Box<AddSubExprNode>),
	Op(CompareOpNode),
	Right(Box<AddSubExprNode>),
}

#[derive(Debug, Clone)]
pub struct CompareExprNode {
	pub nodes: Vec<CompareExprItemNode>,
}

impl CompareExprNode {
	pub fn new() -> Self {
		Self {
			nodes: vec![],
		}
	}
}

#[derive(Debug, Clone)]
pub enum CompareOpNode {
	Lt,
	LtEq,
	Gt,
	GtEq,
	Eq,
	NotEq,
}

#[derive(Debug, Clone)]
pub enum AddSubExprItemNode {
	Left(Box<MulDivExprNode>),
	Op(AddSubOpNode),
	Right(Box<MulDivExprNode>),
}

#[derive(Debug, Clone)]
pub struct AddSubExprNode {
	pub nodes: Vec<AddSubExprItemNode>,
}

impl AddSubExprNode {
	pub fn new() -> Self {
		Self {
			nodes: vec![],
		}
	}
}

#[derive(Debug, Clone)]
pub enum AddSubOpNode {
	Add, // +
	Sub, // -
}

#[derive(Debug, Clone)]
pub enum MulDivExprItemNode {
	Left(Box<OperandNode>),
	Op(MulDivOpNode),
	Right(Box<OperandNode>),
}

#[derive(Debug, Clone)]
pub struct MulDivExprNode {
	pub nodes: Vec<MulDivExprItemNode>,
}

impl MulDivExprNode {
	pub fn new() -> Self {
		Self {
			nodes: vec![],
		}
	}
}

#[derive(Debug, Clone)]
pub enum MulDivOpNode {
	Mul, // *
	Div, // /
	Mod, // %
}

#[derive(Debug, Clone)]
pub struct OperandNode {
	pub i64_value: Option<Box<I64ValueNode>>,
	pub f64_value: Option<Box<F64ValueNode>>,
	pub bool_value: Option<Box<BoolValueNode>>,
	pub string: Option<Box<StringNode>>,
	pub ident: Option<Box<IdentNode>>,
	pub expr: Option<Box<ExprNode>>,
	pub star: bool,
}

impl OperandNode {
	pub fn new() -> Self {
		Self {
			i64_value: None,
			f64_value: None,
			bool_value: None,
			string: None,
			ident: None,
			expr: None,
			star: false,
		}
	}
}

#[derive(Debug, Clone)]
pub struct I64ValueNode {
	pub value: i64,
}

impl I64ValueNode {
	pub fn new() -> Self {
		Self {
			value: 0,
		}
	}
}

#[derive(Debug, Clone)]
pub struct F64ValueNode {
	pub value: f64,
}

impl F64ValueNode {
	pub fn new() -> Self {
		Self {
			value: 0.0,
		}
	}
}

#[derive(Debug, Clone)]
pub struct BoolValueNode {
	pub value: bool,
}

impl BoolValueNode {
	pub fn new() -> Self {
		Self {
			value: false,
		}
	}
}

#[derive(Debug, Clone)]
pub struct StringNode {
	pub value: String,
}

impl StringNode {
	pub fn new() -> Self {
		Self {
			value: String::new(),
		}
	}
}

#[derive(Debug, Clone)]
pub struct IdentNode {
	pub value: String,
}

impl IdentNode {
	pub fn new() -> Self {
		Self {
			value: String::new(),
		}
	}
}

pub fn parse(tok_strm: &mut TokenStream) -> Result<QueryNode, Error> {
	let mut query = QueryNode::new();

	while !tok_strm.is_end() {
		let stmt: Option<Box<StmtNode>> = parse_stmt(tok_strm)?;
		if stmt.is_none() {
			break;
		}
		query.stmts.push(stmt.unwrap());

		if tok_strm.is_end() {
			break;
		}

		let tok = tok_strm.get()?;
		if tok.kind != TokenKind::Semicolon {
			return err_parse!("missing semicolon in stmt: {:?}", tok);
		}
	}

	Ok(query)
}

pub fn parse_stmt(tok_strm: &mut TokenStream) -> Result<Option<Box<StmtNode>>, Error> {
	let mut stmt = StmtNode::new();

	let desc_stmt = parse_desc_stmt(tok_strm)?;
	if desc_stmt.is_some() {
		stmt.desc_stmt = desc_stmt;
		return Ok(Some(Box::new(stmt)));
	}

	let alter_stmt = parse_alter_stmt(tok_strm)?;
	if alter_stmt.is_some() {
		stmt.alter_stmt = alter_stmt;
		return Ok(Some(Box::new(stmt)));
	}

	let drop_stmt = parse_drop_stmt(tok_strm)?;
	if drop_stmt.is_some() {
		stmt.drop_stmt = drop_stmt;
		return Ok(Some(Box::new(stmt)));
	}

	let show_stmt = parse_show_stmt(tok_strm)?;
	if show_stmt.is_some() {
		stmt.show_stmt = show_stmt;
		return Ok(Some(Box::new(stmt)));
	}

	let use_stmt = parse_use_stmt(tok_strm)?;
	if use_stmt.is_some() {
		stmt.use_stmt = use_stmt;
		return Ok(Some(Box::new(stmt)));
	}

	let create_stmt = parse_create_stmt(tok_strm)?;
	if create_stmt.is_some() {
		stmt.create_stmt = create_stmt;
		return Ok(Some(Box::new(stmt)));
	}

	let get_stmt = parse_get_stmt(tok_strm)?;
	if get_stmt.is_some() {
		stmt.get_stmt = get_stmt;
		return Ok(Some(Box::new(stmt)));
	}

	let set_stmt = parse_set_stmt(tok_strm)?;
	if set_stmt.is_some() {
		stmt.set_stmt = set_stmt;
		return Ok(Some(Box::new(stmt)));
	}

	let add_stmt = parse_add_stmt(tok_strm)?;
	if add_stmt.is_some() {
		stmt.add_stmt = add_stmt;
		return Ok(Some(Box::new(stmt)));
	}

	let del_stmt = parse_del_stmt(tok_strm)?;
	if del_stmt.is_some() {
		stmt.del_stmt = del_stmt;
		return Ok(Some(Box::new(stmt)));
	}

	return err_parse!("failed to parse stmt");
}

fn parse_if_exists(tok_strm: &mut TokenStream) -> Result<bool, Error> {
	let tok = tok_strm.get()?;
	if tok.kind == TokenKind::If {
		let tok = tok_strm.get()?;
		if tok.kind != TokenKind::Exists {
			return err_parse!("invalid syntax. missing 'exists'");
		}
		return Ok(true);
	} else {
		tok_strm.prev();
		return Ok(false);
	}
}

pub fn parse_desc_stmt(tok_strm: &mut TokenStream) -> Result<Option<Box<DescStmtNode>>, Error> {
	let mut n = DescStmtNode::new();

	let tok = tok_strm.get()?;
	if tok.kind != TokenKind::Desc {
		tok_strm.prev();
		return Ok(None);
	}	

	n.table_name = parse_ident(tok_strm)?;
	if n.table_name.is_some() {
		return Ok(Some(Box::new(n)));
	}

	Ok(None)
}

pub fn parse_alter_stmt(tok_strm: &mut TokenStream) -> Result<Option<Box<AlterStmtNode>>, Error> {
	let mut n = AlterStmtNode::new();

	n.alter_table = parse_alter_table(tok_strm)?;
	if n.alter_table.is_some() {
		return Ok(Some(Box::new(n)));
	}

	Ok(None)
}

pub fn parse_alter_table(tok_strm: &mut TokenStream) -> Result<Option<Box<AlterTableNode>>, Error> {
	let mut n = AlterTableNode::new();

	let tok = tok_strm.get()?;
	if tok.kind != TokenKind::Alter {
		tok_strm.prev();
		return Ok(None);
	}	

	let tok = tok_strm.get()?;
	if tok.kind != TokenKind::Table {
		tok_strm.prev();
		return Ok(None);
	}	

	n.table_name = parse_ident(tok_strm)?;
	if n.table_name.is_none() {
		return err_parse!("missing table name in alter table stmt");
	}

	n.alter_add_column = parse_alter_add_column(tok_strm)?;
	if n.alter_add_column.is_some() {
		return Ok(Some(Box::new(n)));
	}

	n.alter_drop_column = parse_alter_drop_column(tok_strm)?;
	if n.alter_drop_column.is_some() {
		return Ok(Some(Box::new(n)));
	}

	err_parse!("invalid state: alter table stmt")
}

pub fn parse_alter_drop_column(tok_strm: &mut TokenStream) -> Result<Option<Box<AlterDropColumnNode>>, Error> {
	let mut n = AlterDropColumnNode::new();

	let tok = tok_strm.get()?;
	if tok.kind != TokenKind::Drop {
		tok_strm.prev();
		return Ok(None);
	}		

	let tok = tok_strm.get()?;
	if tok.kind != TokenKind::Column {
		tok_strm.prev();
		return Ok(None);
	}

	n.ident = parse_ident(tok_strm)?;
	if n.ident.is_none() {
		return err_parse!("missing column name in drop column stmt");
	}

	Ok(Some(Box::new(n)))
}

pub fn parse_alter_add_column(tok_strm: &mut TokenStream) -> Result<Option<Box<AlterAddColumnNode>>, Error> {
	let mut n = AlterAddColumnNode::new();

	let tok = tok_strm.get()?;
	if tok.kind != TokenKind::Add {
		tok_strm.prev();
		return Ok(None);
	}		

	let tok = tok_strm.get()?;
	if tok.kind != TokenKind::Column {
		tok_strm.prev();
		return Ok(None);
	}		

	n.ident = parse_ident(tok_strm)?;
	if n.ident.is_none() {
		return err_parse!("missing column ident in alter add column stmt");
	}

	while !tok_strm.is_end() {
		let column_type = parse_column_type(tok_strm)?;
		if column_type.is_none() {
			break;
		}
		n.column_types.push(column_type.unwrap());
	}

	if n.column_types.len() == 0 {
		return err_parse!("missing column types in alter add column stmt");
	}

	Ok(Some(Box::new(n)))
}

pub fn parse_drop_stmt(tok_strm: &mut TokenStream) -> Result<Option<Box<DropStmtNode>>, Error> {
	let mut n = DropStmtNode::new();

	let tok = tok_strm.get()?;
	if tok.kind != TokenKind::Drop {
		tok_strm.prev();
		return Ok(None);
	}	

	let tok = tok_strm.get()?;
	if tok.kind == TokenKind::Table {
		n.if_exists = parse_if_exists(tok_strm)?;
		n.table_name = parse_ident(tok_strm)?;
	} else if tok.kind == TokenKind::Database {
		n.if_exists = parse_if_exists(tok_strm)?;
		n.db_name = parse_ident(tok_strm)?;	
	} else {
		return err_parse!("invalid state: drop stmt");
	}

	Ok(Some(Box::new(n)))
}

pub fn parse_show_stmt(tok_strm: &mut TokenStream) -> Result<Option<Box<ShowStmtNode>>, Error> {
	let mut n = ShowStmtNode::new();

	let tok = tok_strm.get()?;
	if tok.kind != TokenKind::Show {
		tok_strm.prev();
		return Ok(None);
	}

	let tok = tok_strm.get()?;
	if tok.kind == TokenKind::Tables {
		n.tables = true;
	} else if tok.kind == TokenKind::Databases {
		n.databases = true;	
	} else {
		return err_parse!("invalid state: show stmt");
	}

	Ok(Some(Box::new(n)))
}

pub fn parse_use_stmt(tok_strm: &mut TokenStream) -> Result<Option<Box<UseStmtNode>>, Error> {
	let mut n = UseStmtNode::new();

	let tok = tok_strm.get()?;
	if tok.kind != TokenKind::Use {
		tok_strm.prev();
		return Ok(None);
	}

	n.db_name = parse_ident(tok_strm)?;
	if n.db_name.is_none() {
		return err_parse!("missing database name");
	}

	Ok(Some(Box::new(n)))
}

pub fn parse_create_stmt(tok_strm: &mut TokenStream) -> Result<Option<Box<CreateStmtNode>>, Error> {
	let mut n = CreateStmtNode::new();

	n.create_database = parse_create_database(tok_strm)?;
	if !n.create_database.is_none() {
		return Ok(Some(Box::new(n)));
	}

	n.create_table = parse_create_table(tok_strm)?;
	if !n.create_table.is_none() {
		return Ok(Some(Box::new(n)));
	}

	Ok(None)
}

pub fn parse_create_database(tok_strm: &mut TokenStream) -> Result<Option<Box<CreateDatabaseNode>>, Error> {
	let mut n = CreateDatabaseNode::new();

	if tok_strm.is_end() {
		return Ok(None);
	}

	let i = tok_strm.index;

	let tok = tok_strm.get()?;
	if tok.kind != TokenKind::Create {
		tok_strm.index = i;
		return Ok(None);
	}

	let tok = tok_strm.get()?;
	if tok.kind != TokenKind::Database {
		tok_strm.index = i;
		return Ok(None);
	}

	n.ident = parse_ident(tok_strm)?;
	if n.ident.is_none() {
		return err_parse!("missing database name");
	}

	Ok(Some(Box::new(n)))
}

pub fn parse_create_table(tok_strm: &mut TokenStream) -> Result<Option<Box<CreateTableNode>>, Error> {
	let mut n = CreateTableNode::new();

	if tok_strm.is_end() {
		return Ok(None);
	}

	let i = tok_strm.index;

	let tok = tok_strm.get()?;
	if tok.kind != TokenKind::Create {
		tok_strm.index = i;
		return Ok(None);
	}

	let tok = tok_strm.get()?;
	if tok.kind != TokenKind::Table {
		tok_strm.index = i;
		return Ok(None);
	}

	n.ident = parse_ident(tok_strm)?;
	if n.ident.is_none() {
		return err_parse!("missing table name in create table");
	}

	let tok = tok_strm.get()?;
	if tok.kind != TokenKind::LParen {
		return err_parse!("missing ( in create table");
	}

	let column_definition = parse_column_definition(tok_strm)?;
	if column_definition.is_none() {
		return err_parse!("missing column definition in create table");
	}
	n.column_definitions.push(column_definition.unwrap());

	while !tok_strm.is_end() {
		let tok = tok_strm.get()?;
		if tok.kind != TokenKind::Comma {
			tok_strm.prev();
			break;
		}

		if tok_strm.is_end() {
			break;
		}

		let column_definition = parse_column_definition(tok_strm)?;
		if column_definition.is_none() {
			break;
		}
		n.column_definitions.push(column_definition.unwrap());
	}

	let tok = tok_strm.get()?;
	if tok.kind != TokenKind::RParen {
		return err_parse!("missing ) in create table");
	}

	Ok(Some(Box::new(n)))
}

fn check_first_column_type(column_type: &Option<ColumnTypeNode>) -> Result<(), Error> {
	if let Some(column_type) = column_type {
		match column_type {
			ColumnTypeNode::I64 |
			ColumnTypeNode::F64 |
			ColumnTypeNode::Bool |
			ColumnTypeNode::Char(_) => {
				// pass
			}
			_ => return err_parse!("invalid first column type"),
		}
	}
	Ok(())
}

pub fn parse_column_definition(tok_strm: &mut TokenStream) -> Result<Option<Box<ColumnDefinitionNode>>, Error> {
	let mut n = ColumnDefinitionNode::new();

	if tok_strm.is_end() {
		return Ok(None);
	}

	let i = tok_strm.index;

	n.ident = parse_ident(tok_strm)?;
	if n.ident.is_none() {
		tok_strm.index = i;
		return Ok(None);
	}

	let tok = tok_strm.get()?;
	if tok.kind != TokenKind::Colon {
		tok_strm.index = i;
		return Ok(None);
	}

	let column_type = parse_column_type(tok_strm)?;
	if column_type.is_none() {
		return err_parse!("missing column type in column definition");
	}
	check_first_column_type(&column_type)?;
	n.column_types.push(column_type.unwrap());

	while !tok_strm.is_end() {
		let column_type = parse_column_type(tok_strm)?;
		if column_type.is_none() {
			break;
		}
		n.column_types.push(column_type.unwrap());
	}

	Ok(Some(Box::new(n)))
}

pub fn parse_column_type(tok_strm: &mut TokenStream) -> Result<Option<ColumnTypeNode>, Error> {
	if tok_strm.is_end() {
		return Ok(None);
	}

	let tok = tok_strm.get()?;
	if tok.kind == TokenKind::PrimaryKey {
		return Ok(Some(ColumnTypeNode::PrimaryKey));		
	} else if tok.kind == TokenKind::AutoIncrement {
		return Ok(Some(ColumnTypeNode::AutoIncrement));		
	} else if tok.kind == TokenKind::TypeI64 {
		return Ok(Some(ColumnTypeNode::I64));		
	} else if tok.kind == TokenKind::TypeF64 {
		return Ok(Some(ColumnTypeNode::F64));		
	} else if tok.kind == TokenKind::Bool {
		return Ok(Some(ColumnTypeNode::Bool));		
	} else if tok.kind == TokenKind::Char {
		let tok = tok_strm.get()?;
		if tok.kind != TokenKind::LBracket {
			return err_parse!("missing [ in char of column type");
		}

		let i64_value = parse_i64_value(tok_strm)?;
		if i64_value.is_none() {
			return err_parse!("missing number of char in column type");
		}

		let tok = tok_strm.get()?;
		if tok.kind != TokenKind::RBracket {
			return err_parse!("missing ] in char of column type");
		}		

		let value = i64_value.unwrap().value as usize;
		return Ok(Some(ColumnTypeNode::Char(value)))
	} else if tok.kind == TokenKind::Default {
		let value = parse_value(tok_strm)?;
		if value.is_none() {
			return err_parse!("missing value of default attribute");
		}

		return Ok(Some(ColumnTypeNode::Default(value.unwrap())));
	}

	tok_strm.prev();
	Ok(None)
}

pub fn parse_value(tok_strm: &mut TokenStream) -> Result<Option<Box<ValueNode>>, Error> {
	let mut n = ValueNode::new();

	n.i64_value = parse_i64_value(tok_strm)?;
	if n.i64_value.is_some() {
		return Ok(Some(Box::new(n)));
	}

	n.f64_value = parse_f64_value(tok_strm)?;
	if n.f64_value.is_some() {
		return Ok(Some(Box::new(n)));
	}

	n.bool_value = parse_bool_value(tok_strm)?;
	if n.bool_value.is_some() {
		return Ok(Some(Box::new(n)));
	}

	n.string = parse_string(tok_strm)?;
	if n.string.is_some() {
		return Ok(Some(Box::new(n)));
	}

	Ok(None)
}

pub fn parse_get_stmt(tok_strm: &mut TokenStream) -> Result<Option<Box<GetStmtNode>>, Error> {
	let mut n = GetStmtNode::new();

	if tok_strm.is_end() {
		return Ok(None);
	}

	let tok = tok_strm.get()?;
	if tok.kind != TokenKind::Get {
		tok_strm.prev();
		return Ok(None);
	}

	let tok = tok_strm.get()?;
	if tok.kind == TokenKind::All {
		n.all = true;
	} else {
		tok_strm.prev();
	}

	n.expr_list = parse_expr_list(tok_strm)?;
	if n.expr_list.is_none() {
		return err_parse!("failed to parse expr list in get stmt");
	}

	let tok = tok_strm.get()?;
	if tok.kind != TokenKind::Of {
		return err_parse!("missing 'OF' in get stmt");
	}

	n.table = parse_ident(tok_strm)?;
	if n.table.is_none() {
		return err_parse!("missing table name in get stmt");
	}

	n.where_clause = parse_where_clause(tok_strm)?;
	n.limit = parse_limit(tok_strm)?;

	Ok(Some(Box::new(n)))
}

pub fn parse_set_stmt(tok_strm: &mut TokenStream) -> Result<Option<Box<SetStmtNode>>, Error> {
	let mut n = SetStmtNode::new();

	if tok_strm.is_end() {
		return Ok(None);
	}

	let tok = tok_strm.get()?;
	if tok.kind != TokenKind::Set {
		tok_strm.prev();
		return Ok(None);
	}

	let tok = tok_strm.get()?;
	if tok.kind == TokenKind::All {
		n.all = true;
	} else {
		tok_strm.prev();
	}

	let expr_list = parse_expr_list(tok_strm)?;
	if expr_list.is_none() {
		return err_parse!("failed to parse expr list in set stmt");
	}

	n.expr_list = expr_list;

	let tok = tok_strm.get()?;
	if tok.kind != TokenKind::Of {
		return err_parse!("missing 'OF' in set stmt");
	}

	n.table = parse_ident(tok_strm)?;
	if n.table.is_none() {
		return err_parse!("missing table name in set stmt");
	}

	n.where_clause = parse_where_clause(tok_strm)?;
	n.limit = parse_limit(tok_strm)?;

	Ok(Some(Box::new(n)))
}

pub fn parse_add_stmt(tok_strm: &mut TokenStream) -> Result<Option<Box<AddStmtNode>>, Error> {
	let mut n = AddStmtNode::new();

	if tok_strm.is_end() {
		return Ok(None);
	}

	let tok = tok_strm.get()?;
	if tok.kind != TokenKind::Add {
		tok_strm.prev();
		return Ok(None);
	}

	n.expr_list = parse_expr_list(tok_strm)?;
	if n.expr_list.is_none() {
		return err_parse!("failed to parse expr list in add stmt");
	}

	let tok = tok_strm.get()?;
	if tok.kind != TokenKind::Of {
		return err_parse!("missing 'OF' in add stmt: {:?}", tok);
	}

	n.table = parse_ident(tok_strm)?;
	if n.table.is_none() {
		return err_parse!("missing table name in add stmt");
	}

	Ok(Some(Box::new(n)))
}

pub fn parse_del_stmt(tok_strm: &mut TokenStream) -> Result<Option<Box<DelStmtNode>>, Error> {
	let mut n = DelStmtNode::new();

	if tok_strm.is_end() {
		return Ok(None);
	}

	let tok = tok_strm.get()?;
	if tok.kind != TokenKind::Del {
		tok_strm.prev();
		return Ok(None);
	}

	let tok = tok_strm.get()?;
	if tok.kind == TokenKind::All {
		n.all = true;
	} else {
		tok_strm.prev();
	}

	let tok = tok_strm.get()?;
	if tok.kind != TokenKind::Of {
		return err_parse!("missing 'OF' in get stmt");
	}

	n.table = parse_ident(tok_strm)?;
	if n.table.is_none() {
		return err_parse!("missing table name in del stmt");
	}

	n.where_clause = parse_where_clause(tok_strm)?;
	n.limit = parse_limit(tok_strm)?;

	Ok(Some(Box::new(n)))
}

pub fn parse_limit(tok_strm: &mut TokenStream) -> Result<Option<Box<LimitNode>>, Error> {
	if tok_strm.is_end() {
		return Ok(None);
	}

	let tok = tok_strm.get()?;
	if tok.kind != TokenKind::Limit {
		tok_strm.prev();
		return Ok(None);
	}

	let mut n = LimitNode::new();

	n.expr = parse_expr(tok_strm)?;
	if n.expr.is_none() {
		return err_parse!("missing expr in limit");
	}

	Ok(Some(Box::new(n)))
}

pub fn parse_where_clause(tok_strm: &mut TokenStream) -> Result<Option<Box<WhereClauseNode>>, Error> {
	let mut n = WhereClauseNode::new();

	if tok_strm.is_end() {
		return Ok(None);
	}

	let tok = tok_strm.get()?;
	if tok.kind != TokenKind::Where {
		tok_strm.prev();
		return Ok(None);
	}

	n.expr = parse_expr(tok_strm)?;

	Ok(Some(Box::new(n)))
}

pub fn parse_expr_list(tok_strm: &mut TokenStream) -> Result<Option<Box<ExprListNode>>, Error> {
	let mut n = ExprListNode::new();

	if tok_strm.is_end() {
		return Ok(None);
	}

	let expr = parse_expr(tok_strm)?;
	if expr.is_none() {
		tok_strm.prev();
		return Ok(None);
	}
	n.exprs.push(expr.unwrap());

	while !tok_strm.is_end() {
		let tok = tok_strm.get()?;
		if tok.kind != TokenKind::Comma {
			tok_strm.prev();
			break;
		}

		let expr = parse_expr(tok_strm)?;
		if expr.is_none() {
			break;
		}
		n.exprs.push(expr.unwrap());
	}

	Ok(Some(Box::new(n)))
}

pub fn parse_expr(tok_strm: &mut TokenStream) -> Result<Option<Box<ExprNode>>, Error> {
	let mut n = ExprNode::new();

	if tok_strm.is_end() {
		return Ok(None);
	}

	n.ass_expr = parse_ass_expr(tok_strm)?;
	if n.ass_expr.is_none() {
		return Ok(None);
	}

	Ok(Some(Box::new(n)))
}

pub fn parse_ass_expr(tok_strm: &mut TokenStream) -> Result<Option<Box<AssExprNode>>, Error> {
	let mut n = AssExprNode::new();

	if tok_strm.is_end() {
		return Ok(None);
	}

	n.left_or_expr = parse_or_expr(tok_strm)?;
	if n.left_or_expr.is_none() {
		return Ok(None);
	}

	if tok_strm.is_end() {
		return Ok(Some(Box::new(n)));
	}

	let tok = tok_strm.get()?;
	if tok.kind != TokenKind::Assign {
		tok_strm.prev();
		return Ok(Some(Box::new(n)));
	}

	n.right_or_expr = parse_or_expr(tok_strm)?;
	if n.right_or_expr.is_none() {
		return err_parse!("missing right hand operand in ass expr");
	}

	Ok(Some(Box::new(n)))
}

pub fn parse_or_expr(tok_strm: &mut TokenStream) -> Result<Option<Box<OrExprNode>>, Error> {
	let mut n = OrExprNode::new();

	if tok_strm.is_end() {
		return Ok(None);
	}

	let and_expr = parse_and_expr(tok_strm)?;
	if and_expr.is_none() {
		return Ok(None);
	}
	n.nodes.push(and_expr.unwrap());

	while !tok_strm.is_end() {
		let tok = tok_strm.get()?;
		if tok.kind != TokenKind::Or {
			tok_strm.prev();
			break;
		}

		let and_expr = parse_and_expr(tok_strm)?;
		if and_expr.is_none() {
			return err_parse!("failed to parse and expr in or expr");
		}
		n.nodes.push(and_expr.unwrap());
	}

	Ok(Some(Box::new(n)))
}

pub fn parse_and_expr(tok_strm: &mut TokenStream) -> Result<Option<Box<AndExprNode>>, Error> {
	let mut n = AndExprNode::new();

	if tok_strm.is_end() {
		return Ok(None);
	}

	let compare_expr = parse_compare_expr(tok_strm)?;
	if compare_expr.is_none() {
		return Ok(None);
	}
	n.nodes.push(compare_expr.unwrap());

	while !tok_strm.is_end() {
		let tok = tok_strm.get()?;
		if tok.kind != TokenKind::And {
			tok_strm.prev();
			break;
		}

		let compare_expr = parse_compare_expr(tok_strm)?;
		if compare_expr.is_none() {
			return err_parse!("failed to parse and expr in or expr");
		}
		n.nodes.push(compare_expr.unwrap());
	}

	Ok(Some(Box::new(n)))
}

pub fn parse_compare_expr(tok_strm: &mut TokenStream) -> Result<Option<Box<CompareExprNode>>, Error> {
	let mut n = CompareExprNode::new();

	if tok_strm.is_end() {
		return Ok(None);
	}

	let left = parse_add_sub_expr(tok_strm)?;
	if left.is_none() {
		return Ok(None);
	}

	let node = CompareExprItemNode::Left(left.unwrap());
	n.nodes.push(node);

	while !tok_strm.is_end() {
		let op = parse_compare_op(tok_strm)?;
		if op.is_none() {
			break;
		}

		let node = CompareExprItemNode::Op(*op.unwrap());
		n.nodes.push(node);

		let right = parse_add_sub_expr(tok_strm)?;
		if right.is_none() {
			return err_parse!("missing right logic expr in compare expr");
		}		

		let node = CompareExprItemNode::Right(right.unwrap());
		n.nodes.push(node);
	}

	Ok(Some(Box::new(n)))
}

pub fn parse_compare_op(tok_strm: &mut TokenStream) -> Result<Option<Box<CompareOpNode>>, Error> {
	if tok_strm.is_end() {
		return Ok(None);
	}

	let tok = tok_strm.get()?;
	if tok.kind == TokenKind::Eq {
		return Ok(Some(Box::new(CompareOpNode::Eq)));
	} else if tok.kind == TokenKind::NotEq {
		return Ok(Some(Box::new(CompareOpNode::NotEq)));
	} else if tok.kind == TokenKind::Lt {
		return Ok(Some(Box::new(CompareOpNode::Lt)));
	} else if tok.kind == TokenKind::LtEq {
		return Ok(Some(Box::new(CompareOpNode::LtEq)));
	} else if tok.kind == TokenKind::Gt {
		return Ok(Some(Box::new(CompareOpNode::Gt)));
	} else if tok.kind == TokenKind::GtEq {
		return Ok(Some(Box::new(CompareOpNode::GtEq)));
	} else {
		tok_strm.prev();
		return Ok(None);
	}
}

pub fn parse_add_sub_expr(tok_strm: &mut TokenStream) -> Result<Option<Box<AddSubExprNode>>, Error> {
	let mut n = AddSubExprNode::new();

	let left = parse_mul_div_expr(tok_strm)?;
	if left.is_none() {
		return Ok(None);
	}
	n.nodes.push(AddSubExprItemNode::Left(left.unwrap()));

	while !tok_strm.is_end() {
		let op = parse_add_sub_op(tok_strm)?;
		if op.is_none() {
			break;
		}
		n.nodes.push(AddSubExprItemNode::Op(op.unwrap()));

		let right = parse_mul_div_expr(tok_strm)?;
		if right.is_none() {
			return err_parse!("missing mul div expr in add sub expr");
		}
		n.nodes.push(AddSubExprItemNode::Right(right.unwrap()));
	}

	Ok(Some(Box::new(n)))
}

pub fn parse_add_sub_op(tok_strm: &mut TokenStream) -> Result<Option<AddSubOpNode>, Error> {
	let tok = tok_strm.get()?;

	if tok.kind == TokenKind::AddOp {
		return Ok(Some(AddSubOpNode::Add));
	} else if tok.kind == TokenKind::SubOp {
		return Ok(Some(AddSubOpNode::Sub));
	} else {
		tok_strm.prev();
		return Ok(None);
	}
}

pub fn parse_mul_div_expr(tok_strm: &mut TokenStream) -> Result<Option<Box<MulDivExprNode>>, Error> {
	let mut n = MulDivExprNode::new();

	let left = parse_operand(tok_strm)?;
	if left.is_none() {
		return Ok(None);
	}
	n.nodes.push(MulDivExprItemNode::Left(left.unwrap()));

	while !tok_strm.is_end() {
		let op = parse_mul_div_op(tok_strm)?;
		if op.is_none() {
			break;
		}
		n.nodes.push(MulDivExprItemNode::Op(op.unwrap()));

		let right = parse_operand(tok_strm)?;
		if right.is_none() {
			return err_parse!("missing mul div expr in add sub expr");
		}
		n.nodes.push(MulDivExprItemNode::Right(right.unwrap()));
	}

	Ok(Some(Box::new(n)))
}

pub fn parse_mul_div_op(tok_strm: &mut TokenStream) -> Result<Option<MulDivOpNode>, Error> {
	let tok = tok_strm.get()?;

	if tok.kind == TokenKind::MulOp {
		return Ok(Some(MulDivOpNode::Mul));
	} else if tok.kind == TokenKind::DivOp {
		return Ok(Some(MulDivOpNode::Div));
	} else if tok.kind == TokenKind::ModOp {
		return Ok(Some(MulDivOpNode::Mod));
	} else {
		tok_strm.prev();
		return Ok(None);
	}
}

pub fn parse_operand(tok_strm: &mut TokenStream) -> Result<Option<Box<OperandNode>>, Error> {
	let mut n = OperandNode::new();

	if tok_strm.is_end() {
		return Ok(None);
	}

	let tok = tok_strm.get()?;
	if tok.kind == TokenKind::LParen {
		n.expr = parse_expr(tok_strm)?;
		if n.expr.is_none() {
			return err_parse!("missing expression in operand");
		}

		let tok = tok_strm.get()?;
		if tok.kind != TokenKind::RParen {
			return err_parse!("missing ) in operand");
		}

		return Ok(Some(Box::new(n)));
	} else {
		tok_strm.prev();

		n.i64_value = parse_i64_value(tok_strm)?;
		if n.i64_value.is_some() {
			return Ok(Some(Box::new(n)));
		}

		n.f64_value = parse_f64_value(tok_strm)?;
		if n.f64_value.is_some() {
			return Ok(Some(Box::new(n)));
		}

		n.bool_value = parse_bool_value(tok_strm)?;
		if n.bool_value.is_some() {
			return Ok(Some(Box::new(n)));
		}

		n.string = parse_string(tok_strm)?;
		if n.string.is_some() {
			return Ok(Some(Box::new(n)));
		}

		n.ident = parse_ident(tok_strm)?;
		if n.ident.is_some() {
			return Ok(Some(Box::new(n)));
		}

		let tok = tok_strm.get()?;
		if tok.kind == TokenKind::MulOp {
			n.star = true;
			return Ok(Some(Box::new(n)));
		}
		tok_strm.prev();

		return Ok(None);
	}
}

pub fn parse_bool_value(tok_strm: &mut TokenStream) -> Result<Option<Box<BoolValueNode>>, Error> {
	let mut n = BoolValueNode::new();

	if tok_strm.is_end() {
		return Ok(None);
	}

	let tok = tok_strm.get()?;
	if tok.kind == TokenKind::True {
		n.value = true;
	} else if tok.kind == TokenKind::False {
		n.value = false;
	} else {
		tok_strm.prev();
		return Ok(None);
	}

	Ok(Some(Box::new(n)))
}

pub fn parse_i64_value(tok_strm: &mut TokenStream) -> Result<Option<Box<I64ValueNode>>, Error> {
	let mut n = I64ValueNode::new();

	if tok_strm.is_end() {
		return Ok(None);
	}

	let tok = tok_strm.get()?;
	if tok.kind != TokenKind::Int {
		tok_strm.prev();
		return Ok(None);
	}

	n.value = tok.i64_value.unwrap();

	Ok(Some(Box::new(n)))
}

pub fn parse_f64_value(tok_strm: &mut TokenStream) -> Result<Option<Box<F64ValueNode>>, Error> {
	let mut n = F64ValueNode::new();

	if tok_strm.is_end() {
		return Ok(None);
	}

	let tok = tok_strm.get()?;
	if tok.kind != TokenKind::Float {
		tok_strm.prev();
		return Ok(None);
	}

	n.value = tok.f64_value.unwrap();

	Ok(Some(Box::new(n)))
}

pub fn parse_string(tok_strm: &mut TokenStream) -> Result<Option<Box<StringNode>>, Error> {
	let mut n = StringNode::new();

	if tok_strm.is_end() {
		return Ok(None);
	}

	let tok = tok_strm.get()?;
	if tok.kind != TokenKind::String {
		tok_strm.prev();
		return Ok(None);
	}

	n.value = tok.text.clone().unwrap();

	Ok(Some(Box::new(n)))
}

pub fn parse_ident(tok_strm: &mut TokenStream) -> Result<Option<Box<IdentNode>>, Error> {
	let mut n = IdentNode::new();

	if tok_strm.is_end() {
		return Ok(None);
	}

	let tok = tok_strm.get()?;
	if tok.kind != TokenKind::Ident {
		tok_strm.prev();
		return Ok(None);
	}

	n.value = tok.text.clone().unwrap();

	Ok(Some(Box::new(n)))
}

#[cfg(test)]
mod tests {
	use super::*;

	fn do_parse(s: &str) -> bool {
		let tokens: Vec<Token> = match tokenize(s.to_string()) {
			Ok(v) => v,
			Err(e) => {
				eprintln!("{}", e);
				return false;
			}
		};
		let mut strm = TokenStream::new(tokens);
		match parse(&mut strm) {
			Ok(v) => v,
			Err(e) => {
				eprintln!("{}", e);
				return false;
			}
		};

		true
	}

	#[test]
	fn test_use_stmt() {
		assert!(do_parse("USE mydb;") == true);
	}

	#[test]
	fn test_create_database_stmt() {
		assert!(do_parse("CREATE DATABASE mydb;") == true);
	}

	#[test]
	fn test_create_table_stmt_0() {
		assert!(do_parse("CREATE TABLE mytab (id: I64);") == true);
	}

	#[test]
	fn test_create_table_stmt_0a() {
		assert!(do_parse("
create table mytab (id: i64 auto_increment)
") == true);
	}

	#[test]
	fn test_create_table_stmt_1() {
		assert!(do_parse("
CREATE TABLE mytab (
	id: I64 PRIMARY_KEY AUTO_INCREMENT,
	weight: F64,
	name: CHAR[128],
);
") == true);
	}

	#[test]
	fn test_get_stmt_0() {
		assert!(do_parse("GET id OF mytab WHERE age == 123") == true);
	}

	#[test]
	fn test_get_stmt_1() {
		assert!(do_parse("GET id OF mytab WHERE age == 123;") == true);
	}

	#[test]
	fn test_get_stmt_2() {
		assert!(do_parse("GET id OF mytab WHERE age == 123 AND id == 323;") == true);
	}

	#[test]
	fn test_get_stmt_3() {	
		assert!(do_parse("GET id, name OF mytab WHERE age == 123;") == true);
	}

	#[test]
	fn test_get_stmt_4() {
		assert!(do_parse("GET ALL id OF mytab WHERE age == 123;") == true);
	}

	#[test]
	fn test_set_stmt_0() {
		assert!(do_parse("SET age = 20 OF mytab WHERE id == 123;") == true);
	}

	#[test]
	fn test_set_stmt_1() {
		assert!(do_parse("SET age = 20, name = \"hige\" OF mytab WHERE id == 123;") == true);
	}

	#[test]
	fn test_add_stmt_0() {
		assert!(do_parse("ADD id = 1 OF mytab;") == true);
	}

	#[test]
	fn test_add_stmt_1() {
		assert!(do_parse("ADD id = 1, age = 20 OF mytab;") == true);
	}

	#[test]
	fn test_add_sub_expr() {
		assert!(do_parse("ADD id = 1 + 2 - 1, age = 20 OF mytab;") == true);
	}

	#[test]
	fn test_mul_div_expr() {
		assert!(do_parse("ADD id = 2 * 2 / 2, age = 20 OF mytab;") == true);
	}

	#[test]
	fn test_del_stmt_0() {
		assert!(do_parse("DEL OF mytab WHERE id == 1;") == true);
	}

	#[test]
	fn test_del_stmt_1() {
		assert!(do_parse("DEL ALL OF mytab;") == true);
	}

	#[test]
	fn test_first_column_type() {
		assert!(do_parse("CREATE TABLE mytab (id: PRIMARY_KEY)") == false);
	}
}