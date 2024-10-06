// Copyright 2024 Lexi Robinson
// Licensed under the EUPL-1.2

use winnow::binary;
use winnow::binary::bits;
use winnow::combinator::{alt, cut_err, preceded};
use winnow::error::{AddContext, ErrMode, ErrorKind, ParserError, StrContext};
use winnow::prelude::*;
use winnow::stream::Stream;
use winnow::Bytes;

use super::error::{MBResult, MBusEncodErrorCause, MBusEncodeError, MBusError};
use super::transport_layer::MBusMessage;
use super::types::Encode;

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

impl Encode for Control {
	#[allow(clippy::identity_op)]
	fn encode(&self) -> Result<Vec<u8>, MBusEncodeError> {
		let res = match self {
			Self::Primary {
				frame_count_bit,
				message,
			} => {
				const FRAME_COUNT_BIT: u8 = 0b0010_0000;
				const FRAME_COUNT_VALID: u8 = 0b0001_0000;
				const FRAME_COUNT_INVALID: u8 = 0b0000_0000;

				let mut res = 0b0100_0000;

				if *frame_count_bit {
					res |= FRAME_COUNT_BIT;
				}
				res |= match message {
					PrimaryControlMessage::ResetRemoteLink => FRAME_COUNT_INVALID | 0,
					PrimaryControlMessage::ResetUserProcess => FRAME_COUNT_INVALID | 1,
					PrimaryControlMessage::SendUserDataConfirmed => FRAME_COUNT_VALID | 3,
					PrimaryControlMessage::SendUserDataUnconfirmed => FRAME_COUNT_INVALID | 3,
					PrimaryControlMessage::RequestAccessDemand => FRAME_COUNT_INVALID | 8,
					PrimaryControlMessage::RequestLinkStatus => FRAME_COUNT_INVALID | 9,
					PrimaryControlMessage::RequestUserData1 => FRAME_COUNT_VALID | 10,
					PrimaryControlMessage::RequestUserData2 => FRAME_COUNT_VALID | 11,
				};

				res
			}
			Self::Secondary {
				access_demand: acess_demand,
				data_flow_control,
				message,
			} => {
				const ACCESS_DEMAND: u8 = 0b0010_0000;
				const FLOW_CONTROL_PAUSE: u8 = 0b0001_0000;

				let mut res = 0;
				if *acess_demand {
					res |= ACCESS_DEMAND;
				}
				if matches!(data_flow_control, DataFlowControl::Pause) {
					res |= FLOW_CONTROL_PAUSE;
				}

				res |= match message {
					SecondaryControlMessage::ACK => 0,
					SecondaryControlMessage::NACK => 1,
					SecondaryControlMessage::UserData => 8,
					SecondaryControlMessage::UserDataUnavailable => 9,
					SecondaryControlMessage::Status => 11,
					SecondaryControlMessage::LinkNotFunctioning => 14,
					SecondaryControlMessage::LinkNotImplemented => 15,
				};

				res
			}
		};
		Ok(vec![res])
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
			.with_recognized()
			.map(|(control, raw_slice)| (control, raw_slice[0])),
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
		.wrapping_add(raw_control)
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
			.with_recognized()
			.map(|(control, raw_slice)| (control, raw_slice[0])),
		binary::u8.context(StrContext::Label("address byte")),
		binary::u8.context(StrContext::Label("checksum")),
		FRAME_TAIL.void().context(StrContext::Label("frame tail")),
	)
		.parse_next(input)?;

	let sum = raw_control.wrapping_add(address);
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

impl Encode for Packet {
	fn encode(&self) -> Result<Vec<u8>, MBusEncodeError> {
		Ok(match self {
			Self::Ack => vec![ACK_FRAME],
			Self::Short { control, address } => {
				let mut data = control.encode()?;
				data.push(*address);

				let checksum: u8 = data
					.iter()
					.copied()
					.reduce(|a, b| a.wrapping_add(b))
					.unwrap_or(0);

				let mut frame = vec![SHORT_FRAME_HEADER];
				frame.reserve_exact(4);
				frame.append(&mut data);
				frame.push(checksum);
				frame.push(FRAME_TAIL);
				frame
			}
			Self::Long {
				control,
				address,
				message,
			} => {
				let mut data = control.encode()?;
				data.push(*address);
				data.append(&mut message.encode()?);

				let data_length = data.len();
				if data_length > 253 {
					return Err(MBusEncodeError(MBusEncodErrorCause::UserDataTooLong));
				}
				let length_byte: u8 = data_length.try_into().unwrap();

				let checksum: u8 = data
					.iter()
					.copied()
					.reduce(|a, b| a.wrapping_add(b))
					.unwrap_or(0);

				let mut frame = vec![
					LONG_FRAME_HEADER,
					length_byte,
					length_byte,
					LONG_FRAME_HEADER,
				];
				frame.reserve_exact(2 + data_length);
				frame.append(&mut data);
				frame.push(checksum);
				frame.push(FRAME_TAIL);
				frame
			}
		})
	}
}
