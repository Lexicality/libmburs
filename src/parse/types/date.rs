// Copyright 2023 Lexi Robinson
// Licensed under the EUPL-1.2
#![allow(dead_code)]

use winnow::binary::bits;
use winnow::combinator::peek;
use winnow::error::StrContext;
use winnow::prelude::*;
use winnow::Bytes;

use crate::parse::application_layer::dib::RawDataType;
use crate::parse::application_layer::vib::ValueType;
use crate::parse::error::{MBResult, MBusError};
use crate::parse::error::{ParseError, Result};
use crate::parse::Datagram;

use super::BitsInput;
use super::{DataType, ParseResult};

pub fn parse_datetime(dt: RawDataType, vt: ValueType, dg: &mut Datagram) -> ParseResult {
	if let RawDataType::Binary(len) = dt {
		match vt {
			ValueType::TypeFDateTime => decode_type_f(dg.take(len)?),
			ValueType::TypeGDate => decode_type_g(dg.take(len)?),
			ValueType::TypeIDateTime => decode_type_i(dg.take(len)?),
			ValueType::TypeJTime => decode_type_j(dg.take(len)?),
			ValueType::TypeMDatetime => decode_type_m(dg.take(len)?),
			_ => Err(ParseError::DataTypeMismatch),
		}
	} else {
		Err(ParseError::DataTypeMismatch)
	}
}

fn parse_dmy(input: &mut BitsInput<'_>) -> MBResult<(u8, u8, u8)> {
	peek(
		bits::take::<_, u16, _, _>(16_usize)
			.verify(|v| *v != 0xFFFF)
			.void(),
	)
	.context(StrContext::Label("invalid check"))
	.parse_next(input)?;
	(
		// Year lower bits
		bits::take(3_usize).context(StrContext::Label("year (lower)")),
		// Day
		bits::take(5_usize)
			.context(StrContext::Label("day"))
			.verify(|v| matches!(v, 0..=31)),
		// month
		bits::take(4_usize)
			.context(StrContext::Label("month"))
			.verify(|v| matches!(v, 1..=12 | 15)),
		// Year upper bits
		bits::take(4_usize).context(StrContext::Label("year (upper)")),
	)
		.map(|(yl, day, month, yu): (u8, u8, u8, u8)| (day, month, yl + (yu << 3)))
		.verify(|(_, _, y)| matches!(y, 0..=99 | 127))
		.parse_next(input)
}

const MASK_SECOND: u8 = 0b0011_1111;
const MASK_MINUTE: u8 = 0b0011_1111;
const MASK_HOUR: u8 = 0b0001_1111;
const MASK_DAY: u8 = 0b0001_1111;
const MASK_MONTH: u8 = 0b0000_1111;
const MASK_YEAR_B1: u8 = 0b1110_0000;
const MASK_YEAR_B2: u8 = 0b1111_0000;
const MASK_INVALID: u8 = 0b1000_0000;

#[derive(Debug, PartialEq, Eq)]
pub struct TypeFDateTime {
	pub minute: u8,
	pub hour: u8,
	pub day: u8,
	pub month: u8,
	pub year: u8,
	pub hundred_year: u8,
	pub in_dst: bool,
}

impl TypeFDateTime {
	pub fn parse(input: &mut &Bytes) -> MBResult<Self> {
		bits::bits::<_, _, MBusError, _, _>((
			bits::bool
				.context(StrContext::Label("invalid bit"))
				.verify(|v| !v)
				.void(),
			bits::bool
				.context(StrContext::Label("reserved"))
				.verify(|v| !v)
				.void(),
			bits::take(6_usize)
				.context(StrContext::Label("minute"))
				.verify(|v| matches!(v, 0..=59 | 63)),
			bits::bool.context(StrContext::Label("in_dst")),
			bits::take(2_usize).context(StrContext::Label("hundred year")),
			bits::take(5_usize)
				.context(StrContext::Label("hour"))
				.verify(|v| matches!(v, 0..=23 | 31)),
			parse_dmy,
		))
		.map(
			|(_, _, minute, in_dst, hundred_year, hour, (day, month, year))| TypeFDateTime {
				minute,
				in_dst,
				hour,
				day,
				month,
				year,
				hundred_year,
			},
		)
		.parse_next(input)
	}
}

const TYPE_F_HUNDRED: u8 = 0b0110_0000;
const TYPE_F_DST: u8 = 0b1000_0000;
fn decode_type_f(data: Vec<u8>) -> ParseResult {
	if data.len() != 4 {
		return Err(ParseError::InvalidData(
			"Unsupported byte count for Type F datetime",
		));
	}

	if data[0] & MASK_INVALID != 0 {
		return Ok(DataType::Invalid(data));
	}

	let data: [u8; 4] = data
		.try_into()
		.or(Err(ParseError::DecodeError("Failed to decode datetime")))?;

	Ok(DataType::DateTimeF(TypeFDateTime {
		minute: validate_minutes(data[0] & MASK_MINUTE)?,
		hour: validate_hours(data[1] & MASK_HOUR)?,
		day: validate_day(data[2] & MASK_DAY)?,
		month: validate_month(data[3] & MASK_MONTH)?,
		year: validate_year(reconstruct_year(data[2], data[3]))?,
		hundred_year: (data[1] & TYPE_F_HUNDRED) >> 5,
		in_dst: (data[1] & TYPE_F_DST) != 0,
	}))
}

