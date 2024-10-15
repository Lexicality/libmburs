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
		bits::take(3_usize).context(StrContext::Label("year (upper)")),
		// Day
		bits::take(5_usize)
			.verify(|v| matches!(v, 0..=31))
			.context(StrContext::Label("day")),
		// month
		bits::take(4_usize)
			.verify(|v| matches!(v, 1..=12 | 15))
			.context(StrContext::Label("month")),
		// Year upper bits
		bits::take(4_usize).context(StrContext::Label("year (lower)")),
	)
		.map(|(yu, day, month, yl): (u8, u8, u8, u8)| (day, month, yl + (yu << 3)))
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
				.verify(|v| !v)
				.context(StrContext::Label("invalid bit"))
				.void(),
			bits::bool
				.verify(|v| !v)
				.context(StrContext::Label("reserved"))
				.void(),
			bits::take(6_usize)
				.verify(|v| matches!(v, 0..=59 | 63))
				.context(StrContext::Label("minute")),
			bits::bool.context(StrContext::Label("in_dst")),
			bits::take(2_usize).context(StrContext::Label("hundred year")),
			bits::take(5_usize)
				.verify(|v| matches!(v, 0..=23 | 31))
				.context(StrContext::Label("hour")),
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
				.verify(|v| matches!(v, 0..=59 | 63))
				.context(StrContext::Label("second")),
			bits::bool
				.verify(|v| !v)
				.context(StrContext::Label("invalid check"))
				.void(),
			bits::bool.context(StrContext::Label("dst ±")),
			bits::take(6_usize)
				.verify(|v| matches!(v, 0..=59 | 63))
				.context(StrContext::Label("minute")),
			bits::take(3_usize).context(StrContext::Label("day of week")),
			bits::take(5_usize)
				.verify(|v| matches!(v, 0..=23 | 31))
				.context(StrContext::Label("hour")),
			parse_dmy,
			bits::take(2_usize)
				.try_map(|v: u8| v.try_into())
				.context(StrContext::Label("dst offset")),
			bits::take(6_usize)
				.verify(|v| matches!(v, 0..=53))
				.context(StrContext::Label("dst offset")),
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
				.verify(|v| *v == 0)
				.context(StrContext::Label("padding"))
				.void(),
			bits::take(6_usize)
				.verify(|v| matches!(v, 0..=59 | 63))
				.context(StrContext::Label("second")),
			bits::take::<_, u8, _, _>(2_usize)
				.verify(|v| *v == 0)
				.context(StrContext::Label("padding"))
				.void(),
			bits::take(6_usize)
				.verify(|v| matches!(v, 0..=59 | 63))
				.context(StrContext::Label("minute")),
			bits::take::<_, u8, _, _>(3_usize)
				.verify(|v| *v == 0)
				.context(StrContext::Label("padding"))
				.void(),
			bits::take(5_usize)
				.verify(|v| matches!(v, 0..=23 | 31))
				.context(StrContext::Label("hour")),
		))
		.map(|(_, second, _, minute, _, hour)| Self {
			second,
			minute,
			hour,
		})
		.parse_next(input)
	}
}

#[derive(Debug, PartialEq, Eq)]
pub struct TypeKDST {
	pub starts_hour: u8,
	pub starts_day: u8,
	pub starts_month: u8,
	pub ends_day: u8,
	pub ends_month: u8,
	pub enable: bool,
	pub dst_deviation: i8,
	pub local_deviation: u8,
}

impl TypeKDST {
	pub fn parse(input: &mut &Bytes) -> MBResult<Self> {
		bits::bits::<_, _, MBusError, _, _>((
			// byte 1
			bits::take(3_usize).context(StrContext::Label("gmt deviation upper")),
			bits::take(5_usize)
				.verify(|v| matches!(v, 0..=23 | 31))
				.context(StrContext::Label("hour begins")),
			// byte 2
			bits::bool.context(StrContext::Label("enable")),
			bits::take(2_usize).context(StrContext::Label("gmt deviation lower")),
			bits::take(5_usize)
				.verify(|v| matches!(v, 1..=31))
				.context(StrContext::Label("day begins")),
			// byte 3
			bits::bool.context(StrContext::Label("dst ±")),
			bits::take(2_usize)
				.try_map(|v: u8| v.try_into())
				.context(StrContext::Label("dst deviation hours")),
			bits::take(5_usize)
				.verify(|v| matches!(v, 1..=31))
				.context(StrContext::Label("day ends")),
			// byte 4
			bits::take(4_usize)
				.verify(|v| matches!(v, 1..=12))
				.context(StrContext::Label("month ends")),
			bits::take(4_usize)
				.verify(|v| matches!(v, 1..=12))
				.context(StrContext::Label("month begins")),
		))
		.map(
			|(
				gmt_u,
				starts_hour,
				enable,
				gmt_l,
				starts_day,
				dst_plus,
				dst_deviation,
				ends_day,
				ends_month,
				starts_month,
			): (u8, u8, bool, u8, u8, bool, i8, u8, u8, u8)| Self {
				starts_hour,
				starts_day,
				starts_month,
				ends_day,
				ends_month,
				enable,
				dst_deviation: if dst_plus {
					dst_deviation
				} else {
					-dst_deviation
				},
				local_deviation: gmt_l + (gmt_u << 3),
			},
		)
		.verify(|v| matches!(v.local_deviation, 0..=23 | 31))
		.parse_next(input)
	}
}
