// Copyright 2020 lowRISC contributors.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//
// SPDX-License-Identifier: Apache-2.0

//! SPI flash protocol payload.

use crate::io::Read;
use crate::io::Write;
use crate::protocol::wire::FromWireError;
use crate::protocol::wire::FromWire;
use crate::protocol::wire::ToWireError;
use crate::protocol::wire::ToWire;
use crate::protocol::wire::WireEnum;

/// Data for CRC8 implementation.
struct Crc8 {
    crc: u16,
}

/// The CRC8 implementation.
impl Crc8 {
    /// Initialize CRC8 data to 0.
    pub fn init() -> Self {
        Self {
            crc: 0,
        }
    }

    /// Get the calculated CRC8 checksum.
    pub fn get(&self) -> u8 {
        (self.crc >> 8 & 0xff) as u8
    }

    /// Adds the specified data to the CRC8 checksum.
    /// Taken from
    /// https://chromium.googlesource.com/chromiumos/platform/vboot_reference/+/stabilize2/firmware/lib/crc8.c
    /// Uses x^8+x^2+x+1 polynomial.
    pub fn add(&mut self, data: &[u8]) -> &mut Self {
        for byte in data {
            self.crc ^= (*byte as u16) << 8;
            for _ in 0..8 {
                if self.crc & 0x8000 != 0 {
                    self.crc ^= 0x1070 << 3;
                }
                self.crc <<= 1;
            }
        }

        self
    }
}

/// Compute the checksum of the given header and payload buffer.
pub fn compute_checksum(header: &Header, payload: &[u8]) -> u8 {
    Crc8::init()
        .add(&[header.content.to_wire_value()])
        .add(&header.content_len.to_be_bytes())
        .add(&payload[..header.content_len as usize])
        .get()
}

wire_enum! {
    /// The content type.
    pub enum ContentType: u8 {
        /// Unknown message type.
        Unknown = 0xff,

        /// Error
        Error = 0x00,

        /// Manticore
        Manticore = 0x01,

        /// Firmware
        Firmware = 0x02,
    }
}

/// A parsed header.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Header {
    /// The content type following the header.
    pub content: ContentType,

    /// The length of the content following the header.
    pub content_len: u16,

    /// A checksum including the header (excluding this field)
    // and the content following the header.
    pub checksum: u8,
}

/// The length of a payload header on the wire, in bytes.
pub const HEADER_LEN: usize = 4;

impl<'a> FromWire<'a> for Header {
    fn from_wire<R: Read<'a>>(mut r: R) -> Result<Self, FromWireError> {
        let content_u8 = r.read_be::<u8>()?;
        let content = ContentType::from_wire_value(content_u8).ok_or(FromWireError::OutOfRange)?;
        let content_len = r.read_be::<u16>()?;
        let checksum = r.read_be::<u8>()?;
        Ok(Self {
            content,
            content_len,
            checksum,
        })
    }
}

impl ToWire for Header {
    fn to_wire<W: Write>(&self, mut w: W) -> Result<(), ToWireError> {
        w.write_be(self.content.to_wire_value())?;
        w.write_be(self.content_len)?;
        w.write_be(self.checksum)?;
        Ok(())
    }
}
