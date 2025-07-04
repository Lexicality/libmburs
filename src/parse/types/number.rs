// Copyright 2023 Lexi Robinson
// Licensed under the EUPL-1.2

use core::iter::IntoIterator;

use winnow::binary;
use winnow::combinator::{alt, peek, repeat};
use winnow::error::{ParserError, StrContext};
use winnow::prelude::*;
use winnow::token::take;
use winnow::Bytes;

use crate::parse::error::{MBResult, MBusContext, MBusError};

use super::BitsInput;

fn parse_nibble(input: &mut BitsInput<'_>) -> MBResult<i64> {
	binary::bits::take(4_usize).parse_next(input)
}

fn parse_bcd_nibble(input: &mut BitsInput<'_>) -> MBResult<i64> {
	parse_nibble
		.verify(|v| *v < 10)
		.context(MBusContext::Label("BCD nibble"))
		.parse_next(input)
}

pub fn parse_bcd<'a>(bytes: usize) -> impl Parser<&'a Bytes, i64, MBusError> {
	let parser = move |input: &mut BitsInput<'a>| {
		if bytes == 0 {
			return Ok(0);
		} else if bytes > 9 {
			return Err(MBusError::assert(
				input,
				"cannot safely parse more than 9 bytes",
			));
		}
		let mut initial_bytes: Vec<i64> = repeat(
			bytes - 1,
			(parse_bcd_nibble, parse_bcd_nibble).map(|(hi, lo)| hi * 10 + lo),
		)
		.context(StrContext::Label("initial bytes"))
		.parse_next(input)?;

		// last byte
		let (mut high, low) = (
			alt((parse_nibble.verify(|v| *v == 0x0F), parse_bcd_nibble)),
			parse_bcd_nibble,
		)
			.context(StrContext::Label("final byte"))
			.parse_next(input)?;

		let neg = high == 0x0F;
		if neg {
			high = 0;
		}
		initial_bytes.push(high * 10 + low);

		let result = initial_bytes
			.into_iter()
			.rev()
			.reduce(|acc, value| acc * 100 + value)
			.unwrap_or_default();

		Ok(if neg { -result } else { result })
	};

	(peek(take(bytes)).void(), binary::bits::bits(parser))
		.map(|(_, n)| n)
		.context_with(move || {
			[MBusContext::ComputedLabel(format!(
				"signed {bytes} byte BCD number"
			))]
			.into_iter()
		})
}

#[cfg(test)]
mod test_parse_bcd {
	use rstest::rstest;
	use winnow::{Bytes, Parser};

	use super::parse_bcd;

	#[test]
	fn test_basic_unsigned() {
		let input = Bytes::new(&[0x12]);

		let result = parse_bcd(1).parse(input).unwrap();

		assert_eq!(result, 12);
	}

	#[test]
	fn test_byte_order_unsigned() {
		let input = Bytes::new(&[0x34, 0x12]);

		let result = parse_bcd(2).parse(input).unwrap();

		assert_eq!(result, 1234);
	}

	#[test]
	fn test_maximum_lvar_unsigned() {
		let input = Bytes::new(&[0x99; 9]);

		let result = parse_bcd(9).parse(input).unwrap();

		assert_eq!(result, 999_999_999_999_999_999);
	}

	#[test]
	fn test_basic_signed() {
		let input = Bytes::new(&[0xF1]);

		let result = parse_bcd(1).parse(input).unwrap();

		assert_eq!(result, -1);
	}

	#[test]
	fn test_byte_order_signed() {
		let input = Bytes::new(&[0x23, 0xF1]);

		let result = parse_bcd(2).parse(input).unwrap();

		assert_eq!(result, -123);
	}

	#[test]
	fn test_maximum_lvar_signed() {
		let mut data = [0x99; 9];
		data[8] = 0xF9;
		let input = Bytes::new(&data);

		let result = parse_bcd(9).parse(input).unwrap();

		assert_eq!(result, -99_999_999_999_999_999);
	}

	#[test]
	fn test_negative_zero() {
		let input = Bytes::new(&[0xF0]);

		let result = parse_bcd(1).parse(input).unwrap();

		assert_eq!(result, 0);
	}

