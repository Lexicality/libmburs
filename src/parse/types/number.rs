// Copyright 2023 Lexi Robinson
// Licensed under the EUPL-1.2

use winnow::binary;
use winnow::combinator::repeat;
use winnow::error::{ErrMode, ParserError, StrContext};
use winnow::prelude::*;
use winnow::Bytes;

use crate::parse::application_layer::dib::RawDataType;
use crate::parse::application_layer::vib::ValueType;
use crate::parse::error::{MBResult, MBusError, ParseError, Result};
use crate::parse::Datagram;

use super::{BitsInput, DataType, ParseResult};

pub fn parse_number(dt: RawDataType, vt: ValueType, dg: &mut Datagram) -> ParseResult {
	match dt {
		RawDataType::BCD(len) => decode_bcd(dg.take(len)?),
		RawDataType::Real => decode_real(dg.take(4)?),
		RawDataType::Binary(len) => {
			if vt.is_date() {
				super::date::parse_datetime(dt, vt, dg)
			} else if vt.is_unsigned() {
				decode_binary_unsigned(dg.take(len)?)
			} else {
				decode_binary_signed(dg.take(len)?)
			}
		}
		_ => Err(ParseError::DataTypeMismatch),
	}
}

fn parse_nibble(input: &mut BitsInput<'_>) -> MBResult<i64> {
	binary::bits::take(4_usize).parse_next(input)
}

fn parse_bcd_nibble(input: &mut BitsInput<'_>) -> MBResult<i64> {
	parse_nibble.verify(|v| *v < 10).parse_next(input)
}

