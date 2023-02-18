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
 *
 * Much of the code in this file is based on code from the rSCADA/libmbus
 * project by Raditex Control AB (c) 2010-2012
 */
use crate::parse::error::{ParseError, Result};

fn characterise(c: u32) -> Result<char> {
    let c = (c & 0x1F) + 64;
    let c = char::from_u32(c);
    if let Some(c) = c {
        if c.is_ascii_uppercase() {
            Ok(c)
        } else {
            Err(ParseError::DecodeError(
                "Unexpected character in manufacturer code",
            ))
        }
    } else {
        Err(ParseError::DecodeError(
            "Invalid character in manufacturer code",
        ))
    }
}

pub fn unpack_manufacturer_code(packed: u16) -> Result<String> {
    let packed = packed as u32;
    let ret = [
        characterise(packed >> 10)?,
        characterise(packed >> 2)?,
        characterise(packed)?,
    ];
    return Ok(String::from_iter(ret));
}

const fn pack_manufacturer_code(code: &'static str) -> u16 {
    let code = code.as_bytes();
    assert!(code.len() == 3);
    let a = code[0];
    let b = code[1];
    let c = code[2];
    assert!(
        (a as char).is_ascii_uppercase()
            && (b as char).is_ascii_uppercase()
            && (c as char).is_ascii_uppercase()
    );

    return (a as u16 - 64) * 32 * 32 + (b as u16 - 64) * 32 + (c as u16 - 64);
}

// Rust, anonyingly, doesn't suport const function expressions in match statements
const ABB: u16 = pack_manufacturer_code("ABB");
const ACW: u16 = pack_manufacturer_code("ACW");
const AMT: u16 = pack_manufacturer_code("AMT");
const BEC: u16 = pack_manufacturer_code("BEC");
const EFE: u16 = pack_manufacturer_code("EFE");
const ELS: u16 = pack_manufacturer_code("ELS");
const ELV: u16 = pack_manufacturer_code("ELV");
const EMH: u16 = pack_manufacturer_code("EMH");
const EMU: u16 = pack_manufacturer_code("EMU");
const GAV: u16 = pack_manufacturer_code("GAV");
const GMC: u16 = pack_manufacturer_code("GMC");
const KAM: u16 = pack_manufacturer_code("KAM");
const SLB: u16 = pack_manufacturer_code("SLB");
const HYD: u16 = pack_manufacturer_code("HYD");
const JAN: u16 = pack_manufacturer_code("JAN");
const LUG: u16 = pack_manufacturer_code("LUG");
const LSE: u16 = pack_manufacturer_code("LSE");
const NZR: u16 = pack_manufacturer_code("NZR");
const RAM: u16 = pack_manufacturer_code("RAM");
const REL: u16 = pack_manufacturer_code("REL");
const RKE: u16 = pack_manufacturer_code("RKE");
const SBC: u16 = pack_manufacturer_code("SBC");
const SEO: u16 = pack_manufacturer_code("SEO");
const GTE: u16 = pack_manufacturer_code("GTE");
const SEN: u16 = pack_manufacturer_code("SEN");
const SON: u16 = pack_manufacturer_code("SON");
const SPX: u16 = pack_manufacturer_code("SPX");
const SVM: u16 = pack_manufacturer_code("SVM");
const TCH: u16 = pack_manufacturer_code("TCH");
const WZG: u16 = pack_manufacturer_code("WZG");
const ZRM: u16 = pack_manufacturer_code("ZRM");

const MBUS_VARIABLE_DATA_MEDIUM_ELECTRICITY: u8 = 0; // TODO
const MBUS_VARIABLE_DATA_MEDIUM_UNKNOWN: u8 = 1; // TODO

