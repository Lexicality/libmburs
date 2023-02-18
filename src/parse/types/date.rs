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

#[allow(dead_code)]
#[derive(Debug, PartialEq, Eq)]
pub struct TypeFDateTime {
    minute: u8,
    hour: u8,
    day: u8,
    month: u8,
    year: u8,
    hundred_year: u8,
    invalid: bool,
    dst: bool,
}

#[allow(dead_code)]
#[derive(Debug, PartialEq, Eq)]
pub struct TypeGDate {
    day: u8,
    month: u8,
    year: u8,
}

#[allow(dead_code)]
#[derive(Debug, PartialEq, Eq)]
pub struct TypeIDateTime {
    second: u8,
    minute: u8,
    hour: u8,
    day: u8,
    month: u8,
    year: u8,
    day_of_week: u8,
    week: u8,
    invalid: bool,
    in_dst: bool,
    leap_year: bool,
    dst_offset: i8,
}

#[allow(dead_code)]
#[derive(Debug, PartialEq, Eq)]
pub struct TypeJTime {
    second: u8,
    minute: u8,
    hour: u8,
    invalid: bool,
}
