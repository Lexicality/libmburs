/*
 * Copyright 2023 Lexi Robinson
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */
use crate::parse::error::{ParseError, Result};
use crate::parse::Datagram;

const MSD_MARKER: u8 = 0x0F;
const MSD_PLUS_MARKER: u8 = 0x1F;
const IDLE_FILLER: u8 = 0x2F;

const DIF_EXTENSION: u8 = 0b1000_0000;
const DIF_STORAGE: u8 = 0b0100_0000;
const DIF_FUNCTION: u8 = 0b0011_0000;
const DIF_DATA: u8 = 0b0000_1111;
const DIFE_DEVICE: u8 = 0b0100_0000;
const DIFE_TARIFF: u8 = 0b0011_0000;
const DIFE_STORAGE: u8 = 0b0000_1111;

const DIF_FUNCTION_MAX: u8 = 1 << 4;
const DIF_FUNCTION_MIN: u8 = 2 << 4;
const DIF_FUNCTION_ERR: u8 = 3 << 4;

pub enum RawDataType {
    None,
    Binary(usize),
    Real,
    BCD(usize),
    LVAR,
    MSD,
    MSDAndMore,
}

pub enum DataFunction {
    InstantaneousValue,
    MaximumValue,
    MinimumValue,
    ValueDuringErrorState,
}

#[allow(dead_code)]
pub struct DataInfoBlock {
    raw_type: RawDataType,
    function: DataFunction,
    storage: u64,
    // libmbus considers tariff/device missing to be different to 0, which is
    //  technically correct from a datagram perspective but from an actual spec
    //  perspective is mad
    // However, I'm trying to maintain compatability with it, so . . .
    tariff: Option<u32>,
    device: Option<u32>,
}

pub fn parse_dib(dg: &mut Datagram) -> Result<DataInfoBlock> {
    let dif = loop {
        let dif = dg.next()?;
        if dif != IDLE_FILLER {
            break dif;
        }
    };
    if dif == MSD_MARKER || dif == MSD_PLUS_MARKER {
        return Ok(DataInfoBlock {
            raw_type: match dif {
                MSD_MARKER => RawDataType::MSD,
                _ => RawDataType::MSDAndMore,
            },
            function: DataFunction::InstantaneousValue,
            storage: 0,
            device: None,
            tariff: None,
        });
    }
    let mut ret = DataInfoBlock {
        raw_type: parse_raw_type(dif)?,
        function: match dif & DIF_FUNCTION {
            DIF_FUNCTION_MIN => DataFunction::MinimumValue,
            DIF_FUNCTION_MAX => DataFunction::MaximumValue,
            DIF_FUNCTION_ERR => DataFunction::ValueDuringErrorState,
            _ => DataFunction::InstantaneousValue,
        },
        storage: ((dif & DIF_STORAGE) >> 6) as u64,
        device: None,
        tariff: None,
    };
    let mut has_extension = (dif & DIF_EXTENSION) != 0;
    while has_extension {
        let dife = dg.next()?;
        has_extension = (dife & DIF_EXTENSION) != 0;
        let dife_device = ((dife & DIFE_DEVICE) >> 6) as u32;
        let dife_tarif = ((dife & DIFE_TARIFF) >> 4) as u32;
        let dife_storage = (dife & DIFE_STORAGE) as u64;

        ret.storage <<= 4;
        ret.storage += dife_storage;
        ret.device = Some(ret.device.map_or(0, |d| d << 1) + dife_device);
        ret.tariff = Some(ret.tariff.map_or(0, |t| t << 2) + dife_tarif);
    }
    Ok(ret)
}

fn parse_raw_type(dif: u8) -> Result<RawDataType> {
    let data = dif & DIF_DATA;
    match data {
        0 => Ok(RawDataType::None),
        0b0001 | 0b0010 | 0b0011 | 0b0100 | 0b0110 => Ok(RawDataType::Binary(data as usize)),
        0b0101 => Ok(RawDataType::Real),
        0b0111 => Ok(RawDataType::Binary(8)),
        0b1001 | 0b1010 | 0b1011 | 0b1100 | 0b1110 => {
            Ok(RawDataType::BCD((data - 0b1000) as usize))
        }
        0b1101 => Ok(RawDataType::LVAR),
        // TODO: selection for readout (?)
        // TODO: global readout request (?)
        _ => Err(ParseError::UnsupportedDIF(dif)),
    }
}
