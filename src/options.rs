use std::path::{PathBuf};
use crate::error::{Error, make_error, err_runtime};
use crate::security::{check_query};

#[derive(Clone, Debug)]
pub struct Options {
    pub is_query: bool,
    pub is_usage: bool,
    pub is_use: bool,
    pub query_string: String,
    pub root_dir_path: PathBuf,
    pub use_db_name: String,
}

impl Options {
    pub fn new() -> Self {
        Self {
            is_query: false,
            is_usage: false,
            is_use: false,
            query_string: String::new(),
            root_dir_path: PathBuf::new(),
            use_db_name: String::new(),
        }
    }

    pub fn gen_use_query(&self) -> Result<String, Error> {
    	if self.use_db_name.len() > 0 {
    		let query = format!("USE {};", self.use_db_name);
    		check_query(&query)?;
    		Ok(query)
    	} else {
    		err_runtime!("db name is empty")
    	}
    }

    pub fn parse_args(&mut self) -> Result<(), Error> {
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
                        return err_runtime!("missing query string");
                    }
                    let arg = &args[i];
                    self.query_string = arg.to_string();
                }
                "-u" | "--use" => {
                    i += 1;
                    if i >= args.len() {
                        return err_runtime!("missing use db name");
                    }
                    let arg = &args[i];
                	self.use_db_name = arg.to_string();
                	self.is_use = true;
                }
                &_ => {
                	self.root_dir_path = PathBuf::from(arg);
                }
            }

            i += 1;
        }

        Ok(())
    }
}

