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
	(
		peek(bits::take::<_, u16, _, _>(16_usize))
			.verify(|v| *v != 0xFFFF)
			.context(StrContext::Label("invalid check"))
			.void(),
		// Year upper bits
		bits::take(3_usize).context(StrContext::Label("year (upper)")),
		// Day
		bits::take(5_usize)
			.verify(|v| matches!(v, 0..=31))
			.context(StrContext::Label("day")),
		// Year lower bits
		bits::take(4_usize).context(StrContext::Label("year (lower)")),
		// month
		bits::take(4_usize)
			.verify(|v| {
				matches!(
					v,
					// NOTE: This should be 1..=12 but the libmbus test data has
					// invalid dates in the following files:
					// ACW_Itron-BM-plus-m.hex
					// itron_bm_+m.hex
					// siemens_water.hex
					// siemens_wfh21.hex
					0..=12 | 15
				)
			})
			.context(StrContext::Label("month")),
	)
		.map(|(_, yu, day, yl, month): ((), u8, u8, u8, u8)| (day, month, yu + (yl << 3)))
		.verify(|(_, _, y)| matches!(y, 0..=99 | 127))
		.context(StrContext::Label("year"))
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
		bits::bits((
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
			|(
				_,
				_,
				minute,
				in_dst,
				mut hundred_year,
				hour,
				(day, month, year),
				//
			)| {
				// EN 13757-3:2018 Annex A table A.5 footnote a:
				// "For compatibility with old meters with a circular two digit
				// date it is recommended to consider in any master software the
				// years “00” to “80” as the years 2000 to 2080.""
				if hundred_year == 0 && year <= 80 {
					hundred_year = 1;
				}
				TypeFDateTime {
					minute,
					in_dst,
					hour,
					day,
					month,
					year,
					hundred_year,
				}
			},
		)
		.parse_next(input)
	}
}

#[cfg(test)]
mod test_type_f_date_time {
	use rstest::rstest;
	use winnow::prelude::*;
	use winnow::Bytes;

	use crate::parse::error::MBusContext;

	use super::TypeFDateTime;

