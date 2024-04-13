// Copyright 2023 Lexi Robinson
// Licensed under the EUPL-1.2
#![allow(dead_code)]

use winnow::binary::bits;
use winnow::combinator::peek;
use winnow::error::StrContext;
use winnow::prelude::*;
use winnow::Bytes;

use crate::parse::error::{MBResult, MBusError};

use super::BitsInput;

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
