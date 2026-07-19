use crate::tokenizer::{Token, TokenStream, tokenize};
use crate::parser::{QueryNode, parse};
use crate::planner::{PlansNode, planning};
use crate::executor::{exec};
use crate::context::{Context};
use crate::error::{Error};
use crate::options::{Options};

pub fn exec_query(opts: Options, query: String, context: &mut Context) -> Result<(), Error> {
	let query = if opts.is_use {
		format!("{} {}", opts.gen_use_query()?, query)
	} else {
		query
	};
	context.root_dir_path = opts.root_dir_path.clone();
	let tokens: Vec<Token> = tokenize(query)?;
	let mut tok_strm = TokenStream::new(tokens);
	let node: QueryNode = parse(&mut tok_strm)?;
	let mut node: PlansNode = planning(&node)?;
	exec(context, &mut node)?;
	Ok(())
}