#[derive(Debug, PartialEq, Eq)]
pub struct TypeGDate {
	pub day: u8,
	pub month: u8,
	pub year: u8,
}

impl TypeGDate {
	pub fn parse(input: &mut &Bytes) -> MBResult<Self> {
		bits::bits(parse_dmy)
			.map(|(day, month, year)| TypeGDate { day, month, year })
			.parse_next(input)
	}
}

fn decode_type_g(data: Vec<u8>) -> ParseResult {
	if data.len() != 2 {
		return Err(ParseError::InvalidData(
			"Unsupported byte count for Type G date",
		));
	}

	if data == [0xFF, 0xFF] {
		return Ok(DataType::Invalid(data));
	}

	let data: [u8; 2] = data
		.try_into()
		.or(Err(ParseError::DecodeError("Failed to decode date")))?;

	// pass
	Ok(DataType::Date(TypeGDate {
		day: validate_day(data[0] & MASK_DAY)?,
		month: validate_month(data[1] & MASK_MONTH)?,
		year: validate_year(reconstruct_year(data[0], data[1]))?,
	}))
}

#[derive(Debug, PartialEq, Eq)]
pub struct TypeIDateTime {
	pub second: u8,
	pub minute: u8,
	pub hour: u8,
	pub day: u8,
	pub month: u8,
	pub year: u8,
	pub day_of_week: u8,
	pub week: u8,
	pub in_dst: bool,
	pub leap_year: bool,
	pub dst_offset: i8,
}

impl TypeIDateTime {
	pub fn parse(input: &mut &Bytes) -> MBResult<Self> {
		bits::bits((
			bits::bool.context(StrContext::Label("leap year")),
			bits::bool.context(StrContext::Label("in dst")),
			bits::take(6_usize)
				.context(StrContext::Label("second"))
				.verify(|v| matches!(v, 0..=59 | 63)),
			bits::bool
				.context(StrContext::Label("invalid check"))
				.verify(|v| !v)
				.void(),
			bits::bool.context(StrContext::Label("dst ±")),
			bits::take(6_usize)
				.context(StrContext::Label("minute"))
				.verify(|v| matches!(v, 0..=59 | 63)),
			bits::take(3_usize).context(StrContext::Label("day of week")),
			bits::take(5_usize)
				.context(StrContext::Label("hour"))
				.verify(|v| matches!(v, 0..=23 | 31)),
			parse_dmy,
			bits::take(2_usize)
				.context(StrContext::Label("dst offset"))
				.try_map(|v: u8| v.try_into()),
			bits::take(6_usize)
				.context(StrContext::Label("dst offset"))
				.verify(|v| matches!(v, 0..=53)),
		))
		.map(
			|(
				leap_year,
				in_dst,
				second,
				_,
				dst_plus,
				minute,
				day_of_week,
				hour,
				(day, month, year),
				dst_offset,
				week,
			)| TypeIDateTime {
				second,
				minute,
				hour,
				day,
				month,
				year,
				day_of_week,
				week,
				in_dst,
				leap_year,
				dst_offset: if dst_plus { dst_offset } else { -dst_offset },
			},
		)
		.parse_next(input)
	}
}

const TYPE_I_DST: u8 = 0b0100_0000;
const TYPE_I_DST_OFFSET_DIR: u8 = 0b0100_0000;
const TYPE_I_DST_OFFSET_AMT: u8 = 0b1100_0000;
const TYPE_I_LEAPYEAR: u8 = 0b1000_0000;
const TYPE_I_WEEKDAY: u8 = 0b1110_0000;
const TYPE_I_WEEKNUM: u8 = 0b0011_1111;
fn decode_type_i(data: Vec<u8>) -> ParseResult {
	if data.len() != 6 {
		return Err(ParseError::InvalidData(
			"Unsupported byte count for Type I datetime",
		));
	}

	if data[0] & MASK_INVALID != 0 {
		return Ok(DataType::Invalid(data));
	}

	let data: [u8; 6] = data
		.try_into()
		.or(Err(ParseError::DecodeError("Failed to decode datetime")))?;

	let mut offset = ((data[5] & TYPE_I_DST_OFFSET_AMT) >> 6) as i8;
	if offset > 0 && (data[1] & TYPE_I_DST_OFFSET_DIR) == 0 {
		offset *= -1;
	}

	Ok(DataType::DateTimeI(TypeIDateTime {
		second: validate_seconds(data[0] & MASK_SECOND)?,
		minute: validate_minutes(data[1] & MASK_MINUTE)?,
		hour: validate_hours(data[2] & MASK_HOUR)?,
		day: validate_day(data[3] & MASK_DAY)?,
		month: validate_month(data[4] & MASK_MONTH)?,
		year: validate_year(reconstruct_year(data[3], data[4]))?,
		day_of_week: validate_weekday((data[2] & TYPE_I_WEEKDAY) >> 5)?,
		week: validate_week(data[5] & TYPE_I_WEEKNUM)?,
		leap_year: (data[0] & TYPE_I_LEAPYEAR) != 0,
		in_dst: (data[0] & TYPE_I_DST) != 0,
		dst_offset: offset,
	}))
}

