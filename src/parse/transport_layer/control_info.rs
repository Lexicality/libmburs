// Copyright 2024 Lexi Robinson
// Licensed under the EUPL-1.2
use winnow::binary;
use winnow::combinator::repeat;
use winnow::error::{AddContext, ErrMode, ErrorKind, ParserError, StrContext};
use winnow::prelude::*;
use winnow::stream::Stream;
use winnow::Bytes;

use crate::parse::application_layer::frame::Frame;
use crate::parse::application_layer::record::Record;
use crate::parse::error::MBResult;

use super::header::LongHeader;
use super::header::ShortHeader;
use super::header::TPLHeader;

#[derive(Debug)]
pub enum BaudRate {
	Rate300,
	Rate600,
	Rate1200,
	Rate2400,
	Rate4800,
	Rate9600,
	Rate19200,
	Rate38400,
}

#[derive(Debug)]
pub enum ApplicationError {
	Unspecified,
	CIFieldError,
	BufferOverflow,
	RecordOverflow,
	RecordError,
	DIFEOverflow,
	VIFEOverflow,
	ApplicationBusy,
	CreditOverflow,
	NoFunction,
	DataError,
	RoutingOrRelayingError,
	AccessViolation,
	ParameterError,
	SizeError,
	SecurityError,
	SecurityMechanismNotSupported,
	InadequateSecurityMethod,
	DynamicError(Record),
	ManufacturerSpecific(u8, Vec<u8>),
}
impl ApplicationError {
	pub fn parse(input: &mut &Bytes) -> MBResult<Self> {
		if input.is_empty() {
			return Ok(Self::Unspecified);
		}

		let error_code_checkpoint = input.checkpoint();
		let error_code = binary::u8
			.context(StrContext::Label("Error Code"))
			.parse_next(input)?;

		Ok(match error_code {
			0x00 => Self::Unspecified,
			0x01 => Self::CIFieldError,
			0x02 => Self::BufferOverflow,
			0x03 => Self::RecordOverflow,
			0x04 => Self::RecordError,
			0x05 => Self::DIFEOverflow,
			0x06 => Self::VIFEOverflow,
			0x08 => Self::ApplicationBusy,
			0x09 => Self::CreditOverflow,
			0x11 => Self::NoFunction,
			0x12 => Self::DataError,
			0x13 => Self::RoutingOrRelayingError,
			0x14 => Self::AccessViolation,
			0x15 => Self::ParameterError,
			0x16 => Self::SizeError,
			0x20 => Self::SecurityError,
			0x21 => Self::SecurityMechanismNotSupported,
			0x22 => Self::InadequateSecurityMethod,
			0xF0 => Self::DynamicError(Record::parse.parse_next(input)?),
			0xF1..=0xFF => Self::ManufacturerSpecific(
				error_code,
				repeat::<_, _, Vec<_>, _, _>(0.., binary::u8)
					.context(StrContext::Label("Manufacturer Specific Data"))
					.parse_next(input)?,
			),
			_ => {
				return Err(
					ErrMode::from_error_kind(input, ErrorKind::Verify).add_context(
						input,
						&error_code_checkpoint,
						StrContext::Label("reserved error code"),
					),
				);
			}
		})
	}
}

#[derive(Debug)]
pub enum MBusMessage {
	// Application stuff
	ApplicationReset(TPLHeader),           // EN 13757–3:2018, Clause 7
	ApplicationSelect(TPLHeader, Vec<u8>), // EN 13757–3:2018, Clause 7
	SelectedApplicationRequest(TPLHeader), // EN 13757–3:2018, Clause 7
	SelectedApplicationResponse(TPLHeader, Vec<u8>), // EN 13757–3:2018, Clause 7
	// Management Commands
	SelectionOfDevice(Vec<u8>),                 // EN 13757-7:2018, Clause 8.4
	SetBaudRate(BaudRate),                      // EN 13757-7:2018, Clause 8
	SynchroniseAction,                          // EN 13757–3:2018, Clause 12
	TimeAdjustmentToDevice(TPLHeader, Vec<u8>), // EN 13757–3:2018, Clause 8
	TimeSyncToDevice(TPLHeader, Vec<u8>),       // EN 13757–3:2018, Clause 8
	// Data operations
	AlarmFromDevice(TPLHeader, Vec<u8>), // EN 13757–3:2018, Clause 9
	ApplicationErrorFromDevice(TPLHeader, ApplicationError), // EN 13757–3:2018, Clause 10
	CommandToDevice(TPLHeader, Vec<u8>), // EN 13757–3:2018, Clause 6
	ResponseFromDevice(TPLHeader, Frame), // EN 13757–3:2018, Clause 6, Annex G
	// Unsupported
	AuthenticationAndFrgamentation(Vec<u8>), // EN 13757-7:2018, Clause 6
	Dlms(u8, TPLHeader, Vec<u8>),            // TODO: Unsupported "see EN 13757–1"
	ImageTransfer(u8, TPLHeader, Vec<u8>),   // TODO: Unsupported - EN 13757–3:2018, Annex I
	ManufacturerSpecific(u8, Vec<u8>),       // EN 13757–3:2018, Clause 13
	SecurityTransfer(u8, TPLHeader, Vec<u8>), // TODO: Unsupported - EN 13757–3:2018, Annex A
	SpecificUsage(u8, TPLHeader, Vec<u8>),   // "Used for specific national implementations"
	Wireless(u8, TPLHeader),                 // TODO: Unsupported - EN 13757–4, EN 13757–5
}

