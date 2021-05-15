// Copyright 2021 lowRISC contributors.
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

//! Error protocol messages.

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

        /// The checksum on the message was invalid.
        BadChecksum = 0x01,

        /// The content type on the message is not supported.
        ContentTypeNotSupported = 0x02,
    }
}

/// A parsed header.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Header {
    /// The content type following the header.
    pub content: ContentType,
}

/// The length of a firmware header on the wire, in bytes.
pub const HEADER_LEN: usize = 1;

impl<'a> FromWire<'a> for Header {
    fn from_wire<R: Read<'a>>(mut r: R) -> Result<Self, FromWireError> {
        let content_u8 = r.read_be::<u8>()?;
        let content = ContentType::from_wire_value(content_u8).ok_or(FromWireError::OutOfRange)?;
        Ok(Self {
            content,
        })
    }
}

impl ToWire for Header {
    fn to_wire<W: Write>(&self, mut w: W) -> Result<(), ToWireError> {
        w.write_be(self.content.to_wire_value())?;
        Ok(())
    }
}

// ----------------------------------------------------------------------------

/// A message.
///
/// A message is identified by a [`ContentType`]:
///
/// This trait is not implemented by any of the message types
///
/// [`ContentType`]: enum.ContentType.html
pub trait Message<'req>: FromWire<'req> + ToWire {
    /// The unique [`ContentType`] for this `Message`.
    ///
    /// [`ContentType`]: enum.ContentType.html
    const TYPE: ContentType;
}

// ----------------------------------------------------------------------------

/// A parsed `bad checksum` message.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct BadChecksum {
}

/// The length of a `bad checksum` message on the wire, in bytes.
pub const BAD_CHECKSUM_LEN: usize = 0;

impl Message<'_> for BadChecksum {
    const TYPE: ContentType = ContentType::BadChecksum;
}

impl<'a> FromWire<'a> for BadChecksum {
    fn from_wire<R: Read<'a>>(mut _r: R) -> Result<Self, FromWireError> {
        Ok(Self {})
    }
}

impl ToWire for BadChecksum {
    fn to_wire<W: Write>(&self, mut _w: W) -> Result<(), ToWireError> {
        Ok(())
    }
}

// ----------------------------------------------------------------------------

/// A parsed `ContentTypeNotSupported` message.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct ContentTypeNotSupported {
}

/// The length of a `ContentTypeNotSupported` message on the wire, in bytes.
pub const CONTENT_TYPE_NOT_SUPPORTED_LEN: usize = 0;

impl Message<'_> for ContentTypeNotSupported {
    const TYPE: ContentType = ContentType::ContentTypeNotSupported;
}

impl<'a> FromWire<'a> for ContentTypeNotSupported {
    fn from_wire<R: Read<'a>>(mut _r: R) -> Result<Self, FromWireError> {
        Ok(Self {})
    }
}

impl ToWire for ContentTypeNotSupported {
    fn to_wire<W: Write>(&self, mut _w: W) -> Result<(), ToWireError> {
        Ok(())
    }
}

