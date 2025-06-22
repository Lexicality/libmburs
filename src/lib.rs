// Copyright 2023 Lexi Robinson
// Licensed under the EUPL-1.2

pub mod parse;

pub mod utils {
	use crate::parse::error::MBusError;

	pub fn read_test_file(filename: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
		if filename.ends_with(".hex") {
			let data = std::fs::read_to_string(filename)?;

			data.trim()
				.split(' ')
				.map(|substr| u8::from_str_radix(substr, 16))
				.collect::<Result<Vec<_>, _>>()
				.map_err(|e| e.into())
		} else {
			std::fs::read(filename).map_err(|e| e.into())
		}
	}

	pub fn fancy_error(error: &MBusError) {
		// eprint!("{}: ", error.kind());
		if let Some(cause) = error.cause() {
			eprintln!("{}", cause);
		}
		for (n, cause) in error.context().enumerate() {
			eprintln!("{}{}", " ".repeat(n), cause);
		}
	}
}
