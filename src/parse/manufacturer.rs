/*
 * Copyright 2023 Lexi Robinson
 * Licensed under the EUPL-1.2
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
    Ok(String::from_iter(ret))
}

const fn pack_manufacturer_code(code: &'static str) -> u16 {
    let code = code.as_bytes();
    let [a, b, c] = *code else {
        panic!("Code must be 3 bytes")
    };
    assert!(
        (a as char).is_ascii_uppercase()
            && (b as char).is_ascii_uppercase()
            && (c as char).is_ascii_uppercase(),
        "Code must be 3 uppercase letters"
    );

    (a as u16 - 64) * 32 * 32 + (b as u16 - 64) * 32 + (c as u16 - 64)
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

const MEDIUM_ELECTRICITY: u8 = 0x02;
const MEDIUM_WARM_WATER: u8 = 0x06;
const MEDIUM_UNKNOWN: u8 = 0x0F;

pub fn device_name(
    raw_id: [u8; 4],
    manufacturer: u16,
    version: u8,
    medium: u8,
) -> Option<&'static str> {
    // HACK: Some manufacturers put the version in a different field
    let version = match manufacturer {
        SBC | SEO | GTE => raw_id[3],
        _ => version,
    };

    match (manufacturer, version, medium) {
        // ABB AB
        (ABB, 0x02, _) => Some("ABB Delta-Meter"),
        (ABB, 0x20, _) => Some("ABB B21 113-100"),
        // Actaris, France. (Water and Heat)
        (ACW, 0x09, _) => Some("Itron CF Echo 2"),
        (ACW, 0x0A, _) => Some("Itron CF 51"),
        (ACW, 0x0B, _) => Some("Itron CF 55"),
        (ACW, 0x0E, _) => Some("Itron BM +m"),
        (ACW, 0x0F, _) => Some("Itron CF 800"),
        (ACW, 0x14, _) => Some("Itron CYBLE M-Bus 1.4"),
        // INTEGRA METERING AG
        (AMT, 0x00..=0x3F, _) => Some("Aquametro AMTRON"),
        (AMT, 0x40..=0x7F, _) => Some("Aquametro SAPHIR"),
        (AMT, 0x80..=0xBF, _) => Some("Aquametro CALEC MB"),
        (AMT, 0xC0..=0xFF, _) => Some("Aquametro CALEC ST"),
        // ??? This manufacturer code is not registered
        (BEC, 0x00, MEDIUM_ELECTRICITY) => Some("Berg DCMi"),
        (BEC, 0x07, MEDIUM_ELECTRICITY) => Some("Berg BLMi"),
        (BEC, 0x71, MEDIUM_UNKNOWN) => Some("Berg BMB-10S0"),
        // Engelmann Sensor GmbH
        (EFE, 0x00, MEDIUM_WARM_WATER) => Some("Engelmann WaterStar"),
        (EFE, 0x00, _) => Some("Engelmann / Elster SensoStar 2"),
        (EFE, 0x01, _) => Some("Engelmann SensoStar 2C"),
        // Elster GmbH
        (ELS, 0x02, _) => Some("Elster TMP-A"),
        (ELS, 0x0A, _) => Some("Elster Falcon"),
        (ELS, 0x2F, _) => Some("Elster F96 Plus"),
        // Elvaco AB
        (ELV, 0x14..=0x1D, _) => Some("Elvaco CMa10"),
        (ELV, 0x32..=0x3B, _) => Some("Elvaco CMa11"),
        // EMH metering GmbH & Co. KG (formerly EMH Elektrizitatszahler GmbH & CO KG)
        (EMH, 0x00, _) => Some("EMH DIZ"),
        // EMU Elektronik AG
        (EMU, 0x10, MEDIUM_ELECTRICITY) => Some("EMU Professional 3/75 M-Bus"),
        // Carlo Gavazzi Controls S.p.A.
        (GAV, 0x2D..=0x30, MEDIUM_ELECTRICITY) => Some("Carlo Gavazzi EM24"),
        (GAV, 0x39 | 0x3A, MEDIUM_ELECTRICITY) => Some("Carlo Gavazzi EM21"),
        (GAV, 0x40, MEDIUM_ELECTRICITY) => Some("Carlo Gavazzi EM33"),
        // GMC-I Messtechnik GmbH
        (GMC, 0xE6, _) => Some("GMC-I A230 EMMOD 206"),
        // Hydrometer GmbH
        (HYD, 0x28, _) => Some("ABB F95 Typ US770"),
        (HYD, 0x2F, _) => Some("Hydrometer Sharky 775"),
        // Janitza electronics GmbH
        (JAN, 0x09, MEDIUM_ELECTRICITY) => Some("Janitza UMG 96S"),
        // Kamstrup Energi A/S
        (KAM, 0x01, _) => Some("Kamstrup 382 (6850-005)"),
        (KAM, 0x08, _) => Some("Kamstrup Multical 601"),
        // Landis & Staefa electronic
        (LSE, 0x99, _) => Some("Siemens WFH21"),
        // Landis+Gyr GmbH
        (LUG, 0x02, _) => Some("Landis & Gyr Ultraheat 2WR5"),
        (LUG, 0x03, _) => Some("Landis & Gyr Ultraheat 2WR6"),
        (LUG, 0x04, _) => Some("Landis & Gyr Ultraheat UH50"),
        (LUG, 0x07, _) => Some("Landis & Gyr Ultraheat T230"),
        // Nordwestdeutsche Zählerrevision Ing. Aug. Knemeyer GmbH & Co. KG
        (NZR, 0x01, _) => Some("NZR DHZ 5/63"),
        (NZR, 0x50, _) => Some("NZR IC-M2"),
        // Rossweiner Armaturen und Messgeräte GmbH & Co. OHG
        (RAM, 0x03, _) => Some("Rossweiner ETK/ETW Modularis"),
        // Relay GmbH
        (REL, 0x08, _) => Some("Relay PadPuls M1"),
        (REL, 0x12, _) => Some("Relay PadPuls M4"),
        (REL, 0x20, _) => Some("Relay Padin 4"),
        (REL, 0x30, _) => Some("Relay AnDi 4"),
        (REL, 0x40, _) => Some("Relay PadPuls M2"),
        // Viterra Energy Services (formerly Raab Karcher ES)
        (RKE, 0x69, _) => Some("Ista sensonic II mbus"),
        // Saia-Burgess Controls
        (SBC, 0x10 | 0x19, _) => Some("Saia-Burgess ALE3"),
        (SBC, 0x11, _) => Some("Saia-Burgess AWD3"),
        // Sensus Metering Systems
        (SEN, 0x08 | 0x19, _) => Some("Sensus PolluCom E"),
        (SEN, 0x0B, _) => Some("Sensus PolluTherm"),
        (SEN, 0x0E, _) => Some("Sensus PolluStat E"),
        // SENSOCO Greatech GmbH
        // GREATech GmbH
        (SEO | GTE, 0x30, _) => Some("Sensoco PT100"),
        (SEO | GTE, 0x41, _) => Some("Sensoco 2-NTC"),
        (SEO | GTE, 0x45, _) => Some("Sensoco Laser Light"),
        (SEO | GTE, 0x48, _) => Some("Sensoco ADIO"),
        (SEO | GTE, 0x51 | 0x61, _) => Some("Sensoco THU"),
        (SEO | GTE, 0x80, _) => Some("Sensoco PulseCounter for E-Meter"),
        // Schlumberger Industries Ltd.
        (SLB, 0x02, _) => Some("Allmess Megacontrol CF-50"),
        (SLB, 0x06, _) => Some("CF Compact / Integral MK MaXX"),
        // Sontex SA
        (SON, 0x0D, _) => Some("Sontex Supercal 531"),
        // Sensus Metering Systems
        (SPX, 0x31 | 0x34, _) => Some("Sensus PolluTherm"),
        // AB Svensk Värmemätning SVM
        (SVM, 0x08, _) => Some("Elster F2 / Deltamess F2"),
        (SVM, 0x09, _) => Some("Elster F4 / Kamstrup SVM F22"),
        // Techem Service AG & Co. KG
        (TCH, 0x26, _) => Some("Techem m-bus S"),
        (TCH, 0x40, _) => Some("Techem ultra S3"),
        // Neumann & Co. Wasserzähler Glaubitz GmbH
        (WZG, 0x03, _) => Some("Modularis ETW-EAX"),
        // ZENNER International GmbH & Co. KG
        (ZRM, 0x81, _) => Some("Minol Minocal C2"),
        (ZRM, 0x82, _) => Some("Minol Minocal WR3"),
        _ => None,
    }
}
