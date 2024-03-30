// Copyright 2023 Lexi Robinson
// Licensed under the EUPL-1.2

use libmbus::parse::link_layer::Packet;
use libmbus::parse::transport_layer::CICode;
use std::error;
use winnow::{Bytes, Parser};

fn do_file(fname: &str) -> Result<(), Box<dyn error::Error>> {
	let data = std::fs::read(fname).map_err(Box::new)?;

	let packet: Packet = Packet::parse
		.parse(Bytes::new(&data[..]))
		.map_err(|e| e.into_inner().to_string())?;
	println!("{packet:?}");

	match packet {
		Packet::Long { data, .. } => {
			let mut data = Bytes::new(data);
			let ci = CICode::parse
				.parse_next(&mut data)
				.map_err(|e| e.to_string())?;
			println!("{ci:?}");
		}
		_ => todo!(),
	}
	Ok(())
}

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
