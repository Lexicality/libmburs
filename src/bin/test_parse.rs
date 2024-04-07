// Copyright 2023 Lexi Robinson
// Licensed under the EUPL-1.2

use libmbus::parse::application_layer::record::Record;
use libmbus::parse::link_layer::Packet;
use libmbus::parse::transport_layer::CICode;
use std::error;
use winnow::{error::StrContext, Bytes, Parser};

fn do_file(fname: &str) -> Result<(), Box<dyn error::Error>> {
	let data = std::fs::read(fname).map_err(Box::new)?;

	let packet: Packet = Packet::parse
		.parse(Bytes::new(&data[..]))
		.map_err(|e| e.into_inner().to_string())?;
	match packet {
		Packet::Long { data, .. } => {
			let mut data = Bytes::new(data);
			let ci = CICode::parse
				.parse_next(&mut data)
				.map_err(|e| e.to_string())?;
			println!("{ci:#?}");

			// Parse the first record
			let record1 = Record::parse
				.context(StrContext::Label("first record"))
				.parse_next(&mut data)
				.map_err(|e| e.to_string())?;
			println!("{record1:#?}");
		}
		_ => todo!(),
	}
	Ok(())
}

/*
raw_test_data/2024-01-07T14-03-03.dat raw_test_data/2024-01-18T09-30-12.dat raw_test_data/2024-01-18T09-32-53.dat
*/

fn main() {
	for fname in std::env::args().skip(1) {
		println!("Trying to load file {}", fname);
		let res = do_file(&fname);
		match res {
			Ok(_) => println!("Yay"),
			Err(e) => eprintln!("Oh no: {}", e),
		}
	}
	// honk
}
