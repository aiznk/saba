mod cli;
mod error;
mod utils;
mod tokenizer;
mod parser;
mod planner;
mod executor;
mod context;

fn main() {
    cli::run();
}
