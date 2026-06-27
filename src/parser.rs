use crate::tokenizer::{Token, TokenKind, TokenStream, tokenize};
use crate::error::{Error, make_error, err_parse};
use crate::utils::{debug};

pub struct QueryNode {
	stmts: Vec<Box<StmtNode>>,
}

impl QueryNode {
	pub fn new() -> Self {
		Self {
			stmts: vec![],
		}
	}
}

struct StmtNode {
	create_stmt: Option<Box<CreateStmtNode>>,
	get_stmt: Option<Box<GetStmtNode>>,
	set_stmt: Option<Box<SetStmtNode>>,
	add_stmt: Option<Box<AddStmtNode>>,
	del_stmt: Option<Box<DelStmtNode>>,
}

impl StmtNode {
	pub fn new() -> Self {
		Self {
			create_stmt: None,
			get_stmt: None,
			set_stmt: None,
			add_stmt: None,
			del_stmt: None,
		}
	}
}

struct CreateStmtNode {
	create_database: Option<Box<CreateDatabaseNode>>,
	create_table: Option<Box<CreateTableNode>>,
}

impl CreateStmtNode {
	pub fn new() -> Self {
		Self {
			create_database: None,
			create_table: None,
		}
	}
}

struct CreateDatabaseNode {
	ident: Option<Box<IdentNode>>,
}

impl CreateDatabaseNode {
	pub fn new() -> Self {
		Self {
			ident: None,
		}
	}
}

struct CreateTableNode {
	ident: Option<Box<IdentNode>>,
	column_definitions: Vec<Box<ColumnDefinitionNode>>,
}

impl CreateTableNode {
	pub fn new() -> Self {
		Self {
			ident: None,
			column_definitions: vec![],
		}
	}
}

struct ColumnDefinitionNode {
	ident: Option<Box<IdentNode>>,
	column_types: Vec<ColumnTypeNode>,
}

impl ColumnDefinitionNode {
	pub fn new() -> Self {
		Self {
			ident: None,
			column_types: vec![],
		}
	}
}

enum ColumnTypeNode {
	PrimaryKey,
	AutoIncrement,
	I64,
	F64,
	Char(usize),
}

struct GetStmtNode {
	all: bool,
	expr_list: Option<Box<ExprListNode>>,
	table: Option<Box<IdentNode>>,
	where_clause: Option<Box<WhereClauseNode>>,
}

impl GetStmtNode {
	pub fn new() -> Self {
		Self {
			all: false,
			expr_list: None,
			table: None,
			where_clause: None,
		}
	}
}

struct SetStmtNode {
	all: bool,
	expr_list: Option<Box<ExprListNode>>,
	table: Option<Box<IdentNode>>,
	where_clause: Option<Box<WhereClauseNode>>,
}

impl SetStmtNode {
	pub fn new() -> Self {
		Self {
			all: false,
			expr_list: None,
			table: None,
			where_clause: None,
		}
	}
}

struct AddStmtNode {
	expr_list: Option<Box<ExprListNode>>,
	table: Option<Box<IdentNode>>,
}

impl AddStmtNode {
	pub fn new() -> Self {
		Self {
			expr_list: None,
			table: None,
		}
	}
}

struct DelStmtNode {
	all: bool,
	table: Option<Box<IdentNode>>,
	where_clause: Option<Box<WhereClauseNode>>,
}

impl DelStmtNode {
	pub fn new() -> Self {
		Self {
			all: false,
			table: None,
			where_clause: None,
		}
	}
}

struct WhereClauseNode {
	expr_list: Option<Box<ExprListNode>>,
}

impl WhereClauseNode {
	pub fn new() -> Self {
		Self {
			expr_list: None,
		}
	}
}

struct ExprListNode {
	exprs: Vec<Box<ExprNode>>,
}

impl ExprListNode {
	pub fn new() -> Self {
		Self {
			exprs: vec![],
		}
	}
}

struct ExprNode {
	ass_expr: Option<Box<AssExprNode>>,
}

impl ExprNode {
	pub fn new() -> Self {
		Self {
			ass_expr: None,
		}
	}
}

struct AssExprNode {
	left_compare_expr: Option<Box<CompareExprNode>>,
	right_compare_expr: Option<Box<CompareExprNode>>,
}

impl AssExprNode {
	pub fn new() -> Self {
		Self {
			left_compare_expr: None,
			right_compare_expr: None,
		}
	}
}