	#[rstest]
	#[case::ACW_Itron_BM_plus_m__0([0x0B, 0x0B, 0xCD, 0x13], TypeFDateTime{
		hundred_year: 1,
		year: 14,
		in_dst: false,
		month: 3,
		day: 13,
		hour: 11,
		minute: 11,
	})]
	#[case::amt_calec_mb([0x10, 0x09, 0x05, 0xC5], TypeFDateTime{
		hundred_year: 0,
		year: 96,
		in_dst: false,
		month: 5,
		day: 5,
		hour: 9,
		minute: 16,
	})]
	#[case::kamstrup_multical_601([0x1A, 0x2F, 0x65, 0x11], TypeFDateTime{
		hundred_year: 1,
		year: 11,
		month: 1,
		day: 5,
		hour: 15,
		minute: 26,
		in_dst: false,
	})]
	#[allow(non_snake_case)]
	fn test_file_values(#[case] input: [u8; 4], #[case] expected: TypeFDateTime) {
		let input = Bytes::new(&input);

		let result = TypeFDateTime::parse.parse(input).unwrap();

		assert_eq!(result, expected);
	}

	#[rstest]
	#[case::REL_Relay_Padpuls2([0xA1, 0x15, 0xE9, 0x17], "invalid bit")]
	#[case::invalid_bit([0b1000_0000, 0x00, 0x01, 0x01], "invalid bit")]
	#[case::reserved_bit([0b0100_0000, 0x00, 0x01, 0x01], "reserved")]
	#[case::invalid_minute([0x3C, 0x00, 0x01, 0x01], "minute")]
	#[case::invalid_hour([0x00, 0x18, 0x01, 0x01], "hour")]
	#[case::invalid_month([0x00, 0x00, 0b111_00001, 0b0000_1101], "month")]
	#[case::invalid_year([0x00, 0x00, 0b100_00001, 0b1100_0001], "year")]
	#[allow(non_snake_case)]
	fn test_validation(#[case] input: [u8; 4], #[case] context: &'static str) {
		let input = Bytes::new(&input);

		let result = TypeFDateTime::parse.parse(input).unwrap_err();

		let err = result.inner();
		assert_eq!(err.context().next(), Some(&MBusContext::Label(context)));
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

#[cfg(test)]
mod test_type_g_date {
	use rstest::rstest;
	use winnow::prelude::*;
	use winnow::Bytes;

	use crate::parse::error::MBusContext;

	use super::TypeGDate;

	#[rstest]
	#[case::allmess_cf50([0x8C, 0x11], [12, 1, 12])]
	#[case::EFE_Engelmann_WaterStar__0([0xBF, 0x1C], [13, 12, 31])]
	#[case::EFE_Engelmann_WaterStar__1([0xDF, 0x1C], [14, 12, 31])]
	#[case::minol_minocal_c2__0([0x81, 0x11], [12, 1, 1])]
	#[case::minol_minocal_c2__8([0x61, 0x16], [11, 6, 1])]
	#[case::minol_minocal_c2__9([0x81, 0x11], [12, 1, 1])]
	#[case::ZRM_Minol_Minocal_C2__6([0xA1, 0x1A], [13, 10, 1])]
	#[case::REL_Relay_Padpuls2__0([0xDF, 0x1C], [14, 12, 31])]
	#[case::REL_Relay_Padpuls2__1([0xFF, 0x1C], [15, 12, 31])]
	#[case::rel_padpuls2__0([0x1F, 0x0C], [0, 12, 31])]
	#[case::rel_padpuls2__1([0x3F, 0x0C], [1, 12, 31])]
	#[case::ACW_Itron_BM_plus_m__0([0x00, 0x00], [0, 0, 0])] // :/
	#[allow(non_snake_case)]
	fn test_file_values(#[case] input: [u8; 2], #[case] output: [u8; 3]) {
		let input = Bytes::new(&input);
		let [year, month, day] = output;

		let result = TypeGDate::parse.parse(input).unwrap();

		assert_eq!(result.day, day, "days must match");
		assert_eq!(result.month, month, "months must match");
		assert_eq!(result.year, year, "years must match");
	}

	#[test]
	fn test_explicit_invalid_value() {
		let input = Bytes::new(&[0xFF, 0xFF]);

		let result = TypeGDate::parse.parse(input).unwrap_err();

		let err = result.inner();
		assert_eq!(
			err.context().next(),
			Some(&MBusContext::Label("invalid check"))
		);
	}

	#[rstest]
	#[case::month_13([0b111_00001, 0b0000_1101], "month")]
	#[case::month_14([0b111_00001, 0b0000_1110], "month")]
	#[case::year_100([0b100_00001, 0b1100_0001], "year")]
	#[case::year_126([0b110_00001, 0b1111_0001], "year")]
	fn test_validation(#[case] input: [u8; 2], #[case] context: &'static str) {
		let input = Bytes::new(&input);

		let result = TypeGDate::parse.parse(input).unwrap_err();

		let err = result.inner();
		assert_eq!(err.context().next(), Some(&MBusContext::Label(context)));
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
			peek(bits::take::<_, u32, _, _>(24_usize))
				.verify(|v| *v != 0xFFFFFF)
				.context(StrContext::Label("invalid check"))
				.void(),
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
		.map(|(_, _, second, _, minute, _, hour)| Self {
			second,
			minute,
			hour,
		})
		.parse_next(input)
	}
}

#[cfg(test)]
mod test_type_j_time {
	use rstest::rstest;
	use winnow::prelude::*;
	use winnow::Bytes;

	use crate::parse::error::MBusContext;

	use super::TypeJTime;
	#[rstest]
	#[case::zero([0, 0, 0], TypeJTime{hour: 0, minute: 0, second: 0})]
	#[case::max_hours([0, 0, 23], TypeJTime{hour: 23, minute: 0, second: 0})]
	#[case::max_mins([0, 59, 0], TypeJTime{hour: 0, minute: 59, second: 0})]
	#[case::max_secs([59, 0, 0], TypeJTime{hour: 0, minute: 0, second: 59})]
	#[case::all_the_time([63, 63, 31], TypeJTime{hour: 31, minute: 63, second: 63})]
	fn test_works(#[case] input: [u8; 3], #[case] expected: TypeJTime) {
		let input = Bytes::new(&input);

		let result = TypeJTime::parse.parse(input).unwrap();

		assert_eq!(result, expected);
	}

	#[rstest]
	fn test_padding(
		// It's "great" how simple it is to generate 64 tests with such a simple
		// block of code, right?
		#[values(0b00, 0b01, 0b10, 0b11)] first_byte: u8,
		#[values(0b00, 0b01, 0b10, 0b11)] second_byte: u8,
		#[values(0b00, 0b01, 0b10, 0b11)] third_byte: u8,
	) {
		// If all of them are 0, the value is actually valid
		if first_byte == 0 && second_byte == 0 && third_byte == 0 {
			return;
		}

		let input = [first_byte << 6, second_byte << 6, third_byte << 6];
		let input = Bytes::new(&input);

		let result = TypeJTime::parse.parse(input).unwrap_err();

		let err = result.inner();
		assert_eq!(err.context().next(), Some(&MBusContext::Label("padding")));
	}

	#[rstest]
	#[case::invalid([0xFF, 0xFF, 0xFF], "invalid check")]
	#[case::max_hours([0, 0, 24], "hour")]
	#[case::max_mins([0, 60, 0], "minute")]
	#[case::max_secs([60, 0, 0], "second")]
	fn test_validation(#[case] input: [u8; 3], #[case] context: &'static str) {
		let input = Bytes::new(&input);

		let result = TypeJTime::parse.parse(input).unwrap_err();

		let err = result.inner();
		assert_eq!(err.context().next(), Some(&MBusContext::Label(context)));
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
