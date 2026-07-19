use crate::options::Options;
use crate::utils::{exec_query};
use crate::context::{Context};

pub fn run(opts: Options) {
	let query = opts.query_string.clone();
	let mut context = Context::new();
	context.is_cli = true;

	match exec_query(opts, query, &mut context) {
		Ok(_) => {},
		Err(e) => { 
			eprintln!("{}", e);
			std::process::exit(1);
		}
	}
}
