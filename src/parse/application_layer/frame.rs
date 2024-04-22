// Copyright 2024 Lexi Robinson
// Licensed under the EUPL-1.2

use winnow::binary;
use winnow::combinator::{alt, eof, repeat, repeat_till};
use winnow::error::StrContext;
use winnow::prelude::*;
use winnow::Bytes;

use super::record::Record;
use crate::parse::error::MBResult;

const IDLE_FILLER: u8 = 0x2F;

#[derive(Debug)]
pub struct Frame {
	pub records: Vec<Record>,
	pub more_data_follows: bool,
	pub manufacturer_specific: Vec<u8>,
}

impl Frame {
	pub fn parse(input: &mut &Bytes) -> MBResult<Self> {
		let idle_filler = repeat::<_, _, (), _, _>(1.., IDLE_FILLER)
			.context(StrContext::Label("idle filler"))
			.map(|_| None);

		let record = Record::parse
			.context(StrContext::Label("frame record"))
			.map(Some);

		let end_of_records = alt((
			// The frame can simply end on a record boundary indicating no
			// more records
			eof.map(|_| false),
			// Or it can have one of the following bytes
			0x1F.map(|_| true),
			// Though it's not legal for this one to exist without some data after it
			0x0F.map(|_| false),
		))
		.context(StrContext::Label("end of records marker"));

		let records_with_idle = repeat_till::<_, _, Vec<Option<Record>>, _, _, _, _>(
			0..,
			alt((idle_filler, record)),
			end_of_records,
		)
		.map(|(records, more_data)| (records.into_iter().flatten().collect(), more_data));

		let manufacturer_specific = repeat::<_, _, Vec<_>, _, _>(0.., binary::u8)
			.context(StrContext::Label("manufacturer specific data"));

		(records_with_idle, manufacturer_specific)
			.map(
				|((records, more_data_follows), manufacturer_specific)| Self {
					records,
					more_data_follows,
					manufacturer_specific,
				},
			)
			.parse_next(input)
	}
}
