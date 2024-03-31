// Copyright 2024 Lexi Robinson
// Licensed under the EUPL-1.2
#![allow(dead_code)]

use crate::parse::types::{BResult, BitsInput};
use winnow::binary::bits;
use winnow::error::{ErrMode, ParserError};
use winnow::Parser;

#[derive(Debug)]
pub enum RawDataType {
	None,
	Binary(usize),
	Real,
	BCD(usize),
	LVAR,
}

impl RawDataType {
	fn parse<'a>(input: &mut BitsInput<'a>) -> BResult<'a, Self> {
		bits::take(4_usize)
			.verify_map(|value: u8| match value {
				0b0000 => Some(Self::None),
				0b0001..=0b0100 | 0b0110 => Some(Self::Binary(value.into())),
				0b0111 => Some(Self::Binary(8)),
				0b1001 | 0b1010 | 0b1011 | 0b1100 | 0b1110 => {
					Some(Self::BCD((value - 0b1000) as usize))
				}
				0b0101 => Some(Self::Real),
				0b1101 => Some(Self::LVAR),
				0b1000 => None, // TODO: I have no idea what "Selection for readout" means
				_ => unreachable!(),
			})
			.parse_next(input)
	}
}

#[derive(Debug)]
pub enum DataFunction {
	InstantaneousValue,
	MaximumValue,
	MinimumValue,
	ValueDuringErrorState,
}

impl DataFunction {
	fn parse<'a>(input: &mut BitsInput<'a>) -> BResult<'a, Self> {
		bits::take(2_usize)
			.map(|value: u8| match value {
				0b00 => Self::InstantaneousValue,
				0b01 => Self::MaximumValue,
				0b10 => Self::MinimumValue,
				0b11 => Self::ValueDuringErrorState,
				_ => unreachable!(),
			})
			.parse_next(input)
	}
}

#[derive(Debug)]
pub struct DataInfoBlock {
	raw_type: RawDataType,
	function: DataFunction,
	storage: u64,
	tariff: u32,
	device: u16,
}

impl DataInfoBlock {
	pub fn parse<'a>(input: &mut BitsInput<'a>) -> BResult<'a, Self> {
		let (mut extension, mut storage, function, raw_type): (bool, u64, _, _) = (
			bits::bool,
			bits::take(1_usize),
			DataFunction::parse,
			RawDataType::parse,
		)
			.parse_next(input)?;

		let mut tariff = 0;
		let mut device = 0;

		let mut i = 1;
		while extension {
			if i > 10 {
				return Err(ErrMode::assert(input, "Packet has more than 10 DIFEs!"));
			}

			let mut dife_device: u16;
			let mut dife_tariff: u32;
			let mut dife_storage: u64;

			(extension, dife_device, dife_tariff, dife_storage) = (
				bits::bool,
				bits::take(1_usize),
				bits::take(2_usize),
				bits::take(4_usize),
			)
				.parse_next(input)?;

			// TODO: Perhaps this should be a warning rather than an error?
			if !extension && dife_device == 0 && dife_tariff == 0 && dife_storage == 0 {
				return Err(ErrMode::assert(input, "OBIS registers aren't supported"));
			}

			dife_device <<= i;
			dife_tariff <<= 2 * i;
			dife_storage <<= 4 * i;
			i += 1;

			device += dife_device;
			tariff += dife_tariff;
			storage += dife_storage;
		}

		Ok(Self {
			raw_type,
			function,
			storage,
			tariff,
			device,
		})
	}
}
