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
use encoding_rs::mem::decode_latin1;

use crate::parse::dib::RawDataType;
use crate::parse::error::{ParseError, Result};
use crate::parse::Datagram;

use super::{DataType, ParseResult};

pub fn parse_lvar(_dt: RawDataType, dg: &mut Datagram) -> ParseResult {
    let length = dg.next()?;
    match length {
        0xC0..=0xC9 => parse_positive_bcd(length - 0xC0, dg), // Positive BCD number
        0xD0..=0xD9 => parse_negative_bcd(length - 0xD0, dg), // Negative BCD number
        0xE0..=0xEF => parse_binary(length - 0xE0, dg),       // Binary Number
        0xF0..=0xF4 => parse_binary(4 * (length - 0xEC), dg), // Big Binary Number
        0xF5 => parse_binary(48, dg),                         // Really Big Binary Number
        0xF6 => parse_binary(64, dg),                         // Unreasonably Big Binary Number
        0x00..=0xBF => parse_string(length, dg),              // Latin-1 String
        _ => Err(ParseError::InvalidData("Unsupported LVAR value")),
    }
}

fn parse_binary(len: u8, dg: &mut Datagram) -> ParseResult {
    if len <= 8 {
        super::number::parse_number(RawDataType::BinarySigned(len as usize), dg)
    } else {
        Ok(DataType::VariableLengthNumber(dg.take(len as usize)?))
    }
}

fn parse_string(len: u8, dg: &mut Datagram) -> ParseResult {
    Ok(DataType::String(decode_string(dg.take(len as usize)?)?))
}

fn parse_positive_bcd(len: u8, dg: &mut Datagram) -> ParseResult {
    super::number::parse_number(RawDataType::BCD(len as usize), dg)
}

fn parse_negative_bcd(len: u8, dg: &mut Datagram) -> ParseResult {
    match parse_positive_bcd(len, dg)? {
        DataType::Signed(mut ret) => {
            if ret > 0 {
                ret *= -1;
            }
            Ok(DataType::Signed(ret))
        }
        DataType::Unsigned(ret) => Ok(DataType::Signed(-(ret as i64))),
        _ => Err(ParseError::DataTypeMismatch),
    }
}

pub fn decode_string(mut data: Vec<u8>) -> Result<String> {
    data.reverse();
    let res = decode_latin1(&data);
    return Ok(res.into_owned());
}