	#[test]
	fn test_parse_zero() {
		let input = Bytes::new(&[]);

		let result = parse_bcd(0).parse(input).unwrap();

		assert_eq!(result, 0);
	}

	#[test]
	#[should_panic(expected = "cannot safely parse more than 9 bytes")]
	fn test_parse_ten() {
		let input = Bytes::new(&[0; 10]);

		let _ = parse_bcd(10).parse(input);
	}

	#[test]
	fn test_parse_not_enough_data_1() {
		let input = Bytes::new(&[0x12]);

		let result = parse_bcd(3).parse(input).unwrap_err();

		let ctx = result.inner().context().next().unwrap().to_string();

		assert_eq!(ctx, "invalid signed 3 byte BCD number");
	}

	#[test]
	fn test_parse_not_enough_data_2() {
		let input = Bytes::new(&[0x12]);

		let result = parse_bcd(2).parse(input).unwrap_err();

		let ctx = result.inner().context().next().unwrap().to_string();

		assert_eq!(ctx, "invalid signed 2 byte BCD number");
	}

	#[rstest]
	#[case(&[0xAA], 1)]
	#[case(&[0xBB], 1)]
	#[case(&[0xCC], 1)]
	#[case(&[0xDD], 1)]
	#[case(&[0xEE], 1)]
	#[case(&[0xFF], 1)]
	// 0xF0 is valid but 0x0F is not
	#[case(&[0x0F], 1)]
	#[case(&[0xAA, 0x00], 2)]
	#[case(&[0xBB, 0x00], 2)]
	#[case(&[0xCC, 0x00], 2)]
	#[case(&[0xDD, 0x00], 2)]
	#[case(&[0xEE, 0x00], 2)]
	#[case(&[0xFF, 0x00], 2)]
	#[case(&[0x0F, 0x00], 2)]
	// 0xF0 is only valid in the last byte
	#[case(&[0xF0, 0x00], 2)]
	#[case(&[0x00, 0xAA], 2)]
	#[case(&[0x00, 0xBB], 2)]
	#[case(&[0x00, 0xCC], 2)]
	#[case(&[0x00, 0xDD], 2)]
	#[case(&[0x00, 0xEE], 2)]
	#[case(&[0x00, 0xFF], 2)]
	fn test_parse_garbage(#[case] bytes: &'static [u8], #[case] byte_count: usize) {
		let input = Bytes::new(bytes);

		let result = parse_bcd(byte_count).parse(input).unwrap_err();

		let ctx = result.inner().context().next().unwrap().to_string();

		assert_eq!(ctx, "invalid BCD nibble");
	}
}

fn parse_hex_nibble(input: &mut BitsInput<'_>) -> MBResult<char> {
	binary::bits::take(4_usize)
		.verify_map(|i: u32| char::from_digit(i, 16))
		.parse_next(input)
}

pub fn parse_invalid_bcd<'a>(bytes: usize) -> impl Parser<&'a Bytes, String, MBusError> {
	let parser = move |input: &mut BitsInput<'a>| {
		if bytes == 0 {
			return Ok::<_, MBusError>("".to_owned());
		}
		let mut initial_bytes: Vec<(char, char)> =
			repeat(bytes - 1, (parse_hex_nibble, parse_hex_nibble))
				.context(StrContext::Label("initial bytes"))
				.parse_next(input)?;

		// last byte is speical because of the `-` behaviour
		initial_bytes.push(
			(
				parse_hex_nibble.map(|c| if c == 'f' { '-' } else { c }),
				parse_hex_nibble,
			)
				.context(StrContext::Label("final byte"))
				.parse_next(input)?,
		);

		let result: String = initial_bytes
			.into_iter()
			.rev()
			.flat_map(|i| [i.0, i.1])
			.collect();

		Ok(result.to_uppercase())
	};

	binary::bits::bits(parser).context(StrContext::Label("signed BCD number"))
}

#[cfg(test)]
mod test_parse_invalid_bcd {
	use winnow::{Bytes, Parser};

	use super::parse_invalid_bcd;

	#[test]
	fn test_basic_unsigned() {
		let input = Bytes::new(&[0x12]);

		let result = parse_invalid_bcd(1).parse(input).unwrap();

		assert_eq!(result, "12");
	}

