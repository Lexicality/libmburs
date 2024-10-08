// Copyright 2024 Lexi Robinson
// Licensed under the EUPL-1.2
#![allow(dead_code)]

use winnow::binary;
use winnow::error::StrContext;
use winnow::prelude::*;
use winnow::Bytes;

use crate::parse::application_layer::frame::Frame;
use crate::parse::error::MBResult;

use super::header::LongHeader;
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
pub enum MBusMessage {
	Dlms(u8, TPLHeader), // TODO: Unsupported "see EN 13757–1"
	Reserved,
	ApplicationReset(TPLHeader), // or "Select To Device", EN 13757–3:2018, Clause 7
	CommandToDevice(TPLHeader),  // EN 13757–3:2018, Clause 6
	ResponseFromDevice(TPLHeader, Frame), // EN 13757–3:2018, Clause 6, Annex G
	SelectionOfDevice,           // EN 13757-7:2018, Clause 8.4
	SelectedApplicationRequest(TPLHeader), // EN 13757–3:2018, Clause 7
	SelectedApplicationResponse(TPLHeader), // EN 13757–3:2018, Clause 7
	SynchroniseAction,           // EN 13757–3:2018, Clause 12
	SpecificUsage(u8),           // "Used for specific national implementations"
	TimeSyncToDevice(TPLHeader), // EN 13757–3:2018, Clause 8
	TimeAdjustmentToDevice(TPLHeader), // EN 13757–3:2018, Clause 8
	ApplicationErrorFromDevice(TPLHeader), // EN 13757–3:2018, Clause 10
	AlarmFromDevice(TPLHeader),  // EN 13757–3:2018, Clause 9
	Wireless(u8, TPLHeader),     // TODO: Unsupported - EN 13757–4, EN 13757–5
	Afl,                         // EN 13757-7:2018, Clause 6
	ManufacturerSpecific(u8),
	SetBaudRate(BaudRate),
	ImageTransfer(u8),    // TODO: Unsupported - EN 13757–3:2018, Annex 1
	SecurityTransfer(u8), // TODO: Unsupported - EN 13757–3:2018, Annex A
}

impl MBusMessage {
	pub fn parse(input: &mut &Bytes) -> MBResult<MBusMessage> {
		let ci = binary::u8
			.context(StrContext::Label("CI field"))
			.parse_next(input)?;

		let parse_long_header = LongHeader::parse.context(StrContext::Label("long header"));

		Ok(match ci {
			0x72 => (parse_long_header, Frame::parse)
				.map(|(header, frame)| MBusMessage::ResponseFromDevice(header, frame))
				.parse_next(input)?,
			0x73 => todo!("compact frame"),
			_ => todo!("CI code {ci:x}"),
		})
	}
}
