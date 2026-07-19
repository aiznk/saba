use crate::error::{Error, make_error, err_security};

pub fn check_query(query: &str) -> Result<(), Error> {
	Ok(())
}

pub fn check_query_param(param: &str) -> Result<(), Error> {
	if param.contains(";") {
		err_security!("found ; in query param")
	} else {
		Ok(())
	}
}
