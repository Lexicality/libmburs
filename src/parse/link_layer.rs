// Copyright 2024 Lexi Robinson
// Licensed under the EUPL-1.2

use winnow::binary;
use winnow::combinator::{alt, cut_err, preceded};
use winnow::error::{AddContext, ErrMode, ErrorKind, ParserError, StrContext};
use winnow::prelude::*;
use winnow::stream::Stream;
use winnow::Bytes;

const LONG_FRAME_HEADER: u8 = 0x68;
const SHORT_FRAME_HEADER: u8 = 0x10;
const FRAME_TAIL: u8 = 0x16;
const ACK_FRAME: u8 = 0xE5;

#[derive(Debug)]
pub enum Packet<'a> {
	Ack,
	Short {
		control: u8,
		address: u8,
	},
	Long {
		control: u8,
		address: u8,
		data: &'a [u8],
	},
}

fn parse_variable<'a>(input: &mut &'a Bytes) -> PResult<Packet<'a>> {
	let length = binary::u8
		.context(StrContext::Label("length"))
		.parse_next(input)?;
	binary::u8
		.verify(|v| *v == length)
		.void()
		.context(StrContext::Label("length confirmation"))
		.parse_next(input)?;
	LONG_FRAME_HEADER
		.void()
		.context(StrContext::Label("frame marker"))
		.parse_next(input)?;
	let (control, address) = (
		binary::u8.context(StrContext::Label("control byte")),
		binary::u8.context(StrContext::Label("address byte")),
	)
		.parse_next(input)?;
	let length = length.into();
	// There are two bytes after the input
	if input.len() < length {
		return Err(
			ErrMode::from_error_kind(input, ErrorKind::Slice).add_context(
				input,
				&input.checkpoint(),
				StrContext::Label("packet data"),
			),
		);
	}
	let data = input.next_slice(length - 2);
	let (checksum, _) = (
		binary::u8.context(StrContext::Label("checksum")),
		FRAME_TAIL.void().context(StrContext::Label("frame tail")),
	)
		.parse_next(input)?;

	let sum = data
		.iter()
		.copied()
		.reduce(u8::wrapping_add)
		.unwrap_or_default()
		.wrapping_add(control)
		.wrapping_add(address);

	if sum != checksum {
		return Err(
			ErrMode::from_error_kind(input, ErrorKind::Verify).add_context(
				input,
				&input.checkpoint(),
				StrContext::Label("checksum verify"),
			),
		);
	}

	Ok(Packet::Long {
		control,
		address,
		data,
	})
}

fn parse_fixed<'a>(input: &mut &'a Bytes) -> PResult<Packet<'a>> {
	// mbus's fixed length datagrams are 2 bytes long, only control & address
	let (control, address, checksum, _) = (
		binary::u8.context(StrContext::Label("control byte")),
		binary::u8.context(StrContext::Label("address byte")),
		binary::u8.context(StrContext::Label("checksum")),
		FRAME_TAIL.void().context(StrContext::Label("frame tail")),
	)
		.parse_next(input)?;

	let sum = control.wrapping_add(address);
	if sum != checksum {
		return Err(
			ErrMode::from_error_kind(input, ErrorKind::Verify).add_context(
				input,
				&input.checkpoint(),
				StrContext::Label("checksum verify"),
			),
		);
	}

	Ok(Packet::Short { control, address })
}

fn parse_ack<'a>(_input: &mut &'a Bytes) -> PResult<Packet<'a>> {
	Ok(Packet::Ack)
}

impl<'a> Packet<'a> {
	pub fn parse(input: &mut &'a Bytes) -> PResult<Packet<'a>> {
		alt((
			preceded(
				LONG_FRAME_HEADER.void(),
				cut_err(parse_variable.context(StrContext::Label("long frame header"))),
			),
			preceded(
				SHORT_FRAME_HEADER.void(),
				cut_err(parse_fixed.context(StrContext::Label("short frame header"))),
			),
			preceded(ACK_FRAME.void(), cut_err(parse_ack)),
		))
		.parse_next(input)
	}
}
