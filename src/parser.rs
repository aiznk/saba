use crate::tokenizer::{Token, TokenKind, TokenStream};
use crate::error::{Error, err_parse};
use crate::utils::{debug};

pub struct QueryNode {
	stmts: Vec<StmtNode>,
}

impl QueryNode {
	pub fn new() -> Self {
		Self {
			stmts: vec![],
		}
	}
}

struct StmtNode {
	get_stmt: Option<GetStmtNode>,
	set_stmt: Option<SetStmtNode>,
	add_stmt: Option<AddStmtNode>,
	del_stmt: Option<DelStmtNode>,
}

impl StmtNode {
	pub fn new() -> Self {
		Self {
			get_stmt: None,
			set_stmt: None,
			add_stmt: None,
			del_stmt: None,
		}
	}
}

struct GetStmtNode {
	all: bool,
	expr_list: Option<ExprListNode>,
	table: Option<IdentNode>,
	where_clause: Option<WhereClauseNode>,
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
	expr_list: Option<ExprListNode>,
	table: Option<IdentNode>,
	where_clause: Option<WhereClauseNode>,
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
	expr_list: Option<ExprListNode>,
	table: Option<IdentNode>,
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
	table: Option<IdentNode>,
	where_clause: Option<WhereClauseNode>,
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
	expr_list: Option<ExprListNode>,
}

impl WhereClauseNode {
	pub fn new() -> Self {
		Self {
			expr_list: None,
		}
	}
}

struct ExprListNode {
	exprs: Vec<ExprNode>,
}

impl ExprListNode {
	pub fn new() -> Self {
		Self {
			exprs: vec![],
		}
	}
}

struct ExprNode {
	ass_expr: Option<AssExprNode>,
}

impl ExprNode {
	pub fn new() -> Self {
		Self {
			ass_expr: None,
		}
	}
}

struct AssExprNode {
	left_compare_expr: Option<CompareExprNode>,
	right_compare_expr: Option<CompareExprNode>,
}

impl AssExprNode {
	pub fn new() -> Self {
		Self {
			left_compare_expr: None,
			right_compare_expr: None,
		}
	}
}

struct CompareExprNode {
	left_logic_expr: Option<LogicExprNode>,
	compare_op: Option<CompareOpNode>,
	right_logic_expr: Option<LogicExprNode>,
}

