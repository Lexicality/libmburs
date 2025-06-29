// Copyright 2023 Lexi Robinson
// Licensed under the EUPL-1.2
use winnow::{Bytes, Parser};

use libmbus::parse::link_layer::Packet;
use libmbus::utils::read_test_file;

fn main() {
	for fname in std::env::args().skip(1) {
		println!("File {fname:?}:");

		let data = read_test_file(&fname).expect("Could not open file");

		let packet = Packet::parse.parse(Bytes::new(&data[..]));

		match packet {
			Ok(packet) => println!("{packet:#?}"),
			Err(e) => eprintln!("{}", e.into_inner()),
		}
	}
}