	#[test]
	fn test_byte_order_unsigned() {
		let input = Bytes::new(&[0x34, 0x12]);

		let result = parse_invalid_bcd(2).parse(input).unwrap();

		assert_eq!(result, "1234");
	}

	#[test]
	fn test_basic_signed() {
		let input = Bytes::new(&[0xF1]);

		let result = parse_invalid_bcd(1).parse(input).unwrap();

		assert_eq!(result, "-1");
	}

	#[test]
	fn test_byte_order_signed() {
		let input = Bytes::new(&[0x23, 0xF1]);

		let result = parse_invalid_bcd(2).parse(input).unwrap();

		assert_eq!(result, "-123");
	}

	#[test]
	fn test_negative_zero() {
		let input = Bytes::new(&[0xF0]);

		let result = parse_invalid_bcd(1).parse(input).unwrap();

		assert_eq!(result, "-0");
	}

	#[test]
	fn test_parse_zero() {
		let input = Bytes::new(&[]);

		let result = parse_invalid_bcd(0).parse(input).unwrap();

		assert_eq!(result, "");
	}

	#[test]
	fn test_parse_not_enough_data_1() {
		let input = Bytes::new(&[0x12]);

		let result = parse_invalid_bcd(3).parse(input).unwrap_err();

		let ctx = result.inner().context().next().unwrap().to_string();

		assert_eq!(ctx, "invalid initial bytes");
	}

	#[test]
	fn test_parse_not_enough_data_2() {
		let input = Bytes::new(&[0x12]);

		let result = parse_invalid_bcd(2).parse(input).unwrap_err();

		let ctx = result.inner().context().next().unwrap().to_string();

		assert_eq!(ctx, "invalid final byte");
	}

	#[test]
	fn test_hex() {
		let input = Bytes::new(&[0xEF, 0xCD, 0xAB]);

		let result = parse_invalid_bcd(3).parse(input).unwrap();

		assert_eq!(result, "ABCDEF");
	}

	#[test]
	fn test_negative_hex() {
		let input = Bytes::new(&[0xFF]);

		let result = parse_invalid_bcd(1).parse(input).unwrap();

		assert_eq!(result, "-F");
	}
}

pub fn parse_binary_signed<'a>(bytes: usize) -> impl Parser<&'a Bytes, i64, MBusError> {
	(move |input: &mut &'a Bytes| {
		if bytes > 8 {
			return Err(MBusError::assert(input, "cannot parse more than 8 bytes"));
		}
		match bytes {
			0 => Ok(0),
			1 => binary::i8.map(|i| i.into()).parse_next(input),
			2 => binary::le_i16.map(|i| i.into()).parse_next(input),
			4 => binary::le_i32.map(|i| i.into()).parse_next(input),
			8 => binary::le_i64.parse_next(input),
			n => take(n)
				.map(|raw_bytes: &[u8]| {
					let offset = 8 - n;
					let mut data = [0; 8];
					for (i, byte) in raw_bytes.iter().enumerate() {
						data[i + offset] = *byte;
					}
					let res = i64::from_le_bytes(data);
					res >> (offset * 8)
				})
				.parse_next(input),
		}
	})
	.context_with(move || {
		[MBusContext::ComputedLabel(format!(
			"{} bit signed number",
			bytes * 8
		))]
		.into_iter()
	})
}

#[cfg(test)]
mod test_parse_binary_signed {
	use super::parse_binary_signed;
	use winnow::{Bytes, Parser};

	#[test]
	fn test_i8() {
		for i in i8::MIN..=i8::MAX {
			let bytes = i.to_le_bytes();
			let input = Bytes::new(&bytes);
			let result = parse_binary_signed(1).parse(input).unwrap();
			assert_eq!(result, i.into());
		}
	}

	#[test]
	fn test_i16() {
		for i in i16::MIN..=i16::MAX {
			let bytes = i.to_le_bytes();
			let input = Bytes::new(&bytes);
			let result = parse_binary_signed(2).parse(input).unwrap();
			assert_eq!(result, i.into());
		}
	}