pub fn device_name(
    raw_id: [u8; 4],
    manufacturer: u16,
    version: u8,
    medium: u8,
) -> Option<&'static str> {
    match manufacturer {
        ABB => match version {
            0x02 => Some("ABB Delta-Meter"),
            0x20 => Some("ABB B21 113-100"),
            _ => None,
        },
        ACW => match version {
            0x09 => Some("Itron CF Echo 2"),
            0x0A => Some("Itron CF 51"),
            0x0B => Some("Itron CF 55"),
            0x0E => Some("Itron BM +m"),
            0x0F => Some("Itron CF 800"),
            0x14 => Some("Itron CYBLE M-Bus 1.4"),
            _ => None,
        },
        AMT => match version {
            0xC0..=0xFF => Some("Aquametro CALEC ST"),
            0x80..=0xBF => Some("Aquametro CALEC MB"),
            0x40..=0x7F => Some("Aquametro SAPHIR"),
            0x00..=0x3F => Some("Aquametro AMTRON"),
        },
        BEC => match medium {
            MBUS_VARIABLE_DATA_MEDIUM_ELECTRICITY => match version {
                0x00 => Some("Berg DCMi"),
                0x07 => Some("Berg BLMi"),
                _ => None,
            },
            MBUS_VARIABLE_DATA_MEDIUM_UNKNOWN => match version {
                0x71 => Some("Berg BMB-10S0"),
                _ => None,
            },
            _ => None,
        },
        EFE => match version {
            0x00 => match medium {
                0x06 => Some("Engelmann WaterStar"),
                _ => Some("Engelmann / Elster SensoStar 2"),
            },
            0x01 => Some("Engelmann SensoStar 2C"),
            _ => None,
        },
        ELS => match version {
            0x02 => Some("Elster TMP-A"),
            0x0A => Some("Elster Falcon"),
            0x2F => Some("Elster F96 Plus"),
            _ => None,
        },
        ELV => match version {
            0x14 | 0x15 | 0x16 | 0x17 | 0x18 | 0x19 | 0x1A | 0x1B | 0x1C | 0x1D => {
                Some("Elvaco CMa10")
            }
            0x32 | 0x33 | 0x34 | 0x35 | 0x36 | 0x37 | 0x38 | 0x39 | 0x3A | 0x3B => {
                Some("Elvaco CMa11")
            }
            _ => None,
        },
        EMH => match version {
            0x00 => Some("EMH DIZ"),
            _ => None,
        },
        EMU => match medium {
            MBUS_VARIABLE_DATA_MEDIUM_ELECTRICITY => match version {
                0x10 => Some("EMU Professional 3/75 M-Bus"),
                _ => None,
            },
            _ => None,
        },
        GAV => match medium {
            MBUS_VARIABLE_DATA_MEDIUM_ELECTRICITY => match version {
                0x2D | 0x2E | 0x2F | 0x30 => Some("Carlo Gavazzi EM24"),
                0x39 | 0x3A => Some("Carlo Gavazzi EM21"),
                0x40 => Some("Carlo Gavazzi EM33"),
                _ => None,
            },
            _ => None,
        },
        GMC => match version {
            0xE6 => Some("GMC-I A230 EMMOD 206"),
            _ => None,
        },
        KAM => match version {
            0x01 => Some("Kamstrup 382 (6850-005)"),
            0x08 => Some("Kamstrup Multical 601"),
            _ => None,
        },
        SLB => match version {
            0x02 => Some("Allmess Megacontrol CF-50"),
            0x06 => Some("CF Compact / Integral MK MaXX"),
            _ => None,
        },
        HYD => match version {
            0x28 => Some("ABB F95 Typ US770"),
            0x2F => Some("Hydrometer Sharky 775"),
            _ => None,
        },
        JAN => match medium {
            MBUS_VARIABLE_DATA_MEDIUM_ELECTRICITY => match version {
                0x09 => Some("Janitza UMG 96S"),
                _ => None,
            },
            _ => None,
        },
        LUG => match version {
            0x02 => Some("Landis & Gyr Ultraheat 2WR5"),
            0x03 => Some("Landis & Gyr Ultraheat 2WR6"),
            0x04 => Some("Landis & Gyr Ultraheat UH50"),
            0x07 => Some("Landis & Gyr Ultraheat T230"),
            _ => None,
        },
        LSE => match version {
            0x99 => Some("Siemens WFH21"),
            _ => None,
        },
        NZR => match version {
            0x01 => Some("NZR DHZ 5/63"),
            0x50 => Some("NZR IC-M2"),
            _ => None,
        },
        RAM => match version {
            0x03 => Some("Rossweiner ETK/ETW Modularis"),
            _ => None,
        },
        REL => match version {
            0x08 => Some("Relay PadPuls M1"),
            0x12 => Some("Relay PadPuls M4"),
            0x20 => Some("Relay Padin 4"),
            0x30 => Some("Relay AnDi 4"),
            0x40 => Some("Relay PadPuls M2"),
            _ => None,
        },
        RKE => match version {
            0x69 => Some("Ista sensonic II mbus"),
            _ => None,
        },
        SBC => match raw_id[3] {
            0x10 | 0x19 => Some("Saia-Burgess ALE3"),
            0x11 => Some("Saia-Burgess AWD3"),
            _ => None,
        },
        SEO | GTE => match raw_id[3] {
            0x30 => Some("Sensoco PT100"),
            0x41 => Some("Sensoco 2-NTC"),
            0x45 => Some("Sensoco Laser Light"),
            0x48 => Some("Sensoco ADIO"),
            0x51 | 0x61 => Some("Sensoco THU"),
            0x80 => Some("Sensoco PulseCounter for E-Meter"),
            _ => None,
        },
        SEN => match version {
            0x08 | 0x19 => Some("Sensus PolluCom E"),
            0x0B => Some("Sensus PolluTherm"),
            0x0E => Some("Sensus PolluStat E"),
            _ => None,
        },
        SON => match version {
            0x0D => Some("Sontex Supercal 531"),
            _ => None,
        },
        SPX => match version {
            0x31 | 0x34 => Some("Sensus PolluTherm"),
            _ => None,
        },
        SVM => match version {
            0x08 => Some("Elster F2 / Deltamess F2"),
            0x09 => Some("Elster F4 / Kamstrup SVM F22"),
            _ => None,
        },
        TCH => match version {
            0x26 => Some("Techem m-bus S"),
            0x40 => Some("Techem ultra S3"),
            _ => None,
        },
        WZG => match version {
            0x03 => Some("Modularis ETW-EAX"),
            _ => None,
        },
        ZRM => match version {
            0x81 => Some("Minol Minocal C2"),
            0x82 => Some("Minol Minocal WR3"),
            _ => None,
        },
        _ => None,
    }
}