impl MBusMessage {
	pub fn parse(input: &mut &Bytes) -> MBResult<MBusMessage> {
		let ci_checkpoint = input.checkpoint();
		let ci = binary::u8
			.context(StrContext::Label("CI field"))
			.parse_next(input)?;

		let header = match ci {
			0x00..=0x1F
			| 0x54
			| 0x5C
			| 0x66
			| 0x69
			| 0x70..=0x71
			| 0x78..=0x79
			| 0x81
			| 0x83
			| 0x86
			| 0x89
			| 0x8C..=0x90
			| 0xA0..=0xBF => TPLHeader::None,
			0x5A | 0x61 | 0x65 | 0x67 | 0x6A | 0x6E | 0x74 | 0x7A | 0x7B | 0x7D | 0x8A | 0x88
			| 0x9E | 0xC1 | 0xC4 => ShortHeader::parse
				.context(StrContext::Label("short header"))
				.parse_next(input)?,
			0x53
			| 0x55
			| 0x5B
			| 0x5F
			| 0x60
			| 0x64
			| 0x68
			| 0x6B..=0x6D
			| 0x6F
			| 0x72
			| 0x73
			| 0x75
			| 0x7C
			| 0x80
			| 0x82
			| 0x84
			| 0x85
			| 0x87
			| 0x8B
			| 0x9F
			| 0xC0
			| 0xC2
			| 0xC3
			| 0xC5 => LongHeader::parse
				.context(StrContext::Label("long header"))
				.parse_next(input)?,
			_ => {
				return Err(
					ErrMode::from_error_kind(input, ErrorKind::Verify).add_context(
						input,
						&ci_checkpoint,
						StrContext::Label("reserved CI field"),
					),
				);
			}
		};

		let mut parse_remaining = repeat::<_, _, Vec<_>, _, _>(0.., binary::u8)
			.context(StrContext::Label("Remaining Data"));

		Ok(match ci {
			// Unsupported
			0x00..=0x1F | 0x60 | 0x61 | 0x7C | 0x7D => {
				Self::Dlms(ci, header, parse_remaining.parse_next(input)?)
			}
			0x5F | 0x9E | 0x9F => {
				Self::SpecificUsage(ci, header, parse_remaining.parse_next(input)?)
			}
			0x80..=0x83 | 0x86..=0x8F => Self::Wireless(ci, header),
			0x90 => Self::AuthenticationAndFrgamentation(parse_remaining.parse_next(input)?),
			0xA0..=0xB7 => Self::ManufacturerSpecific(ci, parse_remaining.parse_next(input)?),
			0xC0..=0xC2 => Self::ImageTransfer(ci, header, parse_remaining.parse_next(input)?),
			0xC3..=0xC5 => Self::SecurityTransfer(ci, header, parse_remaining.parse_next(input)?),
			// Application behaviour
			0x50 | 0x53 => parse_remaining
				.map(|data| {
					let header = header.clone();
					if data.is_empty() {
						Self::ApplicationReset(header)
					} else {
						Self::ApplicationSelect(header, data)
					}
				})
				.parse_next(input)?,
			0x54 | 0x55 => Self::SelectedApplicationRequest(header),
			0x66..=0x68 => {
				Self::SelectedApplicationResponse(header, parse_remaining.parse_next(input)?)
			}
			0x52 => Self::SelectionOfDevice(parse_remaining.parse_next(input)?),
			// Management Commands
			0x5C => Self::SynchroniseAction,
			0xB8..=0xBF => Self::SetBaudRate(match ci {
				0xB8 => BaudRate::Rate300,
				0xB9 => BaudRate::Rate600,
				0xBA => BaudRate::Rate1200,
				0xBB => BaudRate::Rate2400,
				0xBC => BaudRate::Rate4800,
				0xBD => BaudRate::Rate9600,
				0xBE => BaudRate::Rate19200,
				0xBF => BaudRate::Rate38400,
				_ => unreachable!(),
			}),
			0x6C => Self::TimeSyncToDevice(header, parse_remaining.parse_next(input)?),
			0x6D => Self::TimeAdjustmentToDevice(header, parse_remaining.parse_next(input)?),
			// Actual mbus
			0x51 | 0x5A | 0x5B => Self::CommandToDevice(header, parse_remaining.parse_next(input)?),
			0x69..=0x6B => todo!("format frame"),
			0x6E..=0x70 => {
				Self::ApplicationErrorFromDevice(header, ApplicationError::parse.parse_next(input)?)
			}
			0x71 | 0x74 | 0x75 => Self::AlarmFromDevice(header, parse_remaining.parse_next(input)?),
			0x72 | 0x78 | 0x7A => Self::ResponseFromDevice(header, Frame::parse.parse_next(input)?),
			0x73 | 0x79 | 0x7B => todo!("compact frame"),
			_ => unreachable!(),
		})
	}
}