	#[test]
	fn test_i32() {
		for i in [i32::MIN, -200, 0, 200, i32::MAX] {
			let bytes = i.to_le_bytes();
			let input = Bytes::new(&bytes);
			let result = parse_binary_signed(4).parse(input).unwrap();
			assert_eq!(result, i.into());
		}
	}

	#[test]
	fn test_i64() {
		for i in [
			i64::MIN,
			i32::MIN.into(),
			i16::MIN.into(),
			0,
			i16::MAX.into(),
			i32::MAX.into(),
			i64::MAX,
		] {
			let bytes = i.to_le_bytes();
			let input = Bytes::new(&bytes);
			let result = parse_binary_signed(8).parse(input).unwrap();
			assert_eq!(result, i);
		}
	}

	const I24_BASE: i32 = 2_i32.pow(23);
	const I24_MIN: i32 = -I24_BASE;
	const I24_MAX: i32 = I24_BASE - 1;

	#[test]
	fn test_i24() {
		for i in [I24_MIN, i16::MIN.into(), 0, i16::MAX.into(), I24_MAX] {
			let raw_bytes = i.to_le_bytes();
			let bytes = &raw_bytes[..3];
			let input = Bytes::new(bytes);
			let result = parse_binary_signed(3).parse(input).unwrap();
			assert_eq!(
				result,
				i.into(),
				"Should be able to parse {i} from bytes {bytes:x?}",
			);
		}
	}

	const I48_BASE: i64 = 2_i64.pow(47);
	const I48_MIN: i64 = -I48_BASE;
	const I48_MAX: i64 = I48_BASE - 1;

	#[test]
	fn test_i48() {
		for i in [
			I48_MIN,
			I24_MIN.into(),
			i16::MIN.into(),
			0,
			i16::MAX.into(),
			I24_MAX.into(),
			I48_MAX,
		] {
			let raw_bytes = i.to_le_bytes();
			let bytes = &raw_bytes[..6];
			let input = Bytes::new(bytes);
			let result = parse_binary_signed(6).parse(input).unwrap();
			assert_eq!(
				result, i,
				"Should be able to parse {i} from bytes {bytes:x?}",
			);
		}
	}

	#[test]
	fn test_parse_zero() {
		let input = Bytes::new(&[]);

		let result = parse_binary_signed(0).parse(input).unwrap();

		assert_eq!(result, 0);
	}

	#[test]
	#[should_panic(expected = "cannot parse more than 8 bytes")]
	fn test_parse_ten() {
		let input = Bytes::new(&[0; 9]);

		let _ = parse_binary_signed(9).parse(input);
	}

	#[test]
	fn test_parse_not_enough_data_1() {
		let input = Bytes::new(&[0x12]);

		let result = parse_binary_signed(2).parse(input).unwrap_err();

		let ctx = result.inner().context().next().unwrap().to_string();

		assert_eq!(ctx, "invalid 16 bit signed number");
	}

	#[test]
	fn test_parse_not_enough_data_2() {
		let input = Bytes::new(&[0x12]);

		let result = parse_binary_signed(3).parse(input).unwrap_err();

		let ctx = result.inner().context().next().unwrap().to_string();

		assert_eq!(ctx, "invalid 24 bit signed number");
	}
}

pub fn parse_binary_unsigned<'a>(bytes: usize) -> impl Parser<&'a Bytes, u64, MBusError> {
	(move |input: &mut &'a Bytes| {
		if bytes > 8 {
			return Err(MBusError::assert(input, "cannot parse more than 8 bytes"));
		}
		match bytes {
			0 => Ok(0),
			1 => binary::u8.map(|i| i.into()).parse_next(input),
			2 => binary::le_u16.map(|i| i.into()).parse_next(input),
			4 => binary::le_u32.map(|i| i.into()).parse_next(input),
			8 => binary::le_u64.parse_next(input),
			n => take(n)
				.map(|raw_bytes: &[u8]| {
					let mut data = [0; 8];
					for (i, byte) in raw_bytes.iter().enumerate() {
						data[i] = *byte;
					}
					u64::from_le_bytes(data)
				})
				.parse_next(input),
		}
	})
	.context_with(move || {
		[MBusContext::ComputedLabel(format!(
			"{} bit unsigned number",
			bytes * 8
		))]
		.into_iter()
	})
}

