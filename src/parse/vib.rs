// Copyright 2023 Lexi Robinson
// Licensed under the EUPL-1.2

use crate::parse::error::{ParseError, Result};
use crate::parse::types::lvar::decode_string;
use crate::parse::Datagram;

const VIF_EXTENSION: u8 = 0b1000_0000;
const VIF_VALUE: u8 = !VIF_EXTENSION;

#[allow(dead_code)]
pub struct ValueInfoBlock {
    value_type: ValueType,
    extra_vifes: Option<Vec<u8>>,
}

pub fn parse_vib(dg: &mut Datagram) -> Result<ValueInfoBlock> {
    let vif = dg.next_byte()?;
    let mut value_type = match vif {
        0x7C | 0xFC => ValueType::PlainText("".to_string()), // Plain text VIF
        0x7E | 0xFE => return Err(ParseError::UnsupportedVIF(vif)), // "Any VIF"
        0x7F | 0xFF => ValueType::ManufacturerSpecific,      // Manufacturer specific
        0xFD => parse_extension_1(dg)?,                      // Linear VIF-extension 1
        0xFB => parse_extension_2(dg)?,                      // Linear VIF-extension 2
        0xEF => return Err(ParseError::UnsupportedVIF(vif)), // Linear VIF-extension 3
        _ => parse_primary_vif(vif)?,
    };

    // parsing extensions will move the pointer along
    let has_extension = (dg.last_byte()? & VIF_EXTENSION) != 0;
    // TODO: Support additional VIFE frames
    let extra_vifes = match has_extension {
        true => Some(dump_vifes(dg)?),
        false => None,
    };

    // TODO: words / once the vife is over we get the vif out of the data
    if let ValueType::PlainText(_) = value_type {
        let length = dg.next_byte()?;
        value_type = ValueType::PlainText(decode_string(dg.take(length as usize)?)?);
    }

    Ok(ValueInfoBlock {
        value_type,
        extra_vifes,
    })
}

fn parse_primary_vif(vif: u8) -> Result<ValueType> {
    let _value = vif & VIF_VALUE;
    todo!()
}

fn parse_extension_1(dg: &mut Datagram) -> Result<ValueType> {
    let _value = dg.next_byte()? & VIF_VALUE;
    todo!()
}

fn parse_extension_2(dg: &mut Datagram) -> Result<ValueType> {
    let _value = dg.next_byte()? & VIF_VALUE;
    todo!()
}

fn dump_vifes(dg: &mut Datagram) -> Result<Vec<u8>> {
    let mut ret = Vec::new();
    loop {
        let vife = dg.next_byte()?;
        ret.push(vife);
        if (vife & VIF_EXTENSION) == 0 {
            break;
        }
    }
    Ok(ret)
}

pub enum DurationType {
    Seconds,
    Minutes,
    Hours,
    Days,
    Months,
    Years,
}

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

pub enum ValueType {
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
