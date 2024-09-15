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
const VIF_ASCII: u8 = 0b0111_1100;
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

		let value_type = match (extension, raw_value) {
			(_, value) if value <= 0b0111_1010 => parse_table_10(value),
			(true, VIF_EXTENSION_1 | VIF_EXTENSION_2) => {
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
				if raw_value == VIF_EXTENSION_2 && value == VIF_EXTENSION_2 {
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
				} else if raw_value == VIF_EXTENSION_2 {
					parse_table_12(value)
				} else {
					parse_table_14(value)
				}
			}
			(_, VIF_ASCII) => {
				// TODO: EN 13757-3:2018 Annex C.2 strongly suggests
				// (but doesn't actually explicitly say) that the ascii text
				// should follow the VIFEs, but the test data from libmbus has
				// it between the VIF and the VIFEs.
				//
				// Since this is the only examples of plain text VIF data I
				// have, I'm going to have to trust it, but I'm very confused
				bits::bytes(parse_length_prefix_ascii)
					.map(ValueType::PlainText)
					.context(StrContext::Label("plain text VIF data"))
					.parse_next(input)?
			}
			(_, VIF_MANUFACTURER) => ValueType::ManufacturerSpecific,
			(_, VIF_ANY) => ValueType::Any,
			(_, invalid_value) => ValueType::Invalid(invalid_value),
		};

		// TODO: These should be parsed (except for the manufacturer!)
		let extra_vifes = if extension {
			Some(dump_remaining_vifes(input)?)
		} else {
			None
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

fn parse_table_10(value: u8) -> ValueType {
	match value {
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
		_ => ValueType::ReservedCode(VIFTable::Table10, value),
	}
}

fn parse_table_12(value: u8) -> ValueType {
	match value {
		vif!(E000 00nn) => ValueType::Credit(exp(MASK_NN, value, -3)),
		vif!(E000 01nn) => ValueType::Debit(exp(MASK_NN, value, -3)),
		vif!(E000 1000) => ValueType::UniqueMessageIdentification,
		vif!(E000 1001) => ValueType::DeviceType,
		vif!(E000 1010) => ValueType::Manufacturer,
		vif!(E000 1011) => ValueType::ParameterSetIdentification,
		vif!(E000 1100) => ValueType::ModelVersion,
		vif!(E000 1101) => ValueType::HardwareVersionNumber,
		vif!(E000 1110) => ValueType::MetrologyFirmwareVersionNumber,
		vif!(E000 1111) => ValueType::OtherSoftwareVersionNumber,
		vif!(E001 0000) => ValueType::CustomerLocation,
		vif!(E001 0001) => ValueType::Customer,
		vif!(E001 0010) => ValueType::AccessCodeUser,
		vif!(E001 0011) => ValueType::AccessCodeOperator,
		vif!(E001 0100) => ValueType::AccessCodeSystemOperator,
		vif!(E001 0101) => ValueType::AccessCodeDeveloper,
		vif!(E001 0110) => ValueType::Password,
		vif!(E001 0111) => ValueType::ErrorFlags,
		vif!(E001 1000) => ValueType::ErrorMask,
		vif!(E001 1001) => ValueType::SecurityKey,
		vif!(E001 1010) => ValueType::DigitalOutput,
		vif!(E001 1011) => ValueType::DigitalInput,
		vif!(E001 1100) => ValueType::BaudRate,
		vif!(E001 1101) => ValueType::ResponseDelayTime,
		vif!(E001 1110) => ValueType::Retry,
		vif!(E001 1111) => ValueType::RemoteControl,
		vif!(E010 0000) => ValueType::FirstStorageNumberForCyclicStorage,
		vif!(E010 0001) => ValueType::LastStorageNumberForCyclicStorage,
		vif!(E010 0010) => ValueType::SizeOfStorageBlock,
		vif!(E010 0011) => ValueType::DescriptorForTariffAndSubunit,
		vif!(E010 01nn) => ValueType::StorageInterval(DurationType::decode_nn(value)),
		vif!(E010 1000) => ValueType::StorageInterval(DurationType::Months),
		vif!(E010 1001) => ValueType::StorageInterval(DurationType::Years),
		vif!(E010 1010) => ValueType::OperatorSpecific,
		vif!(E010 1011) => ValueType::TimePointSecond,
		vif!(E010 11nn) => ValueType::DurationSinceLastReadout(DurationType::decode_nn(value)),
		vif!(E011 0000) => ValueType::StartDateTimeOfTariff,
		// Unfortunate overlap so we can't use the macro :(
		// vif!(E011 00nn) => ValueType::DurationOfTariff(DurationType::decode_nn(value)),
		0b0011_0001..=0b0011_0011 => ValueType::DurationOfTariff(DurationType::decode_nn(value)),
		vif!(E011 01nn) => ValueType::PeriodOfTarrif(DurationType::decode_nn(value)),
		vif!(E011 1000) => ValueType::PeriodOfTarrif(DurationType::Months),
		vif!(E011 1001) => ValueType::PeriodOfTarrif(DurationType::Years),
		vif!(E011 1010) => ValueType::Dimensionless,
		vif!(E011 1011) => ValueType::WirelessContainer,
		vif!(E011 11nn) => {
			ValueType::PeriodOfNominalDataTransmissions(DurationType::decode_nn(value))
		}
		vif!(E100 nnnn) => ValueType::Volts(exp(MASK_NNNN, value, -9)),
		vif!(E101 nnnn) => ValueType::Amperes(exp(MASK_NNNN, value, -12)),
		vif!(E110 0000) => ValueType::ResetCounter,
		vif!(E110 0001) => ValueType::CumulationCounter,
		vif!(E110 0010) => ValueType::ControlSignal,
		vif!(E110 0011) => ValueType::DayOfWeek,
		vif!(E110 0100) => ValueType::WeekNumber,
		vif!(E110 0101) => ValueType::TimePointOfDayChange,
		vif!(E110 0110) => ValueType::StateOfParameterActivation,
		vif!(E110 0111) => ValueType::SpecialSupplierInformation,
		vif!(E110 10pp) => ValueType::DurationSinceLastCumulation(DurationType::decode_pp(value)),
		vif!(E110 11pp) => ValueType::OperatingTimeBattery(DurationType::decode_pp(value)),
		vif!(E111 0000) => ValueType::DateAndTimeOfBatteryChange,
		vif!(E111 0001) => ValueType::RFLevel,
		vif!(E111 0010) => ValueType::DSTTypeK,
		vif!(E111 0011) => ValueType::ListeningWindowManagement,
		vif!(E111 0100) => ValueType::RemainingBatteryLife(DurationType::Days),
		vif!(E111 0101) => ValueType::NumberTimesMeterStopped,
		vif!(E111 0110) => ValueType::ManufacturerSpecificContainer,
		_ => ValueType::ReservedCode(VIFTable::Table12, value),
	}
}

fn parse_table_13(value: u8) -> ValueType {
	match value {
		vif!(E000 0000) => ValueType::CurrentlySelectedApplication,
		vif!(E000 0010) => ValueType::RemainingBatteryLife(DurationType::Months),
		vif!(E000 0011) => ValueType::RemainingBatteryLife(DurationType::Years),
		_ => ValueType::ReservedCode(VIFTable::Table13, value),
	}
}

fn parse_table_14(value: u8) -> ValueType {
	// "These codes were used until 2004, now they are reserved for future use."
	match value {
		vif!(E000 000n) => ValueType::Energy(EnergyUnit::MWh, exp(MASK_N, value, -1)),
		vif!(E000 001n) => ValueType::ReactiveEnergy(exp(MASK_N, value, 0)),
		vif!(E000 010n) => ValueType::ApparentEnergy(exp(MASK_N, value, 0)),
		vif!(E000 100n) => ValueType::Energy(EnergyUnit::GJ, exp(MASK_N, value, -1)),
		vif!(E000 11nn) => ValueType::Energy(EnergyUnit::MCal, exp(MASK_NN, value, -1)),
		vif!(E001 000n) => ValueType::Volume(VolumeUnit::M3, exp(MASK_N, value, 2)),
		vif!(E001 01nn) => ValueType::ReactivePower(exp(MASK_NN, value, -3)),
		vif!(E001 100n) => ValueType::Mass(MassUnit::T, exp(MASK_N, value, 2)),
		vif!(E001 101n) => ValueType::RelativeHumidity(exp(MASK_N, value, -1)),
		vif!(E010 0000) => ValueType::Volume(VolumeUnit::Feet3, 0),
		vif!(E010 0001) => ValueType::Volume(VolumeUnit::Feet3, -1), // The table says "0,1 feet³" and I don't know what that means
		0b0010_0010..=0b0010_0110 => ValueType::RetiredCode(VIFTable::Table14, value),
		vif!(E010 100n) => ValueType::Power(PowerUnit::MW, exp(MASK_N, value, -1)),
		vif!(E010 1010) => ValueType::PhaseUU,
		vif!(E010 1011) => ValueType::PhaseUI,
		vif!(E010 11nn) => ValueType::Frequency(exp(MASK_NN, value, -3)),
		vif!(E011 000n) => ValueType::Power(PowerUnit::GJph, exp(MASK_N, value, -1)),
		vif!(E011 01nn) => ValueType::ApparentPower(exp(MASK_NN, value, -1)),
		0b0101_1000..=0b0110_0111 => ValueType::RetiredCode(VIFTable::Table14, value),
		vif!(E110 1000) => ValueType::ResultingPowerFactorK,
		vif!(E110 1001) => ValueType::ThermalOutputRatingFactorKq,
		vif!(E110 1010) => ValueType::ThermalCouplingRatingFactorOverallKc,
		vif!(E110 1011) => ValueType::ThermalCouplingRatingFactorRoomSideKcr,
		vif!(E110 1100) => ValueType::ThermalCouplingRatingFactorHeaterSideKch,
		vif!(E110 1101) => ValueType::LowTemperatureRatingFactorKt,
		vif!(E110 1110) => ValueType::DisplayOutputScalingFactorKD,
		vif!(E111 00nn) => ValueType::RetiredCode(VIFTable::Table14, value),
		vif!(E111 01nn) => ValueType::ColdWarmTemperatureLimit(exp(MASK_NN, value, -3)),
		vif!(E111 1nnn) => ValueType::CumulativeMaxOfActivePower(exp(MASK_NNN, value, -3)),
		_ => ValueType::ReservedCode(VIFTable::Table14, value),
	}
}

#[derive(Debug)]
pub enum VIFTable {
	Table10,
	Table12,
	Table13,
	Table14,
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
	RetiredCode(VIFTable, u8), // "These codes were used until 2004, now they are reserved for future use."
	// These two are for compatability with libmbus, any dataframe with these
	// values is strictly invalid, but it just keeps on trucking anyway
	ReservedCode(VIFTable, u8),
	Invalid(u8),
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
	AccessCodeSystemOperator,
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
				| Self::Customer
				| Self::AccessCodeUser
				| Self::AccessCodeOperator
				| Self::AccessCodeDeveloper
				| Self::Password
				| Self::ErrorMask
				| Self::SecurityKey
				| Self::BaudRate
				| Self::ResponseDelayTime
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