enum CompareExprItemNode {
	Left(Box<LogicExprNode>),
	Op(Box<CompareOpNode>),
	Right(Box<LogicExprNode>),
}

struct CompareExprNode {
	nodes: Vec<CompareExprItemNode>,
}

impl CompareExprNode {
	pub fn new() -> Self {
		Self {
			nodes: vec![],
		}
	}
}

enum CompareOpNode {
	Gt,
	Gte,
	Lt,
	Lte,
	Eq,
	NotEq,
}

enum LogicExprItemNode {
	Left(Box<OperandNode>),
	Op(Box<LogicOpNode>),
	Right(Box<OperandNode>),
}

struct LogicExprNode {
	nodes: Vec<LogicExprItemNode>,
}

impl LogicExprNode {
	pub fn new() -> Self {
		Self {
			nodes: vec![],
		}
	}
}

enum LogicOpNode {
	And,
	Or,
}

struct OperandNode {
	int_value: Option<Box<IntValueNode>>,
	float_value: Option<Box<FloatValueNode>>,
	string: Option<Box<StringNode>>,
	ident: Option<Box<IdentNode>>,
	expr: Option<Box<ExprNode>>,
}

impl OperandNode {
	pub fn new() -> Self {
		Self {
			int_value: None,
			float_value: None,
			string: None,
			ident: None,
			expr: None,
		}
	}
}

struct IntValueNode {
	value: i64,
}

impl IntValueNode {
	pub fn new() -> Self {
		Self {
			value: 0,
		}
	}
}

struct FloatValueNode {
	value: f64,
}

impl FloatValueNode {
	pub fn new() -> Self {
		Self {
			value: 0.0,
		}
	}
}

struct StringNode {
	value: String,
}

impl StringNode {
	pub fn new() -> Self {
		Self {
			value: String::new(),
		}
	}
}

struct IdentNode {
	value: String,
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
			return err_parse!("missing semicolon in stmt");
		}
	}

	Ok(query)
}

pub fn parse_stmt(tok_strm: &mut TokenStream) -> Result<Option<Box<StmtNode>>, Error> {
	let mut stmt = StmtNode::new();

	let create_stmt = parse_create_stmt(tok_strm)?;
	if !create_stmt.is_none() {
		stmt.create_stmt = create_stmt;
		return Ok(Some(Box::new(stmt)));
	}

	let get_stmt = parse_get_stmt(tok_strm)?;
	if !get_stmt.is_none() {
		stmt.get_stmt = get_stmt;
		return Ok(Some(Box::new(stmt)));
	}

	let set_stmt = parse_set_stmt(tok_strm)?;
	if !set_stmt.is_none() {
		stmt.set_stmt = set_stmt;
		return Ok(Some(Box::new(stmt)));
	}

	let add_stmt = parse_add_stmt(tok_strm)?;
	if !add_stmt.is_none() {
		stmt.add_stmt = add_stmt;
		return Ok(Some(Box::new(stmt)));
	}

	let del_stmt = parse_del_stmt(tok_strm)?;
	if !del_stmt.is_none() {
		stmt.del_stmt = del_stmt;
		return Ok(Some(Box::new(stmt)));
	}

	return err_parse!("failed to parse stmt");
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

	Ok(Some(Box::new(n)))
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
	} else if tok.kind == TokenKind::I64 {
		return Ok(Some(ColumnTypeNode::I64));		
	} else if tok.kind == TokenKind::F64 {
		return Ok(Some(ColumnTypeNode::F64));		
	} else if tok.kind == TokenKind::Char {
		let tok = tok_strm.get()?;
		if tok.kind != TokenKind::LBracket {
			return err_parse!("missing [ in char of column type");
		}

		let int_value = parse_int_value(tok_strm)?;
		if int_value.is_none() {
			return err_parse!("missing number of char in column type");
		}

		let tok = tok_strm.get()?;
		if tok.kind != TokenKind::RBracket {
			return err_parse!("missing ] in char of column type");
		}		

		let value = int_value.unwrap().value as usize;
		return Ok(Some(ColumnTypeNode::Char(value)))
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
		return err_parse!("missing 'OF' in add stmt");
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

	let expr_list = parse_expr_list(tok_strm)?;
	n.expr_list = expr_list;

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

	n.left_compare_expr = parse_compare_expr(tok_strm)?;
	if n.left_compare_expr.is_none() {
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

	n.right_compare_expr = parse_compare_expr(tok_strm)?;
	if n.right_compare_expr.is_none() {
		return err_parse!("missing right hand operand in ass expr");
	}

	Ok(Some(Box::new(n)))
}

pub fn parse_compare_expr(tok_strm: &mut TokenStream) -> Result<Option<Box<CompareExprNode>>, Error> {
	let mut n = CompareExprNode::new();

	if tok_strm.is_end() {
		return Ok(None);
	}

	let left = parse_logic_expr(tok_strm)?;
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

		let node = CompareExprItemNode::Op(op.unwrap());
		n.nodes.push(node);

		let right = parse_logic_expr(tok_strm)?;
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
	} else {
		tok_strm.prev();
		return Ok(None);
	}
}

