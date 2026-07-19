use crate::tokenizer::{Token, TokenStream, tokenize};
use crate::parser::{QueryNode, parse};
use crate::planner::{PlansNode, planning};
use crate::executor::{exec};
use crate::context::{Context};
use crate::error::{Error};
use crate::utils::{exec_query};
use crate::options::{Options};
use std::io::Write;
use std::path::{PathBuf};

fn show_prompt(opts: &Options, context: &Context) {
	if context.using_db_name.len() == 0 {
		print!("{} > ", opts.root_dir_path.display());
	} else {
	    print!("{}:{} > ", opts.root_dir_path.display(), context.using_db_name);
	}
	std::io::stdout().flush().unwrap();	
}

fn run_shell(opts: Options) {
	let mut context = Context::new();
	context.is_cli = true;

	loop {
		show_prompt(&opts, &context);

		let mut line = String::new();
		match std::io::stdin().read_line(&mut line) {
			Ok(_) => {},
			Err(e) => eprintln!("failed read line: {}", e),
		}
		if line.len() == 0 {
			break;
		}

		match exec_query(opts.clone(), line, &mut context) {
			Ok(_) => {},
			Err(e) => eprintln!("{}", e),
		}
	}
}

pub fn run(opts: Options) {
	run_shell(opts);
}
