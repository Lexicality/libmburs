// Copyright 2023 Lexi Robinson
// Licensed under the EUPL-1.2
#![allow(dead_code)]

use crate::parse::error::MBResult;
use crate::parse::types::string::parse_length_prefix_ascii;
use crate::parse::types::BitsInput;
use libmbus_macros::vif;
use winnow::binary::bits;
use winnow::error::{AddContext, ErrMode, ErrorKind, ParserError, StrContext};
use winnow::prelude::*;
use winnow::stream::Stream;

const VIF_EXTENSION_1: u8 = 0b0111_1011;
const VIF_EXTENSION_2: u8 = 0b0111_1101;
const VIF_ASCII: u8 = 0b011_1100;
const VIF_MANUFACTURER: u8 = 0b0111_1111;
const VIF_ANY: u8 = 0b0111_1110;

const MASK_N: u8 = 0b0000_0001;
const MASK_NN: u8 = 0b0000_0011;
const MASK_NNN: u8 = 0b0000_0111;
const MASK_NNNN: u8 = 0b0000_1111;
const DURATION_MASK: u8 = 0b0000_0011;

#[allow(dead_code)]
#[derive(Debug)]
pub struct ValueInfoBlock {
	pub value_type: ValueType,
	/// Currently unparsed VIFE that modify the actual value
	/// TODO: parse them!
	pub extra_vifes: Option<Vec<u8>>,
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
		let vif_checkpoint = input.checkpoint();
		let (mut extension, raw_value) = parse_vif_byte
			.context(StrContext::Label("initial VIF"))
			.parse_next(input)?;

