// Copyright 2024 Lexi Robinson
// Licensed under the EUPL-1.2

use libmbus_macros::vif;
use winnow::binary;
use winnow::combinator::{alt, repeat};
use winnow::error::StrContext;
use winnow::prelude::*;
use winnow::Bytes;

use crate::parse::error::{MBResult, MBusError};
use crate::parse::types::date::{TypeFDateTime, TypeGDate, TypeIDateTime, TypeJTime, TypeKDST};
use crate::parse::types::number::{
	parse_bcd, parse_binary_signed, parse_binary_unsigned, parse_invalid_bcd, parse_real,
};
use crate::parse::types::string::parse_latin1;
use crate::parse::types::DataType;

use super::dib::{DataInfoBlock, RawDataType};
use super::vib::{ValueInfoBlock, ValueType};

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

		let vib = handle_date_types(&dib, vib);

		let unsigned = vib.value_type.is_unsigned();
		let data = match vib.value_type {
			ValueType::TypeFDateTime => TypeFDateTime::parse
				.map(DataType::DateTimeF)
				.context(StrContext::Label("Type F Date/Time"))
				.parse_next(input)?,
			ValueType::TypeGDate => TypeGDate::parse
				.map(DataType::Date)
				.context(StrContext::Label("Type G Date"))
				.parse_next(input)?,
			ValueType::TypeIDateTime => TypeIDateTime::parse
				.map(DataType::DateTimeI)
				.context(StrContext::Label("Type I Date/Time"))
				.parse_next(input)?,
			ValueType::TypeJTime => TypeJTime::parse
				.map(DataType::Time)
				.context(StrContext::Label("Type J Time"))
				.parse_next(input)?,
			ValueType::DSTTypeK => TypeKDST::parse
				.map(DataType::DST)
				.context(StrContext::Label("Daylight Savings Type K"))
				.parse_next(input)?,
			// TODO: I've commented this out as it means that these will simply
			// parse as a large lvar number and it's the caller to parse it
			// themselves. I need to figure out a good way of handling this.
			// ValueType::TypeMDatetime => {
			// 	return Err(MBusError::assert(input, "Type M dates not implemented yet"))
			// }
			_ => match dib.raw_type {
				RawDataType::BCD(num) => alt((
					parse_bcd(num).map(DataType::Signed),
					parse_invalid_bcd(num).map(DataType::ErrorValue),
				))
				.parse_next(input)?,
				RawDataType::Binary(num) => parse_binary(unsigned, num).parse_next(input)?,
				RawDataType::Real => parse_real.map(DataType::Real).parse_next(input)?,
				RawDataType::None => DataType::None,
				RawDataType::LVAR => {
					let value = binary::u8
						.verify(
							|v| matches!(v, 0x00..=0xBF | 0xC0..=0xC9 | 0xD0..=0xD9 | 0xE0..=0xEF | 0xF0..=0xF6),
						)
						.map(|v| v.into())
						.context(StrContext::Label("LVAR value"))
						.parse_next(input)?;
					match value {
						// For some unknowable reason, the LVAR value can specify to parse 0 bytes
						n @ 0x00..=0xBF => {
							parse_latin1(n).map(DataType::String).parse_next(input)?
						}
						n @ 0xC0..=0xC9 => parse_bcd(n - 0xC0)
							.verify(|v| *v > 0)
							.map(DataType::Signed)
							.parse_next(input)?,
						n @ 0xD0..=0xD9 => parse_bcd(n - 0xD0)
							.map(|v| DataType::Signed(if v > 0 { -v } else { v }))
							.parse_next(input)?,
						n @ 0xE0..=0xE8 => parse_binary(unsigned, n - 0xE0).parse_next(input)?,
						n @ 0xE9..=0xEF => parse_giant_number(n - 0xE0).parse_next(input)?,
						n @ 0xF0..=0xF4 => parse_giant_number(4 * (n - 0xEC)).parse_next(input)?,
						0xF5 => parse_giant_number(48).parse_next(input)?,
						0xF6 => parse_giant_number(64).parse_next(input)?,
						_ => unreachable!(),
					}
				}
			},
		};

		Ok(Self { dib, vib, data })
	}
}

pub fn parse_binary<'a>(
	unsigned: bool,
	bytes: usize,
) -> impl Parser<&'a Bytes, DataType, MBusError> {
	move |input: &mut &'a Bytes| {
		if unsigned {
			parse_binary_unsigned(bytes)
				.map(DataType::Unsigned)
				.parse_next(input)
		} else {
			parse_binary_signed(bytes)
				.map(DataType::Signed)
				.parse_next(input)
		}
	}
}

fn parse_giant_number<'a>(bytes: usize) -> impl Parser<&'a Bytes, DataType, MBusError> {
	repeat(bytes, binary::u8).map(DataType::VariableLengthNumber)
}

fn handle_date_types(dib: &DataInfoBlock, mut vib: ValueInfoBlock) -> ValueInfoBlock {
	vib.value_type = match vib.value_type {
		ValueType::TypeGDate => match dib.raw_type {
			RawDataType::Binary(2) => ValueType::TypeGDate,
			_ => ValueType::Invalid(vif!(E110 1100)),
		},
		ValueType::VariableDateTime => match dib.raw_type {
			RawDataType::LVAR => ValueType::TypeMDatetime,
			RawDataType::Binary(4) => ValueType::TypeFDateTime,
			RawDataType::Binary(3) => ValueType::TypeJTime,
			RawDataType::Binary(5) => ValueType::TypeIDateTime,
			_ => ValueType::Invalid(vif!(E110 1101)),
		},
		vt => vt,
	};
	vib
}
