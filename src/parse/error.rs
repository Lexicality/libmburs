/*
 * Copyright 2023 Lexi Robinson
 * Licensed under the EUPL-1.2
 */
use std::{error, fmt};

#[derive(Debug, PartialEq, Eq)]
pub enum ParseError {
    // TODO: Probably need to go back and rationalise these error types after all the
    // parsing is done since I've just added new ones at random
    // TODO: This really should have the byte location embedded in it to make debugging
    // way easier
    InvalidPacket(&'static str),
    DecodeError(&'static str),
    InvalidData(&'static str),
    DataTypeMismatch,
    UnsupportedDIF(u8),
    UnsupportedVIF(u8),
    UnsupportedVIFE(u8),
    UnexpectedEOF,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPacket(e) => write!(f, "invalid packet: {}", e),
            Self::DecodeError(e) => write!(f, "error decoding data: {}", e),
            Self::InvalidData(e) => write!(f, "data is invalid: {}", e),
            Self::DataTypeMismatch => write!(f, "data type mismatch"),
            Self::UnsupportedDIF(v) => write!(f, "unsupported data information field: {:X}", v),
            Self::UnsupportedVIF(v) => write!(f, "unsupported value information field: {:X}", v),
            Self::UnsupportedVIFE(v) => {
                write!(f, "unsupported value information field extension: {:X}", v)
            }
            Self::UnexpectedEOF => write!(f, "unexpected EOF while parsing"),
        }
    }
}

impl error::Error for ParseError {}

pub type Result<T> = std::result::Result<T, ParseError>;
