use std::path::{PathBuf};

#[derive(Clone, Debug)]
pub struct Options {
    pub is_query: bool,
    pub is_usage: bool,
    pub query_string: String,
    pub root_dir_path: PathBuf,
}

impl Options {
    pub fn new() -> Self {
        Self {
            is_query: false,
            is_usage: false,
            query_string: String::new(),
            root_dir_path: PathBuf::new(),
        }
    }

    pub fn parse_args(&mut self) {
        let args: Vec<String> = std::env::args().collect();
        let mut i = 0;

        while i < args.len() {
            let arg = &args[i];

            match arg.as_str() {
                "-h" | "--help" => {
                    self.is_usage = true;
                }
                "-q" | "--query" => {
                    self.is_query = true;
                    i += 1;
                    if i >= args.len() {
                        panic!("missing query string");
                    }
                    let arg = &args[i];
                    self.query_string = arg.to_string();
                }
                &_ => {
                	self.root_dir_path = PathBuf::from(arg);
                }
            }

            i += 1;
        }
    }
}