pub fn parse_logic_expr(tok_strm: &mut TokenStream) -> Result<Option<Box<LogicExprNode>>, Error> {
	let mut n = LogicExprNode::new();

	if tok_strm.is_end() {
		return Ok(None);
	}

	let left = parse_operand(tok_strm)?;
	if left.is_none() {
		return Ok(None);
	}

	let node = LogicExprItemNode::Left(left.unwrap());
	n.nodes.push(node);

	while !tok_strm.is_end() {
		let op = parse_logic_op(tok_strm)?;
		if op.is_none() {
			break;
		}

		let node = LogicExprItemNode::Op(op.unwrap());
		n.nodes.push(node);

		let right = parse_operand(tok_strm)?;
		if right.is_none() {
			return err_parse!("missing right hand operand in logic expr");
		}

		let node = LogicExprItemNode::Right(right.unwrap());
		n.nodes.push(node);
	}

	Ok(Some(Box::new(n)))
}

pub fn parse_logic_op(tok_strm: &mut TokenStream) -> Result<Option<Box<LogicOpNode>>, Error> {
	if tok_strm.is_end() {
		return Ok(None);
	}

	let tok = tok_strm.get()?;
	if tok.kind == TokenKind::And {
		return Ok(Some(Box::new(LogicOpNode::And)));
	} else if tok.kind == TokenKind::Or {
		return Ok(Some(Box::new(LogicOpNode::Or)));
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

		n.int_value = parse_int_value(tok_strm)?;
		if !n.int_value.is_none() {
			return Ok(Some(Box::new(n)));
		}

		n.float_value = parse_float_value(tok_strm)?;
		if !n.float_value.is_none() {
			return Ok(Some(Box::new(n)));
		}

		n.string = parse_string(tok_strm)?;
		if !n.string.is_none() {
			return Ok(Some(Box::new(n)));
		}

		n.ident = parse_ident(tok_strm)?;
		if !n.ident.is_none() {
			return Ok(Some(Box::new(n)));
		}

		return Ok(None);
	}
}

pub fn parse_int_value(tok_strm: &mut TokenStream) -> Result<Option<Box<IntValueNode>>, Error> {
	let mut n = IntValueNode::new();

	if tok_strm.is_end() {
		return Ok(None);
	}

	let tok = tok_strm.get()?;
	if tok.kind != TokenKind::Int {
		tok_strm.prev();
		return Ok(None);
	}

	n.value = tok.int_value.unwrap();

	Ok(Some(Box::new(n)))
}

pub fn parse_float_value(tok_strm: &mut TokenStream) -> Result<Option<Box<FloatValueNode>>, Error> {
	let mut n = FloatValueNode::new();

	if tok_strm.is_end() {
		return Ok(None);
	}

	let tok = tok_strm.get()?;
	if tok.kind != TokenKind::Float {
		tok_strm.prev();
		return Ok(None);
	}

	n.value = tok.float_value.unwrap();

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
		let node = match parse(&mut strm) {
			Ok(v) => v,
			Err(e) => {
				eprintln!("{}", e);
				return false;
			}
		};

		true
	}

	#[test]
	fn test_create_database_stmt() {
		assert!(do_parse("CREATE DATABASE mydb;") == true);
	}

	#[test]
	fn test_create_table_stmt_0() {
		assert!(do_parse("CREATE TABLE mytab (id: I64);") == true);
	}

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
	fn test_del_stmt_0() {
		assert!(do_parse("DEL OF mytab WHERE id == 1;") == true);
	}

	#[test]
	fn test_del_stmt_1() {
		assert!(do_parse("DEL ALL OF mytab;") == true);
	}
}