// Copyright 2024 Lexi Robinson
// Licensed under the EUPL-1.2
#![allow(dead_code)]
use winnow::binary;
use winnow::combinator::peek;
use winnow::error::StrContext;
use winnow::prelude::*;
use winnow::Bytes;

use crate::parse::error::{MBResult, MBusError};
use crate::parse::types::number::parse_bcd;

use super::manufacturer::{device_name, unpack_manufacturer_code};

#[derive(Debug, Clone)]
pub enum ApplicationError {
	None,
	Busy,
	/// Shall be used to communicate a failure during the interpretation or the
	/// execution of a received command, e.g. if a non-decipherable message was
	/// received.
	Error,
	/// Shall be used if a correct working application detects an abnormal
	/// behaviour like a permanent flow of water by a water meter.
	Alarm,
}

// TODO: This is packed into a single byte so we should be able to use a
// bitfield or something as opposed to 7 bytes
#[derive(Debug, Clone)]
pub struct MeterStatus {
	pub manufacturer_2: bool,
	pub manufacturer_1: bool,
	pub manufacturer_0: bool,
	/// Warning — The bit “temporary error” is set only if the meter signals a
	/// slight error condition (which not immediately requires a service
	/// action). This error condition may later disappear.
	pub temporary_error: bool,
	/// Failure — The bit “permanent error” is set only if the meter signals a
	/// fatal device error (which requires a service action).
	/// Error can be reset only by a service action.
	pub permanent_error: bool,
	/// Warning — The bit “power low” is set only to signal interruption of
	/// external power supply or the end of battery life.
	pub power_low: bool,
	pub application: ApplicationError,
}

impl MeterStatus {
	fn parse(input: &mut &Bytes) -> MBResult<MeterStatus> {
		binary::bits::bits::<_, _, MBusError, _, _>((
			binary::bits::bool,
			binary::bits::bool,
			binary::bits::bool,
			binary::bits::bool,
			binary::bits::bool,
			binary::bits::bool,
			binary::bits::take(2_usize),
		))
		.map(
			|(
				manufacturer_2,
				manufacturer_1,
				manufacturer_0,
				temporary_error,
				permanent_error,
				power_low,
				application,
			)| MeterStatus {
				manufacturer_2,
				manufacturer_1,
				manufacturer_0,
				temporary_error,
				permanent_error,
				power_low,
				application: match application {
					0b00 => ApplicationError::None,
					0b01 => ApplicationError::Busy,
					0b10 => ApplicationError::Error,
					0b11 => ApplicationError::Alarm,
					_ => unreachable!(),
				},
			},
		)
		.parse_next(input)
	}
}

/// This is a placeholder until I actually have some way to test security modes
/// For more information see BS EN 13757-7:2018 7.6.2 and 7.6.3
#[derive(Debug, Clone)]
pub struct ExtraHeader;

#[derive(Debug, Clone)]
pub enum SecurityMode {
	None,
	/// Indicates that the packet is corrupted and should be discarded, unless
	/// you're the libmbus test data that requires me to support this
	Reserved(u16),
}
impl SecurityMode {
	fn parse(input: &mut &Bytes) -> MBResult<SecurityMode> {
		let raw_value = peek(binary::le_u16)
			.context(StrContext::Label("Raw value peek"))
			.parse_next(input)?;
		(binary::bits::bits::<_, _, MBusError, _, _>((
			binary::bits::take(8_usize).context(StrContext::Label("Security mode info low")),
			binary::bits::take(5_usize).context(StrContext::Label("Security mode")),
			binary::bits::take(3_usize).context(StrContext::Label("Security mode info high")),
		)))
		.verify_map(|(info_low, security_mode, info_high): (u8, u8, u8)| {
			match security_mode {
				0 => {
					if info_high == 0 && info_low == 0 {
						Some(SecurityMode::None)
					} else {
						None
					}
				}
				// libmbus strikes again
				6 | 11 | 12 | 14 | 16..=31 => Some(SecurityMode::Reserved(raw_value)),
				_ => todo!("Packet encryption is not yet supported (mode {security_mode})"),
			}
		})
		.parse_next(input)
	}
}

