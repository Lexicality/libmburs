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
pub mod dib;
pub mod error;
pub mod manufacturer;
pub mod types;
pub mod vib;

use crate::parse::error::{ParseError, Result};

// This is going to get more complicated
pub struct Datagram {
    data: Vec<u8>,
    index: usize,
}

impl Datagram {
    pub fn new(data: Vec<u8>) -> Datagram {
        Datagram { data, index: 0 }
    }

    pub fn current(&self) -> Result<u8> {
        if self.index == 0 {
            return Err(ParseError::UnexpectedEOF);
        } else {
            self.data
                .get(self.index - 1)
                .map(|d| *d)
                .ok_or(ParseError::UnexpectedEOF)
        }
    }

    pub fn next(&mut self) -> Result<u8> {
        let ret = self.data.get(self.index);
        if let Some(ret) = ret {
            self.index += 1;
            return Ok(*ret);
        }
        return Err(ParseError::UnexpectedEOF);
    }

    pub fn take(&mut self, n: usize) -> Result<Vec<u8>> {
        if self.index + n >= self.data.len() {
            return Err(ParseError::UnexpectedEOF);
        }
        let start = self.index;
        self.index += n;
        Ok(self.data[start..self.index].into())
    }
}
