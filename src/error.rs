#[derive(Debug)]
pub enum Error {
    Tokenize(String),
    Parse(String),
    Exec(String),
    Runtime(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::Tokenize(s) => write!(f, "failed to tokenize. {}", s),
            Error::Parse(s) => write!(f, "failed to parse. {}", s),
            Error::Exec(s) => write!(f, "failed to execute. {}", s),
            Error::Runtime(s) => write!(f, "failed on runtime. {}", s),
        }
    }
}

impl std::error::Error for Error {}

macro_rules! make_error {
    ($kind:ident, $msg:expr) => {
        Error::$kind(format!(
            "{}:{}:{}: {}",
            file!(),
            line!(),
            column!(),
            $msg
        ))
    };
}

macro_rules! err_tokenize {
    ($msg:expr) => {
        Err(make_error!(Tokenize, $msg))
    };
}

macro_rules! err_parse {
    ($msg:expr) => {
        Err(make_error!(Parse, $msg))
    };
}

macro_rules! err_exec {
    ($msg:expr) => {
        Err(make_error!(Exec, $msg))
    };
}

macro_rules! err_runtime {
    ($msg:expr) => {
        Err(make_error!(Runtime, $msg))
    };
}

pub(crate) use make_error;
pub(crate) use err_tokenize;
pub(crate) use err_parse;
pub(crate) use err_exec;
pub(crate) use err_runtime;