#[derive(Debug, Clone)]
pub struct ShortHeader {
	pub access_number: u8,
	pub status: MeterStatus,
	pub configuration_field: SecurityMode,
	pub extra_header: Option<ExtraHeader>,
}

impl ShortHeader {
	pub fn parse(input: &mut &Bytes) -> MBResult<TPLHeader> {
		Self::parse_raw.map(TPLHeader::Short).parse_next(input)
	}

	fn parse_raw(input: &mut &Bytes) -> MBResult<ShortHeader> {
		(
			binary::u8.context(StrContext::Label("access number")),
			MeterStatus::parse.context(StrContext::Label("status")),
			SecurityMode::parse.context(StrContext::Label("tpl configuration field")),
		)
			.map(|(access_number, status, configuration_field)| ShortHeader {
				access_number,
				status,
				configuration_field,
				// This value is set by the contents of `configuration_field`
				// which as established above is always 0 at this point which
				// means "no extra headers"
				extra_header: None,
			})
			.parse_next(input)
	}
}

#[derive(Debug, Clone, Copy)]
pub enum WaterMeterType {
	Potable,      // temperature unspecified
	Irrigation,   // (unpotable)
	Cold,         // (potable)
	Warm,         // 30°C..90°C
	Hot,          // >=90°C
	DualRegister, // (potable)
	Waste,
}

#[derive(Debug, Clone, Copy)]
pub enum ThermalMeterType {
	OutletHeat,
	InletHeat,
	OutletCooling,
	InletCooling,
	Combined,
}

#[derive(Debug, Clone, Copy)]
pub enum DeviceType {
	Other,
	OilMeter,
	ElectricityMeter,
	GasMeter,
	ThermalEnergyMeter(ThermalMeterType),
	SteamMeter,
	WaterMeter(WaterMeterType),
	HeatCostAllocator,
	CompressedAir,
	BusOrSystemComponent,
	Unknown, // Different to "other" apparently
	WaterDataLogger,
	GasDataLogger,
	GasConverter,
	CalorificValue,
	PressureMeter,
	ADConverter,
	SmokeDetector,
	RoomSensor, // "e.g. temperature or humidity"
	GasDetector,
	ReservedSensor,
	ElectricalBreaker,
	Valve, // Gas or water
	ReservedSwitchingDevice,
	CustomerUnit, // Display device
	ReservedCustomerUnit,
	Garbage,
	ReservedCO2,
	ReservedEnvironmental,
	ServiceTool,
	CommunicationController, // "Gateway"
	UnidirectionalRepeater,
	BidirectionalRepeater,
	ReservedSystemDevice,
	RadioConverterSystemSide,
	RadioConverterMeterSide,
	BusConverterMeterSide,
	Reserved, // Just in general
	Wildcard,
}

