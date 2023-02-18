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
    VariableLengthNumber(Vec<u8>),
    ManufacturerSpecific(Vec<u8>),
}
