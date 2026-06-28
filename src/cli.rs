use crate::tokenizer::{Token, TokenStream, tokenize};
use crate::parser::{QueryNode, parse};
use crate::planner::{PlansNode, planning};
use crate::executor::{exec};
use crate::context::{Context};
use crate::error::{Error, make_error, err_exec};
use std::io::Write;

#[derive(Clone)]
struct Options {
	help: bool,
	root_dir_path: Option<String>,
}

impl Options {
	pub fn new() -> Self {
		Self {
			help: false,
			root_dir_path: None,
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

fn exec_query(opts: Options, query: String) -> Result<(), Error> {
	if opts.root_dir_path.is_none() {
		return err_exec!("root directory is none");
	}

	let mut context = Context::new();
	context.root_dir_path = opts.root_dir_path.clone().unwrap();
	let tokens: Vec<Token> = tokenize(query)?;
	let mut tok_strm = TokenStream::new(tokens);
	let node: QueryNode = parse(&mut tok_strm)?;
	let node: PlansNode = planning(&node)?;
	exec(&mut context, &node)?;
	Ok(())
}

fn run_shell(opts: Options) {
	// match exec_query(opts, String::from("GET id OF table WHERE name = \"hige\";")) {
	// 	Ok(_) => {},
	// 	Err(e) => eprintln!("{}", e),
	// }
	// return;

	loop {
		print!("query > ");
		std::io::stdout().flush().unwrap();

		let mut line = String::new();
		match std::io::stdin().read_line(&mut line) {
			Ok(_) => {},
			Err(e) => eprintln!("failed read line: {}", e),
		}
		if line.len() == 0 {
			break;
		}

		match exec_query(opts.clone(), line) {
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
				opts.root_dir_path = Some(String::from(arg));
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

	if opts.help {
		usage();
	} else if !opts.root_dir_path.is_none() {
		run_shell(opts);
	}
}