impl DeviceType {
	fn parse(input: &mut &Bytes) -> MBResult<Self> {
		binary::u8
			.map(|v| match v {
				0x00 => Self::Other,
				0x01 => Self::OilMeter,
				0x02 => Self::ElectricityMeter,
				0x03 => Self::GasMeter,
				0x04 => Self::ThermalEnergyMeter(ThermalMeterType::OutletHeat),
				0x05 => Self::SteamMeter,
				0x06 => Self::WaterMeter(WaterMeterType::Warm),
				0x07 => Self::WaterMeter(WaterMeterType::Potable),
				0x08 => Self::HeatCostAllocator,
				0x09 => Self::CompressedAir,
				0x0A => Self::ThermalEnergyMeter(ThermalMeterType::OutletCooling),
				0x0B => Self::ThermalEnergyMeter(ThermalMeterType::InletCooling),
				0x0C => Self::ThermalEnergyMeter(ThermalMeterType::InletHeat),
				0x0D => Self::ThermalEnergyMeter(ThermalMeterType::Combined),
				0x0E => Self::BusOrSystemComponent,
				0x0F => Self::Unknown,
				0x10 => Self::WaterMeter(WaterMeterType::Irrigation),
				0x11 => Self::WaterDataLogger,
				0x12 => Self::GasDataLogger,
				0x13 => Self::GasConverter,
				0x14 => Self::CalorificValue,
				0x15 => Self::WaterMeter(WaterMeterType::Hot),
				0x16 => Self::WaterMeter(WaterMeterType::Cold),
				0x17 => Self::WaterMeter(WaterMeterType::DualRegister),
				0x18 => Self::PressureMeter,
				0x19 => Self::ADConverter,
				0x1A => Self::SmokeDetector,
				0x1B => Self::RoomSensor,
				0x1C => Self::GasDetector,
				0x1D..=0x1F => Self::ReservedSensor,
				0x20 => Self::ElectricalBreaker,
				0x21 => Self::Valve,
				0x22..=0x24 => Self::ReservedSwitchingDevice,
				0x25 => Self::CustomerUnit,
				0x26 | 0x27 => Self::ReservedCustomerUnit,
				0x28 => Self::WaterMeter(WaterMeterType::Waste),
				0x29 => Self::Garbage,
				0x2A => Self::ReservedCO2,
				0x2B..=0x2F => Self::ReservedEnvironmental,
				0x30 => Self::ServiceTool,
				0x31 => Self::CommunicationController,
				0x32 => Self::UnidirectionalRepeater,
				0x33 => Self::BidirectionalRepeater,
				0x34 | 0x35 => Self::ReservedSystemDevice,
				0x36 => Self::RadioConverterSystemSide,
				0x37 => Self::RadioConverterMeterSide,
				0x38 => Self::BusConverterMeterSide,
				0x39..=0x3F => Self::ReservedSystemDevice,
				0x40..=0xFE => Self::Reserved,
				0xFF => Self::Wildcard,
			})
			.parse_next(input)
	}
}

#[derive(Debug, Clone)]
pub struct LongHeader {
	pub identifier: u32,
	pub manufacturer: String,
	pub device_name: Option<&'static str>,
	pub version: u8,
	pub device_type: DeviceType,
	pub access_number: u8,
	pub status: MeterStatus,
	pub configuration_field: SecurityMode,
	pub extra_header: Option<ExtraHeader>,
}

impl LongHeader {
	pub fn parse(input: &mut &Bytes) -> MBResult<TPLHeader> {
		(
			parse_bcd(4)
				.try_map(u32::try_from)
				.with_recognized()
				.context(StrContext::Label("device identifier")),
			binary::le_u16
				.verify_map(|raw| {
					unpack_manufacturer_code(raw)
						.ok()
						.filter(|parsed| parsed.chars().all(|c| c.is_ascii_uppercase()))
						.map(|parsed| (parsed, raw))
				})
				.context(StrContext::Label("manufacturer")),
			binary::u8.context(StrContext::Label("version")),
			DeviceType::parse.context(StrContext::Label("device type")),
			// The rest of the long header is simply the short header, so use that parser
			ShortHeader::parse_raw,
		)
			.map(
				|(
					(identifier, raw_identifier),
					(manufacturer, raw_manufacturer),
					version,
					device_type,
					short_header,
				)| LongHeader {
					identifier,
					manufacturer,
					device_name: device_name(
						raw_identifier,
						raw_manufacturer,
						version,
						device_type,
					),
					version,
					device_type,
					access_number: short_header.access_number,
					status: short_header.status,
					configuration_field: short_header.configuration_field,
					extra_header: short_header.extra_header,
				},
			)
			.map(TPLHeader::Long)
			.parse_next(input)
	}
}

#[derive(Debug, Clone)]
pub enum TPLHeader {
	None,
	Short(ShortHeader),
	Long(LongHeader),
}
