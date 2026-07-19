mod consts;
mod cli;
mod query;
mod usage;
mod error;
mod options;
mod utils;
mod tokenizer;
mod parser;
mod planner;
mod executor;
mod context;
mod objects;
mod security;

use crate::options::{Options};

fn main() {
    let mut opts = Options::new();
    opts.parse_args();

    if opts.is_usage {
        usage::run();
    } else if opts.is_query {
        query::run(opts);
    } else {
        cli::run(opts);
    }
}
