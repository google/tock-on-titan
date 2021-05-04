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

wire_enum! {
    /// The content type.
    pub enum ContentType: u8 {
        /// Unknown message type.
        Unknown = 0xff,

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
}

/// The length of a payload header on the wire, in bytes.
pub const HEADER_LEN: usize = 3;

impl<'a> FromWire<'a> for Header {
    fn from_wire<R: Read<'a>>(mut r: R) -> Result<Self, FromWireError> {
        let content_u8 = r.read_be::<u8>()?;
        let content = ContentType::from_wire_value(content_u8).ok_or(FromWireError::OutOfRange)?;
        let content_len = r.read_be::<u16>()?;
        Ok(Self {
            content,
            content_len,
        })
    }
}

impl ToWire for Header {
    fn to_wire<W: Write>(&self, mut w: W) -> Result<(), ToWireError> {
        w.write_be(self.content.to_wire_value())?;
        w.write_be(self.content_len)?;
        Ok(())
    }
}