impl CompareExprNode {
	pub fn new() -> Self {
		Self {
			left_logic_expr: None,
			compare_op: None,
			right_logic_expr: None,
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

struct LogicExprNode {
	left_operand: Option<OperandNode>,
	logic_op: Option<LogicOpNode>,
	right_operand: Option<OperandNode>,
}

impl LogicExprNode {
	pub fn new() -> Self {
		Self {
			left_operand: None,
			logic_op: None,
			right_operand: None,
		}
	}
}

enum LogicOpNode {
	And,
	Or,
}

struct OperandNode {
	int_value: Option<IntValueNode>,
	float_value: Option<FloatValueNode>,
	string: Option<StringNode>,
	ident: Option<IdentNode>,
}

impl OperandNode {
	pub fn new() -> Self {
		Self {
			int_value: None,
			float_value: None,
			string: None,
			ident: None,
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
		let stmt: Option<StmtNode> = parse_stmt(tok_strm)?;
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

pub fn parse_stmt(tok_strm: &mut TokenStream) -> Result<Option<StmtNode>, Error> {
	let mut stmt = StmtNode::new();

	let get_stmt = parse_get_stmt(tok_strm)?;
	if !get_stmt.is_none() {
		stmt.get_stmt = get_stmt;
		return Ok(Some(stmt));
	}

	let set_stmt = parse_set_stmt(tok_strm)?;
	if !set_stmt.is_none() {
		stmt.set_stmt = set_stmt;
		return Ok(Some(stmt));
	}

	let add_stmt = parse_add_stmt(tok_strm)?;
	if !add_stmt.is_none() {
		stmt.add_stmt = add_stmt;
		return Ok(Some(stmt));
	}

	let del_stmt = parse_del_stmt(tok_strm)?;
	if !del_stmt.is_none() {
		stmt.del_stmt = del_stmt;
		return Ok(Some(stmt));
	}

	return err_parse!("failed to parse stmt");
}

pub fn parse_get_stmt(tok_strm: &mut TokenStream) -> Result<Option<GetStmtNode>, Error> {
	let mut n = GetStmtNode::new();

	if tok_strm.is_end() {
		return err_parse!("reached eof");
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

	Ok(Some(n))
}

pub fn parse_set_stmt(tok_strm: &mut TokenStream) -> Result<Option<SetStmtNode>, Error> {
	let mut n = SetStmtNode::new();

	if tok_strm.is_end() {
		return err_parse!("reached eof");
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

	Ok(Some(n))
}

pub fn parse_add_stmt(tok_strm: &mut TokenStream) -> Result<Option<AddStmtNode>, Error> {
	let mut n = AddStmtNode::new();

	if tok_strm.is_end() {
		return err_parse!("reached eof");
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

	Ok(Some(n))
}

pub fn parse_del_stmt(tok_strm: &mut TokenStream) -> Result<Option<DelStmtNode>, Error> {
	let mut n = DelStmtNode::new();

	if tok_strm.is_end() {
		return err_parse!("reached eof");
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

	Ok(Some(n))
}

pub fn parse_where_clause(tok_strm: &mut TokenStream) -> Result<Option<WhereClauseNode>, Error> {
	let mut n = WhereClauseNode::new();

	if tok_strm.is_end() {
		return err_parse!("reached eof");
	}

	let tok = tok_strm.get()?;
	if tok.kind != TokenKind::Where {
		tok_strm.prev();
		return Ok(None);
	}

	let expr_list = parse_expr_list(tok_strm)?;
	n.expr_list = expr_list;

	Ok(Some(n))
}

pub fn parse_expr_list(tok_strm: &mut TokenStream) -> Result<Option<ExprListNode>, Error> {
	let mut n = ExprListNode::new();

	if tok_strm.is_end() {
		return err_parse!("reached eof");
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

	Ok(Some(n))
}

pub fn parse_expr(tok_strm: &mut TokenStream) -> Result<Option<ExprNode>, Error> {
	let mut n = ExprNode::new();

	if tok_strm.is_end() {
		return err_parse!("reached eof");
	}

	n.ass_expr = parse_ass_expr(tok_strm)?;
	if n.ass_expr.is_none() {
		return Ok(None);
	}

	Ok(Some(n))
}

pub fn parse_ass_expr(tok_strm: &mut TokenStream) -> Result<Option<AssExprNode>, Error> {
	let mut n = AssExprNode::new();

	if tok_strm.is_end() {
		return err_parse!("reached eof");
	}

	n.left_compare_expr = parse_compare_expr(tok_strm)?;
	if n.left_compare_expr.is_none() {
		return Ok(None);
	}

	let tok = tok_strm.get()?;
	if tok.kind != TokenKind::Assign {
		tok_strm.prev();
		return Ok(Some(n));
	}

	n.right_compare_expr = parse_compare_expr(tok_strm)?;
	if n.right_compare_expr.is_none() {
		return err_parse!("missing right hand operand in ass expr");
	}

	Ok(Some(n))
}

pub fn parse_compare_expr(tok_strm: &mut TokenStream) -> Result<Option<CompareExprNode>, Error> {
	let mut n = CompareExprNode::new();

	if tok_strm.is_end() {
		return err_parse!("reached eof");
	}

	n.left_logic_expr = parse_logic_expr(tok_strm)?;
	if n.left_logic_expr.is_none() {
		return Ok(None);
	}

	n.compare_op = parse_compare_op(tok_strm)?;
	if n.compare_op.is_none() {
		return Ok(Some(n));
	}

	n.right_logic_expr = parse_logic_expr(tok_strm)?;
	if n.right_logic_expr.is_none() {
		return err_parse!("missing right logic expr in compare expr");
	}

	Ok(Some(n))
}

pub fn parse_compare_op(tok_strm: &mut TokenStream) -> Result<Option<CompareOpNode>, Error> {
	if tok_strm.is_end() {
		return err_parse!("reached eof");
	}

	let tok = tok_strm.get()?;
	if tok.kind == TokenKind::Eq {
		return Ok(Some(CompareOpNode::Eq));
	} else if tok.kind == TokenKind::NotEq {
		return Ok(Some(CompareOpNode::NotEq));
	} else {
		tok_strm.prev();
		return Ok(None);
	}
}

pub fn parse_logic_expr(tok_strm: &mut TokenStream) -> Result<Option<LogicExprNode>, Error> {
	let mut n = LogicExprNode::new();

	if tok_strm.is_end() {
		return err_parse!("reached eof");
	}

	n.left_operand = parse_operand(tok_strm)?;
	if n.left_operand.is_none() {
		return Ok(None);
	}

	n.logic_op = parse_logic_op(tok_strm)?;
	if n.logic_op.is_none() {
		return Ok(Some(n));
	}

	n.right_operand = parse_operand(tok_strm)?;
	if n.right_operand.is_none() {
		return err_parse!("missing right hand operand in logic expr");
	}

	Ok(Some(n))
}

pub fn parse_logic_op(tok_strm: &mut TokenStream) -> Result<Option<LogicOpNode>, Error> {
	if tok_strm.is_end() {
		return err_parse!("reached eof");
	}

	let tok = tok_strm.get()?;
	if tok.kind == TokenKind::And {
		return Ok(Some(LogicOpNode::And));
	} else if tok.kind == TokenKind::Or {
		return Ok(Some(LogicOpNode::Or));
	} else {
		tok_strm.prev();
		return Ok(None);
	}
}

pub fn parse_operand(tok_strm: &mut TokenStream) -> Result<Option<OperandNode>, Error> {
	let mut n = OperandNode::new();

	if tok_strm.is_end() {
		return err_parse!("reached eof");
	}

	n.int_value = parse_int_value(tok_strm)?;
	if !n.int_value.is_none() {
		return Ok(Some(n));
	}

	n.float_value = parse_float_value(tok_strm)?;
	if !n.float_value.is_none() {
		return Ok(Some(n));
	}

	n.string = parse_string(tok_strm)?;
	if !n.string.is_none() {
		return Ok(Some(n));
	}

	n.ident = parse_ident(tok_strm)?;
	if !n.ident.is_none() {
		return Ok(Some(n));
	}

	Ok(None)
}

pub fn parse_int_value(tok_strm: &mut TokenStream) -> Result<Option<IntValueNode>, Error> {
	let mut n = IntValueNode::new();

	if tok_strm.is_end() {
		return err_parse!("reached eof");
	}

	let tok = tok_strm.get()?;
	if tok.kind != TokenKind::Int {
		tok_strm.prev();
		return Ok(None);
	}

	n.value = tok.int_value.unwrap();

	Ok(Some(n))
}

pub fn parse_float_value(tok_strm: &mut TokenStream) -> Result<Option<FloatValueNode>, Error> {
	let mut n = FloatValueNode::new();

	if tok_strm.is_end() {
		return err_parse!("reached eof");
	}

	let tok = tok_strm.get()?;
	if tok.kind != TokenKind::Float {
		tok_strm.prev();
		return Ok(None);
	}

	n.value = tok.float_value.unwrap();

	Ok(Some(n))
}

pub fn parse_string(tok_strm: &mut TokenStream) -> Result<Option<StringNode>, Error> {
	let mut n = StringNode::new();

	if tok_strm.is_end() {
		return err_parse!("reached eof");
	}

	let tok = tok_strm.get()?;
	if tok.kind != TokenKind::String {
		tok_strm.prev();
		return Ok(None);
	}

	n.value = tok.text.clone().unwrap();

	Ok(Some(n))
}

pub fn parse_ident(tok_strm: &mut TokenStream) -> Result<Option<IdentNode>, Error> {
	let mut n = IdentNode::new();

	if tok_strm.is_end() {
		return err_parse!("reached eof");
	}

	let tok = tok_strm.get()?;
	if tok.kind != TokenKind::Ident {
		tok_strm.prev();
		return Ok(None);
	}

	n.value = tok.text.clone().unwrap();

	Ok(Some(n))
}
