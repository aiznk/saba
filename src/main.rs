mod consts;
mod cli;
mod error;
mod utils;
mod tokenizer;
mod parser;
mod planner;
mod executor;
mod context;
mod objects;

fn main() {
    cli::run();
}
