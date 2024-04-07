// Copyright 2024 Lexi Robinson
// Licensed under the EUPL-1.2

use super::record::Record;
use crate::parse::error::MBResult;
use winnow::combinator::{alt, eof, repeat, repeat_till};
use winnow::error::StrContext;
use winnow::prelude::*;
use winnow::Bytes;

const IDLE_FILLER: u8 = 0x2F;

#[derive(Debug)]
pub struct Frame {
	pub records: Vec<Record>,
	pub more_data_follows: bool,
	pub manufacturer_specific: Vec<u8>,
}

impl Frame {
	pub fn parse(input: &mut &Bytes) -> MBResult<Self> {
		let (records, more_data_follows) = repeat_till(
			1..,
			(
				repeat::<_, _, (), _, _>(0.., IDLE_FILLER),
				Record::parse.context(StrContext::Label("frame record")),
			)
				.map(|(_, record)| record),
			alt((
				// The frame can simply end on a record boundary indicating no
				// more records
				eof.map(|_| false),
				// Or it can have one of the following bytes
				0x1F.map(|_| true),
				// Though it's not legal for this one to exist without some data after it
				0x0F.map(|_| false),
			)),
		)
		.parse_next(input)?;

		Ok(Self {
			records,
			more_data_follows,
			manufacturer_specific: input.iter().copied().collect(),
		})
	}
}
