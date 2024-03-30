// Copyright 2024 Lexi Robinson
// Licensed under the EUPL-1.2
#![allow(dead_code)]
use winnow::binary;
use winnow::error::{ContextError, InputError, ParserError, StrContext};
use winnow::prelude::*;
use winnow::Bytes;

use super::manufacturer::{device_name, unpack_manufacturer_code};

/// This is a placeholder until I actually have some way to test security modes
/// For more information see BS EN 13757-7:2018 7.6.2 and 7.6.3
#[derive(Debug)]
pub struct ExtraHeader;

#[derive(Debug)]
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
#[derive(Debug)]
pub struct MeterStatus {
	manufacturer_2: bool,
	manufacturer_1: bool,
	manufacturer_0: bool,
	/// Warning — The bit “temporary error” is set only if the meter signals a
	/// slight error condition (which not immediately requires a service
	/// action). This error condition may later disappear.
	temporary_error: bool,
	/// Failure — The bit “permanent error” is set only if the meter signals a
	/// fatal device error (which requires a service action).
	/// Error can be reset only by a service action.
	permanent_error: bool,
	/// Warning — The bit “power low” is set only to signal interruption of
	/// external power supply or the end of battery life.
	power_low: bool,
	application: ApplicationError,
}

impl MeterStatus {
	fn parse(input: &mut &Bytes) -> PResult<MeterStatus> {
		binary::bits::bits::<_, _, InputError<_>, _, _>((
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
		.map_err(|err| {
			err.map(|err: InputError<_>| ContextError::from_error_kind(&err.input, err.kind))
		})
	}
}

#[derive(Debug)]
pub struct ShortHeader {
	access_number: u8,
	status: MeterStatus,
	configuration_field: u16,
	extra_header: Option<ExtraHeader>,
}

impl ShortHeader {
	pub fn parse(input: &mut &Bytes) -> PResult<TPLHeader> {
		Self::parse_raw.map(TPLHeader::Short).parse_next(input)
	}

	fn parse_raw(input: &mut &Bytes) -> PResult<ShortHeader> {
		(
			binary::u8.context(StrContext::Label("access number")),
			MeterStatus::parse.context(StrContext::Label("status")),
			binary::le_u16
				.context(StrContext::Label("tpl configuration field"))
				.verify(|v| {
					// TODO: This field can be many things that are not 0 but I
					// don't have any way of testing that behaviour so I'm just
					// going to ignore its existence
					*v == 0
				}),
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
	fn parse(input: &mut &Bytes) -> PResult<DeviceType> {
		binary::u8
			.map(|v| match v {
				0x00 => DeviceType::Other,
				0x01 => DeviceType::OilMeter,
				0x02 => DeviceType::ElectricityMeter,
				0x03 => DeviceType::GasMeter,
				0x04 => DeviceType::ThermalEnergyMeter(ThermalMeterType::OutletHeat),
				0x05 => DeviceType::SteamMeter,
				0x06 => DeviceType::WaterMeter(WaterMeterType::Warm),
				0x07 => DeviceType::WaterMeter(WaterMeterType::Potable),
				0x08 => DeviceType::HeatCostAllocator,
				// TODO
				_ => todo!(),
				// _ => DeviceType::Reserved,
			})
			.parse_next(input)
	}
}

#[derive(Debug)]
pub struct LongHeader {
	identifier: u32,
	manufacturer: String,
	device_name: Option<&'static str>,
	version: u8,
	device_type: DeviceType,
	access_number: u8,
	status: MeterStatus,
	configuration_field: u16,
	extra_header: Option<ExtraHeader>,
}

impl LongHeader {
	pub fn parse(input: &mut &Bytes) -> PResult<TPLHeader> {
		(
			binary::le_u32
				.context(StrContext::Label("device identifier"))
				.with_recognized(), // FIXME: This should be a BCD
			binary::le_u16
				.context(StrContext::Label("manufacturer"))
				.verify_map(|raw| {
					unpack_manufacturer_code(raw)
						.ok()
						.filter(|parsed| parsed.chars().all(|c| c.is_ascii_uppercase()))
						.map(|parsed| (parsed, raw))
				}),
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

#[derive(Debug)]
pub enum TPLHeader {
	None,
	Short(ShortHeader),
	Long(LongHeader),
}