#[cfg(test)]
mod test_parse_binary_unsigned {
	use super::parse_binary_unsigned;
	use winnow::{Bytes, Parser};

	#[test]
	fn test_u8() {
		for i in 0..=u8::MAX {
			let bytes = i.to_le_bytes();
			let input = Bytes::new(&bytes);
			let result = parse_binary_unsigned(1).parse(input).unwrap();
			assert_eq!(result, i.into());
		}
	}

	#[test]
	fn test_u16() {
		for i in 0..=u16::MAX {
			let bytes = i.to_le_bytes();
			let input = Bytes::new(&bytes);
			let result = parse_binary_unsigned(2).parse(input).unwrap();
			assert_eq!(result, i.into());
		}
	}

	#[test]
	fn test_u32() {
		for i in [0, u8::MAX.into(), u16::MAX.into(), u32::MAX] {
			let bytes = i.to_le_bytes();
			let input = Bytes::new(&bytes);
			let result = parse_binary_unsigned(4).parse(input).unwrap();
			assert_eq!(result, i.into());
		}
	}

	#[test]
	fn test_u64() {
		for i in [0, u16::MAX.into(), u32::MAX.into(), u64::MAX] {
			let bytes = i.to_le_bytes();
			let input = Bytes::new(&bytes);
			let result = parse_binary_unsigned(8).parse(input).unwrap();
			assert_eq!(result, i);
		}
	}

	const U24_MAX: u32 = 2_u32.pow(24) - 1;

	#[test]
	fn test_u24() {
		for i in [0, u16::MAX.into(), U24_MAX] {
			let raw_bytes = i.to_le_bytes();
			let bytes = &raw_bytes[..3];
			let input = Bytes::new(bytes);
			let result = parse_binary_unsigned(3).parse(input).unwrap();
			assert_eq!(
				result,
				i.into(),
				"Should be able to parse {i} from bytes {bytes:x?}",
			);
		}
	}

	const I48_MAX: u64 = 2_u64.pow(48) - 1;

	#[test]
	fn test_u48() {
		for i in [0, u16::MAX.into(), U24_MAX.into(), I48_MAX] {
			let raw_bytes = i.to_le_bytes();
			let bytes = &raw_bytes[..6];
			let input = Bytes::new(bytes);
			let result = parse_binary_unsigned(6).parse(input).unwrap();
			assert_eq!(
				result, i,
				"Should be able to parse {i} from bytes {bytes:x?}",
			);
		}
	}

	#[test]
	fn test_parse_zero() {
		let input = Bytes::new(&[]);

		let result = parse_binary_unsigned(0).parse(input).unwrap();

		assert_eq!(result, 0);
	}

	#[test]
	#[should_panic(expected = "cannot parse more than 8 bytes")]
	fn test_parse_ten() {
		let input = Bytes::new(&[0; 9]);

		let _ = parse_binary_unsigned(9).parse(input);
	}

	#[test]
	fn test_parse_not_enough_data_1() {
		let input = Bytes::new(&[0x12]);

		let result = parse_binary_unsigned(2).parse(input).unwrap_err();

		let ctx = result.inner().context().next().unwrap().to_string();

		assert_eq!(ctx, "invalid 16 bit unsigned number");
	}

	#[test]
	fn test_parse_not_enough_data_2() {
		let input = Bytes::new(&[0x12]);

		let result = parse_binary_unsigned(3).parse(input).unwrap_err();

		let ctx = result.inner().context().next().unwrap().to_string();

		assert_eq!(ctx, "invalid 24 bit unsigned number");
	}
}

pub fn parse_real(input: &mut &Bytes) -> MBResult<f32> {
	binary::le_f32.parse_next(input)
}

#[cfg(test)]
mod test_parse_real {
	use super::parse_real;
	use winnow::{Bytes, Parser};

	#[test]
	fn test_works() {
		for i in [f32::NEG_INFINITY, f32::MIN, 0.0, f32::MAX, f32::INFINITY] {
			let bytes = i.to_le_bytes();
			let input = Bytes::new(&bytes);
			let result = parse_real.parse(input).unwrap();
			assert_eq!(result, i);
		}
	}
}