pub fn parse_bcd<'a>(bytes: usize) -> impl Parser<&'a Bytes, i64, MBusError> {
	let parser = move |input: &mut BitsInput<'a>| {
		if bytes == 0 {
			return Err(ErrMode::assert(input, "cannot parse 0 bytes"));
		} else if bytes > 9 {
			return Err(ErrMode::assert(
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
			parse_nibble.verify(|v| *v == 0x0F || *v < 10),
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

	move |input: &mut &'a Bytes| {
		binary::bits::bits(parser)
			.context(StrContext::Label("signed BCD number"))
			.parse_next(input)
	}
}

#[cfg(test)]
mod test_parse_bcd {
	use winnow::error::ErrorKind;
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
	#[should_panic(expected = "cannot parse 0 bytes")]
	fn test_parse_zero() {
		let input = Bytes::new(&[]);

		let _ = parse_bcd(0).parse(input);
	}

	#[test]
	#[should_panic(expected = "cannot safely parse more than 9 bytes")]
	fn test_parse_ten() {
		let input = Bytes::new(&[]);

		let _ = parse_bcd(10).parse(input);
	}

	#[test]
	fn test_parse_not_enough_data() {
		let input = Bytes::new(&[0x12]);

		let result = parse_bcd(2).parse(input).unwrap_err();

		assert_eq!(result.inner().kind(), ErrorKind::Eof);
	}

	#[test]
	fn test_parse_garbage() {
		for byte in [
			[0xAA],
			[0xBB],
			[0xCC],
			[0xDD],
			[0xEE],
			[0xFF],
			// 0xF0 is valid but 0x0F is not
			[0x0F],
		] {
			let input = Bytes::new(&byte);

			let result = parse_bcd(1).parse(input).unwrap_err();

			assert_eq!(
				result.inner().kind(),
				ErrorKind::Verify,
				"cannot parse invalid BCD byte {:#X}",
				byte[0]
			);
		}
	}
}

fn decode_bcd(mut data: Vec<u8>) -> ParseResult {
	data.reverse();
	let last = data[0];
	let negative = last & 0xF0 == 0xF0;
	if negative {
		data[0] = last & 0x0F;
	}

	let mut ret: i64 = 0;
	for byte in data {
		ret = (ret * 10) + decode_bcd_digit(byte >> 4)? as i64;
		ret = (ret * 10) + decode_bcd_digit(byte)? as i64;
	}
	if negative {
		ret *= -1;
	}

	Ok(DataType::Signed(ret))
}

fn decode_bcd_digit(mut byte: u8) -> Result<u8> {
	byte &= 0x0F;
	if byte < 0x0A {
		Ok(byte)
	} else {
		Err(ParseError::InvalidData("Invalid BCD nybble"))
	}
}

const TWOS_COMPLEMENT_MASK: u8 = 0b1000_0000;

fn decode_binary_signed(mut data: Vec<u8>) -> ParseResult {
	Ok(DataType::Signed(match data.len() {
		1 => i8::from_le_bytes(data.try_into().unwrap()) as i64,
		2 => i16::from_le_bytes(data.try_into().unwrap()) as i64,
		4 => i32::from_le_bytes(data.try_into().unwrap()) as i64,
		8 => i64::from_le_bytes(data.try_into().unwrap()),
		len @ (3 | 6) => {
			let is_negative = data.last().unwrap() & TWOS_COMPLEMENT_MASK == TWOS_COMPLEMENT_MASK;
			let filler = if is_negative { 0xFF } else { 0x00 };
			data.extend((0..(8 - len)).map(|_| filler));
			i64::from_le_bytes(data.try_into().unwrap())
		}
		_ => {
			return Err(ParseError::InvalidData(
				"Unsupported byte count for signed binary",
			))
		}
	}))
}

fn decode_binary_unsigned(mut data: Vec<u8>) -> ParseResult {
	Ok(DataType::Unsigned(match data.len() {
		1 => data[0] as u64,
		2 => u16::from_le_bytes(data.try_into().unwrap()) as u64,
		4 => u32::from_le_bytes(data.try_into().unwrap()) as u64,
		8 => u64::from_le_bytes(data.try_into().unwrap()),
		len @ (3 | 6) => {
			data.extend((0..(8 - len)).map(|_| 0x00));
			u64::from_le_bytes(data.try_into().unwrap())
		}
		_ => {
			return Err(ParseError::InvalidData(
				"Unsupported byte count for unsigned binary",
			))
		}
	}))
}

fn decode_real(data: Vec<u8>) -> ParseResult {
	Ok(DataType::Real(match data.len() {
		4 => f32::from_le_bytes(data.try_into().unwrap()),
		_ => return Err(ParseError::InvalidData("Unsupported byte count for real")),
	}))
}

#[cfg(test)]
mod bcd_tests {
	use super::*;

	#[test]
	fn single_byte() {
		let result = decode_bcd(vec![0x12]);
		assert_eq!(result, Ok(DataType::Signed(12)));
	}

	#[test]
	fn double_byte() {
		let result = decode_bcd(vec![0x34, 0x12]);
		assert_eq!(result, Ok(DataType::Signed(1234)));
	}

	#[test]
	fn twelve_digits() {
		let result = decode_bcd(vec![0x34, 0x12, 0x90, 0x78, 0x56, 0x34, 0x12]);
		assert_eq!(result, Ok(DataType::Signed(12345678901234)));
	}

	#[test]
	fn eighteen_digits() {
		// for the LVAR stuff
		let result = decode_bcd(vec![0x11; 18 / 2]);
		assert_eq!(result, Ok(DataType::Signed(111111111111111111)));
	}

	#[test]
	fn negativity() {
		let result = decode_bcd(vec![0xF1]);
		assert_eq!(result, Ok(DataType::Signed(-1)));
	}

	#[test]
	fn mass_negativity() {
		let result = decode_bcd(vec![0x23, 0xF1]);
		assert_eq!(result, Ok(DataType::Signed(-123)));
	}

	#[test]
	fn failed_negativity() {
		let result = decode_bcd(vec![0xF1, 0x23]);
		assert!(matches!(result, Err(ParseError::InvalidData(_))));
	}

	#[test]
	fn dodgy_data() {
		let result = decode_bcd(vec![0xA2]);
		assert!(matches!(result, Err(ParseError::InvalidData(_))));
	}
}

#[cfg(test)]
mod binary_signed_tests {
	use super::*;

	#[test]
	fn single_byte() {
		let result = decode_binary_signed(vec![0x05]);
		assert_eq!(result, Ok(DataType::Signed(5)));
		let result = decode_binary_signed(vec![0xFF]);
		assert_eq!(result, Ok(DataType::Signed(-1)));
	}

	#[test]
	fn i8() {
		for i in [i8::MIN, -1, 0, 1, i8::MAX] {
			let result = decode_binary_signed(i.to_le_bytes().into());
			assert_eq!(result, Ok(DataType::Signed(i as i64)));
		}
	}

	#[test]
	fn i16() {
		for i in [i16::MIN, -1, 0, 1, i16::MAX] {
			let result = decode_binary_signed(i.to_le_bytes().into());
			assert_eq!(result, Ok(DataType::Signed(i as i64)));
		}
	}

	#[test]
	fn i32() {
		for i in [i32::MIN, -1, 0, 1, i32::MAX] {
			let result = decode_binary_signed(i.to_le_bytes().into());
			assert_eq!(result, Ok(DataType::Signed(i as i64)));
		}
	}

	#[test]
	fn i64() {
		for i in [i64::MIN, -1, 0, 1, i64::MAX] {
			let result = decode_binary_signed(i.to_le_bytes().into());
			assert_eq!(result, Ok(DataType::Signed(i)));
		}
	}

	#[test]
	fn i24() {
		for (expected, bytes) in [
			(-8388608, [0x00, 0x00, 0x80]),
			(-1, [0xFF, 0xFF, 0xFF]),
			(0, [0x00, 0x00, 0x00]),
			(1, [0x01, 0x00, 0x00]),
			(8388607, [0xFF, 0xFF, 0x7F]),
		] {
			let result = decode_binary_signed(bytes.into());
			assert_eq!(result, Ok(DataType::Signed(expected as i64)));
		}
	}

	#[test]
	fn i48() {
		for (expected, bytes) in [
			(-140737488355328, [0x00, 0x00, 0x00, 0x00, 0x00, 0x80]),
			(-1, [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]),
			(0_i64, [0x00, 0x00, 0x00, 0x00, 0x00, 0x00]),
			(1, [0x01, 0x00, 0x00, 0x00, 0x00, 0x00]),
			(140737488355327, [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x7F]),
		] {
			let result = decode_binary_signed(bytes.into());
			assert_eq!(result, Ok(DataType::Signed(expected)));
		}
	}

	#[test]
	fn i40() {
		let bytes = [0x00; 5];
		let result = decode_binary_signed(bytes.into());
		assert!(matches!(result, Err(ParseError::InvalidData(_))));
	}

	#[test]
	fn i128() {
		let bytes = [0x00; 16];
		let result = decode_binary_signed(bytes.into());
		assert!(matches!(result, Err(ParseError::InvalidData(_))));
	}
}
