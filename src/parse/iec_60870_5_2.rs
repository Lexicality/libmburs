// Copyright 2024 Lexi Robinson
// Licensed under the EUPL-1.2

use winnow::binary::u8 as parse_u8;
use winnow::combinator::alt;
use winnow::error::ErrMode;
use winnow::error::ErrorKind;
use winnow::error::ParserError;
use winnow::prelude::*;
use winnow::stream::Stream;

#[derive(Debug)]
pub enum Packet<'a> {
    Ack,
    Data(DataPacket<'a>),
}

#[derive(Debug)]
pub struct DataPacket<'a> {
    pub control: u8,
    pub address: u8,
    pub data: &'a [u8],
}

fn parse_variable<'a>(input: &mut &'a [u8]) -> PResult<Packet<'a>> {
    0x68.parse_next(input)?;
    let length = parse_u8.parse_next(input)?;
    parse_u8.verify(|v| *v == length).parse_next(input)?;
    0x68.parse_next(input)?;
    let (control, address) = (parse_u8, parse_u8).parse_next(input)?;
    let length = length.into();
    // There are two bytes after the input
    if input.len() < length {
        return Err(ErrMode::from_error_kind(input, ErrorKind::Slice));
    }
    let data = input.next_slice(length - 2);
    let (checksum, _) = (parse_u8, 0x16).parse_next(input)?;

    let sum = data
        .iter()
        .copied()
        .reduce(u8::wrapping_add)
        .unwrap_or_default()
        .wrapping_add(control)
        .wrapping_add(address);

    if sum != checksum {
        return Err(ErrMode::from_error_kind(input, ErrorKind::Verify));
    }

    Ok(Packet::Data(DataPacket {
        control,
        address,
        data,
    }))
}

fn parse_fixed<'a>(input: &mut &'a [u8]) -> PResult<Packet<'a>> {
    // mbus's fixed length datagrams are 2 bytes long, only control & address
    let (_, control, address, checksum, _) =
        (0x10, parse_u8, parse_u8, parse_u8, 0x16).parse_next(input)?;

    let sum = control.wrapping_add(address);
    if sum != checksum {
        return Err(ErrMode::from_error_kind(input, ErrorKind::Verify));
    }

    Ok(Packet::Data(DataPacket {
        control,
        address,
        data: &[],
    }))
}

fn parse_ack<'a>(input: &mut &'a [u8]) -> PResult<Packet<'a>> {
    0xE5.map(|_| Packet::Ack).parse_next(input)
}

pub fn parse_packet<'a>(input: &mut &'a [u8]) -> PResult<Packet<'a>> {
    alt((parse_variable, parse_fixed, parse_ack)).parse_next(input)
}
