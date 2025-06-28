// Copyright 2024 Lexi Robinson
// Licensed under the EUPL-1.2
#![allow(dead_code)]

use winnow::binary;
use winnow::combinator::{alt, eof, repeat};
use winnow::error::{AddContext, ParserError, StrContext};
use winnow::prelude::*;
use winnow::stream::Stream;
use winnow::Bytes;

use crate::parse::error::{MBResult, MBusError};

use super::record::Record;

#[derive(Debug)]
pub enum ApplicationErrorMessage {
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

impl ApplicationErrorMessage {
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
				return Err(MBusError::from_input(input).add_context(
					input,
					&error_code_checkpoint,
					StrContext::Label("reserved error code"),
				));
			}
		})
	}
}

#[derive(Debug)]
pub enum MessageApplication {
	All,
	UserData,        // Consumption
	SimpleBilling,   // Current and fixed date values + dates
	EnhancedBilling, // Historic values
	MultiTariffBilling,
	InstantaneousValues, // For regulation
	LoadProfileValuesForManagement,
	StaticContent,
	InstallationAndStartup, // Bus address, fixed dates
	Testing,                // High resolution values
	Calibration,
	Manufacturing,
	Development,
	SelfTest,
	ConfigurationData,
	UserDefinedData, // Data set selected by the user
	ManufacturerSpecific(u8),
}

#[derive(Debug)]
pub struct ApplicationMessage {
	// Yes, the `ApplicationMessage` type has a `message_application` field
	message_application: MessageApplication,
	block_number: u64,
}

impl ApplicationMessage {
	pub fn parse(input: &mut &Bytes) -> MBResult<Option<Self>> {
		alt((
			eof.void().default_value(),
			repeat(
				1..=10,
				binary::bits::bits::<_, _, MBusError, _, _>((
					binary::bits::take(4_usize),
					binary::bits::take(4_usize),
				)),
			)
			.fold(
				|| (0_u64, 0_u64, false),
				|(mut acc_ma, mut acc_bn, mut ma_done), (ma, bn): (u64, u64)| {
					if !ma_done {
						acc_ma += ma;
						ma_done = ma >= 0x0F
					}
					acc_bn <<= 4;
					acc_bn += bn;
					(acc_ma, acc_bn, ma_done)
				},
			)
			.verify_map(|(message_application, block_number, _)| {
				Some(Some(ApplicationMessage {
					message_application: match message_application {
						0 => MessageApplication::All,
						1 => MessageApplication::UserData,
						2 => MessageApplication::SimpleBilling,
						3 => MessageApplication::EnhancedBilling,
						4 => MessageApplication::MultiTariffBilling,
						5 => MessageApplication::InstantaneousValues,
						6 => MessageApplication::LoadProfileValuesForManagement,
						7 => MessageApplication::StaticContent,
						8 => MessageApplication::InstallationAndStartup,
						9 => MessageApplication::Testing,
						10 => MessageApplication::Calibration,
						11 => MessageApplication::Manufacturing,
						12 => MessageApplication::Development,
						13 => MessageApplication::SelfTest,
						14 => MessageApplication::ConfigurationData,
						15 => MessageApplication::UserDefinedData,
						26..=45 => MessageApplication::ManufacturerSpecific(
							message_application
								.try_into()
								.expect("26..=45 fits into a u8"),
						),
						_ => return None,
					},
					block_number,
				}))
			}),
		))
		.parse_next(input)
	}
}