		let value_type = match raw_value {
			value if value <= 0b0111_1010 => parse_table_10(value),
			VIF_EXTENSION_1 | VIF_EXTENSION_2 => {
				if !extension {
					return Err(
						ErrMode::from_error_kind(input, ErrorKind::Verify).add_context(
							input,
							&vif_checkpoint,
							StrContext::Label("vife missing for vif extension"),
						),
					);
				}
				let vife_checkpoint = input.checkpoint();
				let value: u8;
				(extension, value) = parse_vif_byte
					.context(StrContext::Label("VIF extension byte"))
					.parse_next(input)?;
				if raw_value == VIF_EXTENSION_1 && value == VIF_EXTENSION_2 {
					if !extension {
						return Err(ErrMode::from_error_kind(input, ErrorKind::Verify)
							.add_context(
								input,
								&vife_checkpoint,
								StrContext::Label("vife missing for vif extension level 2"),
							));
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
				Some(ValueType::PlainText(String::new()))
			}
			VIF_MANUFACTURER => Some(ValueType::ManufacturerSpecific),
			VIF_ANY => Some(ValueType::Any),
			_ => None,
		};

		let Some(value_type) = value_type else {
			return Err(
				ErrMode::from_error_kind(input, ErrorKind::Verify).add_context(
					input,
					&vif_checkpoint,
					StrContext::Label("reserved vif"),
				),
			);
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

fn exp(mask: u8, value: u8, offset: i8) -> Exponent {
	(value & mask) as i8 + offset
}

fn parse_table_10(value: u8) -> Option<ValueType> {
	Some(match value {
		vif!(E000 0nnn) => ValueType::Energy(EnergyUnit::Wh, exp(MASK_NNN, value, -3)),
		vif!(E000 1nnn) => ValueType::Energy(EnergyUnit::J, exp(MASK_NNN, value, 0)),
		vif!(E001 0nnn) => ValueType::Volume(VolumeUnit::M3, exp(MASK_NNN, value, -6)),
		vif!(E001 1nnn) => ValueType::Mass(MassUnit::Kg, exp(MASK_NNN, value, -3)),
		vif!(E010 00nn) => ValueType::OnTime(DurationType::decode_nn(value)),
		vif!(E010 01nn) => ValueType::OperatingTime(DurationType::decode_nn(value)),
		vif!(E010 1nnn) => ValueType::Power(PowerUnit::W, exp(MASK_NNN, value, -3)),
		vif!(E011 0nnn) => ValueType::Power(PowerUnit::Jph, exp(MASK_NNN, value, 0)),
		vif!(E011 1nnn) => ValueType::VolumeFlow(DurationType::Hours, exp(MASK_NNN, value, -6)),
		vif!(E100 0nnn) => ValueType::VolumeFlow(DurationType::Minutes, exp(MASK_NNN, value, -7)),
		vif!(E100 1nnn) => ValueType::VolumeFlow(DurationType::Seconds, exp(MASK_NNN, value, -9)),
		vif!(E101 0nnn) => ValueType::MassFlow(DurationType::Hours, exp(MASK_NNN, value, -3)),
		vif!(E101 10nn) => ValueType::FlowTemperature(exp(MASK_NN, value, -3)),
		vif!(E101 11nn) => ValueType::ReturnTemperature(exp(MASK_NN, value, -3)),
		vif!(E110 00nn) => ValueType::TemperatureDifference(exp(MASK_NN, value, -3)),
		vif!(E110 01nn) => ValueType::ExternalTemperature(exp(MASK_NN, value, -3)),
		vif!(E110 10nn) => ValueType::Pressure(exp(MASK_NN, value, -3)),
		// 0b0110_1100..=0b0110_1101 => todo!("dates go here"),
		0b0110_1100..=0b0110_1101 => ValueType::Any,
		vif!(E110 1110) => ValueType::HCA,
		vif!(E111 00nn) => ValueType::AveragingDuration(DurationType::decode_nn(value)),
		vif!(E111 01nn) => ValueType::ActualityDuration(DurationType::decode_nn(value)),
		vif!(E111 1000) => ValueType::FabricationNumber,
		vif!(E111 1001) => ValueType::EnhancedIdentification,
		vif!(E111 1010) => ValueType::Address,
		_ => return None,
	})
}

fn parse_table_12(value: u8) -> Option<ValueType> {
	Some(match value {
		vif!(E011 01nn) => ValueType::PeriodOfTarrif(DurationType::decode_nn(value)),
		vif!(E011 1000) => ValueType::PeriodOfTarrif(DurationType::Months),
		vif!(E011 1001) => ValueType::PeriodOfTarrif(DurationType::Years),
		_ => todo!("table 12 {value} {value:x} {value:b}"),
	})
}

fn parse_table_13(value: u8) -> Option<ValueType> {
	todo!("table 13 {value} {value:x} {value:b}")
}

fn parse_table_14(value: u8) -> Option<ValueType> {
	Some(match value {
		0b0000_0000..=0b0000_0001 => ValueType::Energy(EnergyUnit::MWh, exp(MASK_N, value, -1)),
		_ => todo!("table 14 {value} {value:x} {value:b}"),
	})
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
		match value & MASK_NN {
			0b00 => Self::Seconds,
			0b01 => Self::Minutes,
			0b10 => Self::Hours,
			0b11 => Self::Days,
			_ => unreachable!(),
		}
	}

	fn decode_pp(value: u8) -> Self {
		match value & MASK_NN {
			0b00 => Self::Hours,
			0b01 => Self::Days,
			0b10 => Self::Months,
			0b11 => Self::Years,
			_ => unreachable!(),
		}
	}
}

#[derive(Debug)]
pub enum EnergyUnit {
	Wh,   // Wh
	J,    // J
	MWh,  // MWh
	MCal, // MCal
	GJ,   // GJ
}

#[derive(Debug)]
pub enum PowerUnit {
	W,    // W
	Jph,  // J/h
	MW,   // MW
	GJph, // GJ/h
}

#[derive(Debug)]
pub enum VolumeUnit {
	M3,    // m³
	Feet3, // feet³
}

#[derive(Debug)]
pub enum MassUnit {
	Kg, // kg
	T,  // t
}

pub type Exponent = i8;

#[derive(Debug)]
pub enum ValueType {
	// Special
	Any,
	PlainText(String),
	ManufacturerSpecific,
	// Table 10 - Primary VIF-codes
	Energy(EnergyUnit, Exponent),
	Volume(VolumeUnit, Exponent),
	Mass(MassUnit, Exponent),
	OnTime(DurationType),
	OperatingTime(DurationType),
	Power(PowerUnit, Exponent),
	VolumeFlow(DurationType, Exponent),
	MassFlow(DurationType, Exponent),
	FlowTemperature(Exponent),
	ReturnTemperature(Exponent),
	TemperatureDifference(Exponent),
	ExternalTemperature(Exponent),
	Pressure(Exponent),
	TypeGDate,
	TypeFDateTime,
	TypeJTime,
	TypeIDateTime,
	TypeMDatetime,
	HCA, // Heat cost allocators perhaps? Not explained
	AveragingDuration(DurationType),
	ActualityDuration(DurationType),
	FabricationNumber,
	EnhancedIdentification,
	Address,
	// Table 12 — Main VIFE-code extension table
	Credit(Exponent),
	Debit(Exponent),
	UniqueMessageIdentification, // "Previously named Access number (transmission count)"
	DeviceType,
	Manufacturer,
	ParameterSetIdentification,
	ModelVersion,
	HardwareVersionNumber,
	MetrologyFirmwareVersionNumber,
	OtherSoftwareVersionNumber,
	CustomerLocation,
	Customer,
	AccessCodeUser,
	AccessCodeOperator,
	AccessCodeDeveloper,
	Password,
	ErrorFlags,
	ErrorMask,
	SecurityKey,
	DigitalOutput,
	DigitalInput,
	BaudRate,
	ResponseDelayTime,
	Retry,
	RemoteControl,
	FirstStorageNumberForCyclicStorage,
	LastStorageNumberForCyclicStorage,
	SizeOfStorageBlock,
	DescriptorForTariffAndSubunit,
	StorageInterval(DurationType),
	OperatorSpecific,
	TimePointSecond,
	DurationSinceLastReadout(DurationType),
	StartDateTimeOfTariff, // What type of date? Unspecified. Good luck!
	DurationOfTariff(DurationType),
	PeriodOfTarrif(DurationType),
	Dimensionless, // L + "no VIF"
	WirelessContainer,
	PeriodOfNominalDataTransmissions(DurationType),
	Volts(Exponent),
	Amperes(Exponent),
	ResetCounter,
	CumulationCounter,
	ControlSignal,
	DayOfWeek,
	WeekNumber,
	TimePointOfDayChange,
	StateOfParameterActivation,
	SpecialSupplierInformation,
	DurationSinceLastCumulation(DurationType),
	OperatingTimeBattery(DurationType),
	DateAndTimeOfBatteryChange, // This is one of the date formats, you are instructed to guess which one based on the size of the data field
	RFLevel,                    // dBm
	DSTTypeK,
	ListeningWindowManagement, // DataTypeL
	RemainingBatteryLife(DurationType),
	NumberTimesMeterStopped,
	ManufacturerSpecificContainer,
	// Table 13 — 2nd level VIFE code extension table
	CurrentlySelectedApplication,
	// Table 14 — Alternate extended VIF-code table
	ReactiveEnergy(Exponent),
	ApparentEnergy(Exponent),
	ReactivePower(Exponent),
	RelativeHumidity(Exponent),
	PhaseUU, // "volt. to volt."
	PhaseUI, // "volt. to current"
	Frequency(Exponent),
	ApparentPower(Exponent),
	ColdWarmTemperatureLimit(Exponent),
	CumulativeMaxOfActivePower(Exponent),
	ResultingPowerFactorK,
	ThermalOutputRatingFactorKq,
	ThermalCouplingRatingFactorOverallKc,
	ThermalCouplingRatingFactorRoomSideKcr,
	ThermalCouplingRatingFactorHeaterSideKch,
	LowTemperatureRatingFactorKt,
	DisplayOutputScalingFactorKD,
}

impl ValueType {
	pub fn is_unsigned(&self) -> bool {
		matches!(
			self,
			Self::UniqueMessageIdentification
				| Self::DeviceType
				| Self::Manufacturer
				| Self::ParameterSetIdentification
				| Self::ModelVersion
				| Self::HardwareVersionNumber
				| Self::MetrologyFirmwareVersionNumber
				| Self::OtherSoftwareVersionNumber
				| Self::CustomerLocation
				| Self::Customer | Self::AccessCodeUser
				| Self::AccessCodeOperator
				| Self::AccessCodeDeveloper
				| Self::Password | Self::ErrorMask
				| Self::SecurityKey
				| Self::BaudRate | Self::ResponseDelayTime
				| Self::FirstStorageNumberForCyclicStorage
				| Self::LastStorageNumberForCyclicStorage
				| Self::SizeOfStorageBlock
				| Self::DescriptorForTariffAndSubunit
				| Self::TimePointSecond
				| Self::DurationSinceLastReadout(_)
				| Self::DurationOfTariff(_)
				| Self::PeriodOfTarrif(_)
				| Self::PeriodOfNominalDataTransmissions(_)
				| Self::DayOfWeek
				| Self::WeekNumber
				| Self::StateOfParameterActivation
				| Self::SpecialSupplierInformation
				| Self::DurationSinceLastCumulation(_)
				| Self::RemainingBatteryLife(_)
				| Self::NumberTimesMeterStopped
				| Self::RelativeHumidity(_)
				| Self::ResultingPowerFactorK
				| Self::ThermalCouplingRatingFactorHeaterSideKch
				| Self::ThermalCouplingRatingFactorOverallKc
				| Self::ThermalCouplingRatingFactorRoomSideKcr
				| Self::ThermalOutputRatingFactorKq
		)
	}

	pub fn is_boolean(&self) -> bool {
		matches!(
			self,
			Self::ErrorFlags | Self::DigitalOutput | Self::DigitalInput | Self::RemoteControl
		)
	}
}
