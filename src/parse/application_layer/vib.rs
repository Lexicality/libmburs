// Copyright 2023 Lexi Robinson
// Licensed under the EUPL-1.2
#![allow(dead_code)]

use crate::parse::error::MBResult;
use crate::parse::types::string::parse_length_prefix_ascii;
use crate::parse::types::BitsInput;
use winnow::binary::bits;
use winnow::error::{ErrMode, ErrorKind, ParserError, StrContext};
use winnow::prelude::*;

const VIF_EXTENSION_1: u8 = 0b0111_1011;
const VIF_EXTENSION_2: u8 = 0b0111_1101;
const VIF_ASCII: u8 = 0b011_1100;
const VIF_MANUFACTURER: u8 = 0b0111_1111;
const VIF_ANY: u8 = 0b0111_1110;

const DURATION_MASK: u8 = 0b0000_0011;

#[allow(dead_code)]
#[derive(Debug)]
pub struct ValueInfoBlock {
	value_type: ValueType,
	/// Currently unparsed VIFE that modify the actual value
	/// TODO: parse them!
	extra_vifes: Option<Vec<u8>>,
}

pub fn parse_vif_byte(input: &mut BitsInput<'_>) -> MBResult<(bool, u8)> {
	(bits::bool, bits::take(7_usize)).parse_next(input)
}

pub fn dump_remaining_vifes(input: &mut BitsInput<'_>) -> MBResult<Vec<u8>> {
	let mut ret = Vec::new();
	loop {
		let (extension, value) = parse_vif_byte
			.context(StrContext::Label("VIFE"))
			.parse_next(input)?;
		ret.push(value);
		if !extension {
			break;
		}
	}
	Ok(ret)
}

impl ValueInfoBlock {
	pub fn parse(input: &mut BitsInput<'_>) -> MBResult<Self> {
		let (mut extension, raw_value) = parse_vif_byte
			.context(StrContext::Label("initial VIF"))
			.parse_next(input)?;

		let value_type = match raw_value {
			value if value <= 0b0111_1010 => parse_table_10(value),
			VIF_EXTENSION_1 | VIF_EXTENSION_2 => {
				if !extension {
					return Err(ErrMode::from_error_kind(input, ErrorKind::Verify));
				}
				let value: u8;
				(extension, value) = parse_vif_byte
					.context(StrContext::Label("VIF extension byte"))
					.parse_next(input)?;
				if raw_value == VIF_EXTENSION_1 && value == VIF_EXTENSION_2 {
					if !extension {
						return Err(ErrMode::from_error_kind(input, ErrorKind::Verify));
					}
					let value: u8;
					(extension, value) = parse_vif_byte
						.context(StrContext::Label("VIF extension layer 2 byte"))
						.parse_next(input)?;
					parse_table_13(value)
				} else if raw_value == VIF_EXTENSION_1 {
					parse_table_12(value)
				} else {
					parse_table_14(value)
				}
			}
			VIF_ASCII => {
				// We need to deal with any potential extensions before we can
				// read the vif string, so chuck a placeholder in there
				ValueType::PlainText(String::new())
			}
			VIF_MANUFACTURER => ValueType::ManufacturerSpecific,
			VIF_ANY => ValueType::Any,
			_ => ValueType::Reserved,
		};

		// TODO: These should be parsed (except for the manufacturer!)
		let extra_vifes = if extension {
			Some(dump_remaining_vifes(input)?)
		} else {
			None
		};

		// Now we've parsed all the VIFEs we can get the ascii VIF if necessary
		let value_type = match value_type {
			ValueType::PlainText(_) => ValueType::PlainText(
				bits::bytes(parse_length_prefix_ascii)
					.context(StrContext::Label("plain text VIF data"))
					.parse_next(input)?,
			),
			value_type => value_type,
		};

		Ok(Self {
			value_type,
			extra_vifes,
		})
	}
}

fn parse_table_10(value: u8) -> ValueType {
	match value {
		0b0111_0100..=0b0111_0111 => {
			ValueType::ActualityDuration(DurationType::decode_nn(value & DURATION_MASK))
		}
		_ => todo!("table 10 {value} {value:x} {value:b}"),
	}
}

fn parse_table_12(value: u8) -> ValueType {
	todo!("table 12 {value} {value:x} {value:b}")
}

fn parse_table_13(value: u8) -> ValueType {
	todo!("table 13 {value} {value:x} {value:b}")
}

fn parse_table_14(value: u8) -> ValueType {
	todo!("table 14 {value} {value:x} {value:b}")
}

#[derive(Debug)]
pub enum DurationType {
	Seconds,
	Minutes,
	Hours,
	Days,
	Months,
	Years,
}

impl DurationType {
	fn decode_nn(value: u8) -> Self {
		match value {
			0b00 => Self::Seconds,
			0b01 => Self::Minutes,
			0b10 => Self::Hours,
			0b11 => Self::Days,
			_ => unreachable!(),
		}
	}

	fn decode_pp(value: u8) -> Self {
		match value {
			0b00 => Self::Hours,
			0b01 => Self::Days,
			0b10 => Self::Months,
			0b11 => Self::Years,
			_ => unreachable!(),
		}
	}
}

#[derive(Debug)]
pub enum Unit {
	Bar,   // bar
	C,     // °C
	Feet3, // feet³
	GJ,    // GJ
	GJph,  // GJ/h
	Hz,    // Hz
	J,     // J
	Jph,   // J/h
	K,     // K
	KVAR,  // kVAR
	KVAh,  // kVAh
	KVA,   // kVA
	Kg,    // kg
	Kvarh, // kvarh
	M3,    // m³
	MCal,  // MCal
	KWh,   // kWh
	MW,    // MW
	MWh,   // MWh
	Pct,   // %
	T,     // t
	W,     // W
	Wh,    // Wh
}

#[derive(Debug)]
pub enum ValueType {
	Any,
	Reserved,
	Unsupported,
	PlainText(String),
	ManufacturerSpecific,
	Energy(Unit, i8),
	Volume(Unit, i8),
	Mass(Unit, i8),
	OnTime(DurationType),
	Pressure(Unit, i8),
	Power(Unit, i8),
	VolumeFlow(Unit, DurationType, i8),
	MassFlow(Unit, DurationType, i8),
	FlowTemperature(Unit, i8),
	ExternalTemperature(Unit, i8),
	ReturnTemperature(Unit, i8),
	TemperatureDifference(Unit, i8),
	AveragingDuration(DurationType),
	ActualityDuration(DurationType),
	FabricationNumber,
	HCA, // TODO: what
	Address,
	TypeFDateTime,
	TypeGDate,
	TypeIDateTime,
	TypeJTime,
	TypeMDatetime,
	// TODO: But wait there's more
}

impl ValueType {
	pub fn is_unsigned(&self) -> bool {
		// TODO
		false
	}

	pub fn is_date(&self) -> bool {
		matches!(
			self,
			Self::TypeFDateTime
				| Self::TypeGDate
				| Self::TypeIDateTime
				| Self::TypeJTime
				| Self::TypeMDatetime
		)
	}
}
