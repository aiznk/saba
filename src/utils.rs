macro_rules! debug {
	($text:expr) => {
		println!("{}", $text.to_string());
	}
}

pub(crate) use debug;
