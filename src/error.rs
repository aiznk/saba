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
    ($kind:ident, $($arg:tt)*) => {
        Error::$kind(format!(
            "{}:{}:{}: {}",
            file!(),
            line!(),
            column!(),
            format!($($arg)*)
        ))
    };
}

macro_rules! err_parse {
    ($($arg:tt)*) => {
        Err(make_error!(Parse, $($arg)*))
    };
}

macro_rules! err_exec {
    ($($arg:tt)*) => {
        Err(make_error!(Exec, $($arg)*))
    };
}

macro_rules! err_runtime {
    ($($arg:tt)*) => {
        Err(make_error!(Runtime, $($arg)*))
    };
}

macro_rules! err_planning {
    ($($arg:tt)*) => {
        Err(make_error!(Runtime, $($arg)*))
    };
}

pub(crate) use make_error;
pub(crate) use err_parse;
pub(crate) use err_exec;
pub(crate) use err_runtime;
pub(crate) use err_planning;
