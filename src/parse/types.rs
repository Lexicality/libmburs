/*
 * Copyright 2023 Lexi Robinson
 * Licensed under the EUPL-1.2
 */
use super::error::Result;

pub mod date;
pub mod lvar;
pub mod number;

// Note to self, enums always take up the maxmium size so there's no reason to
// store any of the smaller integer types
#[derive(Debug, PartialEq)]
pub enum DataType {
    Unsigned(u64),                  // Type A, C
    Signed(i64),                    // Type A, B
    Bool(bool),                     // Type D
    Real(f32),                      // Type H
    DateTimeF(date::TypeFDateTime), // Type F
    DateTimeI(date::TypeIDateTime), // type I
    Date(date::TypeGDate),          // type G
    Time(date::TypeJTime),          // Type J
    String(String),
    Invalid(Vec<u8>),
    VariableLengthNumber(Vec<u8>),
    ManufacturerSpecific(Vec<u8>),
}

pub type ParseResult = Result<DataType>;
