// Copyright 2023 Lexi Robinson
// Licensed under the EUPL-1.2

pub mod application_layer;
pub mod error;
pub mod link_layer;
pub mod transport_layer;
pub mod types;

use crate::parse::error::{ParseError, Result};

#[allow(dead_code)]
pub struct Datagram {
	data: Vec<u8>,
	index: usize,
	/// IEC 60780 control field
	packet_control: u8,
	/// IEC 60780 address field
	address: u8,
	/// IEC 13757 (mbus) control information (CI) field
	mbus_control: u8,
}

impl Datagram {
	pub fn new(data: Vec<u8>, packet_control: u8, address: u8, mbus_control: u8) -> Self {
		Self {
			data,
			index: 0,
			packet_control,
			address,
			mbus_control,
		}
	}

	pub fn parse(data: Vec<u8>) -> Result<Self> {
		let len = data.len();
		if len == 0 {
			return Err(ParseError::InvalidPacket("Packet is empty"));
		}
		let start1 = data[0];
		match start1 {
			0x68 => (),
			// TODO: Figure out where these are defined and why libmbus supports them,
			//  because they're not in IEC 60870-5-2
			0xE5 => todo!("ACK packets aren't supported yet"),
			0x10 => todo!("Short packets aren't supported yet"),
			_ => return Err(ParseError::InvalidPacket("Start byte is invalid")),
		}

		let [_, length1, length2, start2, packet_control, address, mbus_control, .., checksum, end] =
			data[..]
		else {
			return Err(ParseError::InvalidPacket("Packet is too short"));
		};
		if start1 != 0x68 {
			return Err(ParseError::InvalidPacket("Start byte is invalid"));
		} else if length1 != length2 {
			return Err(ParseError::InvalidPacket("Lengths don't match"));
		} else if length1 as usize != len - 6 {
			return Err(ParseError::InvalidPacket("Packet length incorrect"));
		} else if start2 != 0x68 {
			return Err(ParseError::InvalidPacket("Second start byte is invalid"));
		} else if checksum
			!= data
				.iter()
				.skip(4)
				.take(length1 as usize)
				.copied()
				.reduce(u8::wrapping_add)
				.unwrap_or(0)
		{
			return Err(ParseError::InvalidPacket("Checksum doesn't match"));
		} else if end != 0x16 {
			return Err(ParseError::InvalidPacket("End byte is invalid"));
		}
		Ok(Self::new(
			(data[6..len - 2]).into(),
			packet_control,
			address,
			mbus_control,
		))
	}

	pub fn get_byte(&self, index: usize) -> Result<u8> {
		self.data
			.get(index)
			.copied()
			.ok_or(ParseError::UnexpectedEOF)
	}

	pub fn peek(&self) -> Result<u8> {
		self.get_byte(self.index)
	}

	pub fn last_byte(&self) -> Result<u8> {
		if self.index > 0 {
			self.get_byte(self.index - 1)
		} else {
			Err(ParseError::UnexpectedEOF)
		}
	}

	pub fn next_byte(&mut self) -> Result<u8> {
		let ret = self.get_byte(self.index);
		if ret.is_ok() {
			self.index += 1;
		}
		ret
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