#[derive(Debug, PartialEq, Eq)]
pub struct TypeJTime {
	pub second: u8,
	pub minute: u8,
	pub hour: u8,
}

impl TypeJTime {
	pub fn parse(input: &mut &Bytes) -> MBResult<Self> {
		bits::bits::<_, _, MBusError, _, _>((
			bits::take::<_, u8, _, _>(2_usize)
				.context(StrContext::Label("padding"))
				.verify(|v| *v == 0)
				.void(),
			bits::take(6_usize)
				.context(StrContext::Label("second"))
				.verify(|v| matches!(v, 0..=59 | 63)),
			bits::take::<_, u8, _, _>(2_usize)
				.context(StrContext::Label("padding"))
				.verify(|v| *v == 0)
				.void(),
			bits::take(6_usize)
				.context(StrContext::Label("minute"))
				.verify(|v| matches!(v, 0..=59 | 63)),
			bits::take::<_, u8, _, _>(3_usize)
				.context(StrContext::Label("padding"))
				.verify(|v| *v == 0)
				.void(),
			bits::take(5_usize)
				.context(StrContext::Label("hour"))
				.verify(|v| matches!(v, 0..=23 | 31)),
		))
		.map(|(_, second, _, minute, _, hour)| Self {
			second,
			minute,
			hour,
		})
		.parse_next(input)
	}
}

fn decode_type_j(data: Vec<u8>) -> ParseResult {
	if data.len() != 3 {
		return Err(ParseError::InvalidData(
			"Unsupported byte count for Type J time",
		));
	}

	// My copy of EN 13757–3:2018 says that 0x0000000 is valid but notes that
	//  EN 13757–3:2013 would consider it sentinel invalid. I'm not sure if I
	//  should try to deal with that case or hope for the best
	if data == [0xFF, 0xFF, 0xFF] {
		return Ok(DataType::Invalid(data));
	}

	let data: [u8; 4] = data
		.try_into()
		.or(Err(ParseError::DecodeError("Failed to decode time")))?;

	Ok(DataType::Time(TypeJTime {
		second: validate_seconds(data[0] & MASK_SECOND)?,
		minute: validate_minutes(data[1] & MASK_MINUTE)?,
		hour: validate_hours(data[2] & MASK_HOUR)?,
	}))
}

fn decode_type_m(_data: Vec<u8>) -> ParseResult {
	todo!("Pull requests welcome")
}

/// Conveniently the year is always in the same bytes on every datatype so we can
/// use one function to mask and combine
fn reconstruct_year(byte1: u8, byte2: u8) -> u8 {
	((byte1 & MASK_YEAR_B1) >> 1) | ((byte2 & MASK_YEAR_B2) >> 4)
}

fn validate_seconds(data: u8) -> Result<u8> {
	match data {
		0..=59 | 63 => Ok(data),
		_ => Err(ParseError::DecodeError("Unexpected value for seconds")),
	}
}

fn validate_minutes(data: u8) -> Result<u8> {
	match data {
		0..=59 | 63 => Ok(data),
		_ => Err(ParseError::DecodeError("Unexpected value for minutes")),
	}
}

fn validate_hours(data: u8) -> Result<u8> {
	match data {
		0..=23 | 31 => Ok(data),
		_ => Err(ParseError::DecodeError("Unexpected value for hours")),
	}
}

fn validate_day(data: u8) -> Result<u8> {
	match data {
		0..=31 => Ok(data),
		_ => Err(ParseError::DecodeError("Unexpected value for day")),
	}
}

fn validate_weekday(data: u8) -> Result<u8> {
	match data {
		0..=7 => Ok(data),
		_ => Err(ParseError::DecodeError("Unexpected value for weekday")),
	}
}
fn validate_week(data: u8) -> Result<u8> {
	match data {
		0..=53 => Ok(data),
		_ => Err(ParseError::DecodeError("Unexpected value for week")),
	}
}

fn validate_month(data: u8) -> Result<u8> {
	match data {
		// Technically some formats say 0 is ok and others say 15 is ok but none
		// say both, however, I don't care
		0..=12 | 15 => Ok(data),
		_ => Err(ParseError::DecodeError("Unexpected value for month")),
	}
}

fn validate_year(data: u8) -> Result<u8> {
	match data {
		0..=99 | 127 => Ok(data),
		_ => Err(ParseError::DecodeError("Unexpected value for year")),
	}
}
