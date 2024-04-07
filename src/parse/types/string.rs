// Copyright 2024 Lexi Robinson
// Licensed under the EUPL-1.2

use encoding_rs::WINDOWS_1252;
use winnow::binary;
use winnow::combinator::repeat;
use winnow::prelude::*;
use winnow::stream::Bytes;

use crate::parse::error::{MBResult, MBusError};

pub fn parse_length_prefix_ascii(input: &mut &Bytes) -> MBResult<String> {
	binary::length_take(binary::u8)
		.try_map(convert_ascii_string)
		.parse_next(input)
}

fn convert_ascii_string(data: &[u8]) -> core::result::Result<String, std::str::Utf8Error> {
	Ok(std::str::from_utf8(data)?.chars().rev().collect())
}

pub fn parse_latin1<'a>(num_bytes: usize) -> impl Parser<&'a Bytes, String, MBusError> {
	move |input: &mut &'a Bytes| {
		repeat::<_, _, (), _, _>(num_bytes, binary::u8)
			.recognize()
			.map(|data| WINDOWS_1252.decode(data).0.chars().rev().collect())
			.parse_next(input)
	}
}
