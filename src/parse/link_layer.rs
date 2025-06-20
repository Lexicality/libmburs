// Copyright 2024 Lexi Robinson
// Licensed under the EUPL-1.2

use winnow::binary;
use winnow::binary::bits;
use winnow::combinator::{alt, cut_err, preceded};
use winnow::error::{AddContext, ErrMode, ParserError, StrContext};
use winnow::prelude::*;
use winnow::stream::Stream;
use winnow::Bytes;

use super::error::{MBResult, MBusError};
use super::transport_layer::MBusMessage;

const LONG_FRAME_HEADER: u8 = 0x68;
const SHORT_FRAME_HEADER: u8 = 0x10;
const FRAME_TAIL: u8 = 0x16;
const ACK_FRAME: u8 = 0xE5;

#[derive(Debug)]
pub enum PrimaryControlMessage {
	ResetRemoteLink,
	ResetUserProcess,
	SendUserDataConfirmed,
	SendUserDataUnconfirmed,
	RequestAccessDemand,
	RequestLinkStatus,
	RequestUserData1, // REQ UD1
	RequestUserData2, // REQ UD2
}

#[derive(Debug)]
pub enum SecondaryControlMessage {
	ACK,
	NACK,
	UserData,
	UserDataUnavailable,
	Status, // "Status of link or access demand"
	LinkNotFunctioning,
	LinkNotImplemented,
}

#[derive(Debug)]
pub enum DataFlowControl {
	Continue, // "further messages are acceptable"
	Pause,    // "further messages may cause data overflow"
}

#[derive(Debug)]
pub enum Control {
	Primary {
		frame_count_bit: bool,
		message: PrimaryControlMessage,
	},
	Secondary {
		access_demand: bool, // The secondary wants you to send it a REQ UD1 ASAP
		data_flow_control: DataFlowControl,
		message: SecondaryControlMessage,
	},
}

impl Control {
	fn parse(input: &mut &Bytes) -> MBResult<Self> {
		bits::bits((
			bits::bool
				.verify(|v| !v)
				.context(StrContext::Label("reserved"))
				.void(),
			bits::bool.context(StrContext::Label("PRM")),
			bits::bool.context(StrContext::Label("FCB/ACD")),
			bits::bool.context(StrContext::Label("FCV/DFC")),
			bits::take::<_, u8, _, MBusError>(4_usize).context(StrContext::Label("function")),
		))
		.verify_map(|(_, prm, fcb_acd, fcv_dfc, function)| {
			Some(if prm {
				Self::Primary {
					frame_count_bit: fcb_acd,
					message: match (fcv_dfc, function) {
						(false, 0) => PrimaryControlMessage::ResetRemoteLink,
						(false, 1) => PrimaryControlMessage::ResetUserProcess,
						(_, 2) => return None, // "Reserved for balanced transmission procedure"
						(true, 3) => PrimaryControlMessage::SendUserDataConfirmed,
						(false, 4) => PrimaryControlMessage::SendUserDataUnconfirmed,
						(false, 8) => PrimaryControlMessage::RequestAccessDemand,
						(false, 9) => PrimaryControlMessage::RequestLinkStatus,
						(true, 10) => PrimaryControlMessage::RequestUserData1,
						(true, 11) => PrimaryControlMessage::RequestUserData2,
						_ => return None,
					},
				}
			} else {
				Self::Secondary {
					access_demand: fcb_acd,
					data_flow_control: if fcv_dfc {
						DataFlowControl::Pause
					} else {
						DataFlowControl::Continue
					},
					message: match function {
						0 => SecondaryControlMessage::ACK,
						1 => SecondaryControlMessage::NACK,
						8 => SecondaryControlMessage::UserData,
						9 => SecondaryControlMessage::UserDataUnavailable,
						11 => SecondaryControlMessage::Status,
						14 => SecondaryControlMessage::LinkNotFunctioning,
						15 => SecondaryControlMessage::LinkNotFunctioning,
						_ => return None,
					},
				}
			})
		})
		.parse_next(input)
	}
}

#[derive(Debug)]
pub enum Packet {
	Ack,
	Short {
		control: Control,
		address: u8,
	},
	Long {
		control: Control,
		address: u8,
		message: MBusMessage,
	},
}

fn parse_variable(input: &mut &Bytes) -> MBResult<Packet> {
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
	let ((control, raw_control), address) = (
		Control::parse
			.context(StrContext::Label("control byte"))
			.with_taken()
			.map(|(control, raw_slice)| (control, raw_slice[0])),
		binary::u8.context(StrContext::Label("address byte")),
	)
		.parse_next(input)?;
	let length = length.into();
	// There are two bytes after the input
	if input.len() < length {
		return Err(ErrMode::from_input(input).add_context(
			input,
			&input.checkpoint(),
			StrContext::Label("packet data"),
		));
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
		.wrapping_add(raw_control)
		.wrapping_add(address);

	if sum != checksum {
		return Err(ErrMode::from_input(input).add_context(
			input,
			&input.checkpoint(),
			StrContext::Label("checksum verify"),
		));
	}

	let mut data = Bytes::new(data);

	let message = MBusMessage::parse.parse_next(&mut data)?;

	Ok(Packet::Long {
		control,
		address,
		message,
	})
}

fn parse_fixed(input: &mut &Bytes) -> MBResult<Packet> {
	// mbus's fixed length datagrams are 2 bytes long, only control & address
	let ((control, raw_control), address, checksum, _) = (
		Control::parse
			.context(StrContext::Label("control byte"))
			.with_taken()
			.map(|(control, raw_slice)| (control, raw_slice[0])),
		binary::u8.context(StrContext::Label("address byte")),
		binary::u8.context(StrContext::Label("checksum")),
		FRAME_TAIL.void().context(StrContext::Label("frame tail")),
	)
		.parse_next(input)?;

	let sum = raw_control.wrapping_add(address);
	if sum != checksum {
		return Err(ErrMode::from_input(input).add_context(
			input,
			&input.checkpoint(),
			StrContext::Label("checksum verify"),
		));
	}

	Ok(Packet::Short { control, address })
}

fn parse_ack(_input: &mut &Bytes) -> MBResult<Packet> {
	Ok(Packet::Ack)
}

impl Packet {
	pub fn parse(input: &mut &Bytes) -> MBResult<Packet> {
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
