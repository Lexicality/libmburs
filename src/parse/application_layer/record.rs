// Copyright 2024 Lexi Robinson
// Licensed under the EUPL-1.2

use super::{dib::DataInfoBlock, vib::ValueInfoBlock};
use crate::parse::application_layer::dib::RawDataType;
use crate::parse::error::MBResult;
use crate::parse::types::number::parse_bcd;
use crate::parse::types::DataType;
use winnow::binary;
use winnow::prelude::*;
use winnow::Bytes;

#[derive(Debug)]
pub struct Record {
	pub dib: DataInfoBlock,
	pub vib: ValueInfoBlock,
	pub data: DataType,
}

impl Record {
	pub fn parse(input: &mut &Bytes) -> MBResult<Self> {
		let (dib, vib) =
			binary::bits::bits((DataInfoBlock::parse, ValueInfoBlock::parse)).parse_next(input)?;

		// TODO: The vib can change how this data is parsed!
		let data = match dib.raw_type {
			RawDataType::BCD(num) => parse_bcd(num).map(DataType::Signed).parse_next(input)?,
			RawDataType::None => DataType::None,
			_ => unimplemented!(),
		};

		Ok(Self { dib, vib, data })
	}
}
