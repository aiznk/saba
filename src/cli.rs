use crate::tokenizer::{Token, TokenStream, tokenize};
use crate::parser::{QueryNode, parse};
use crate::planner::{PlansNode, planning};
use crate::executor::{exec};
use crate::context::{Context};
use crate::error::{Error};
use std::io::Write;
use std::path::{PathBuf};

#[derive(Clone)]
struct Options {
	help: bool,
	root_dir_path: PathBuf,
}

impl Options {
	pub fn new() -> Self {
		Self {
			help: false,
			root_dir_path: PathBuf::new(),
		}
	}
}

fn usage() {
	println!("Saba is database of CSV files.

Usage:

	saba [directory] [options]

The options are:

	-h, --help    Show usage.
");
	std::process::exit(0);
}

fn exec_query(opts: Options, query: String, context: &mut Context) -> Result<(), Error> {
	context.root_dir_path = opts.root_dir_path.clone();
	let tokens: Vec<Token> = tokenize(query)?;
	let mut tok_strm = TokenStream::new(tokens);
	let node: QueryNode = parse(&mut tok_strm)?;
	let mut node: PlansNode = planning(&node)?;
	exec(context, &mut node)?;
	Ok(())
}

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

fn parse_options(args: Vec<String>) -> Options {
	let mut opts = Options::new();

	for arg in args {
		match arg.as_str() {
			"-h" => { opts.help = true; }
			"--help" => { opts.help = true; }
			&_ => {
				opts.root_dir_path = PathBuf::from(arg);
			}
		}
	}

	opts
}

pub fn run() {
	let args: Vec<String> = std::env::args().collect();
	if args.len() < 2 {
		usage();
	}

	let opts = parse_options(args);
	if !opts.root_dir_path.exists() {
		eprintln!("\"{}\" does not exists path", opts.root_dir_path.display());
		std::process::exit(1);
	}

	if opts.help {
		usage();
	} else {
		run_shell(opts);
	}
}
