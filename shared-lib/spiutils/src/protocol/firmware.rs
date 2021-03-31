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

//! Firmware protocol payload.

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

        /// Request to prepare for an update
        UpdatePrepareRequest = 0x01,

        /// Response to PrepareRequest
        UpdatePrepareResponse = 0x02,

        /// Request to rrite a chunk of firmware
        WriteChunkRequest = 0x03,

        /// Response to WriteChunkRequest
        WriteChunkResponse = 0x04,
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

wire_enum! {
    /// The content type.
    pub enum SegmentAndLocation: u8 {
        /// Unknown message type.
        Unknown = 0xff,

        /// RO in location A
        RoA = 0x01,

        /// RO in location B
        RoB = 0x02,

        /// RW in location A
        RwA = 0x03,

        /// RW in location B
        RwB = 0x04,
    }
}

// ----------------------------------------------------------------------------

/// A parsed update prepare request.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct UpdatePrepareRequest {
    /// The segment and location.
    pub segment_and_location: SegmentAndLocation,
}

/// The length of a update prepare request on the wire, in bytes.
pub const UPDATE_PREPARE_REQUEST_LEN: usize = 1;

impl<'a> FromWire<'a> for UpdatePrepareRequest {
    fn from_wire<R: Read<'a>>(mut r: R) -> Result<Self, FromWireError> {
        let sal_u8 = r.read_be::<u8>()?;
        let segment_and_location = SegmentAndLocation::from_wire_value(sal_u8).ok_or(FromWireError::OutOfRange)?;
        Ok(Self {
            segment_and_location,
        })
    }
}

impl ToWire for UpdatePrepareRequest {
    fn to_wire<W: Write>(&self, mut w: W) -> Result<(), ToWireError> {
        w.write_be(self.segment_and_location.to_wire_value())?;
        Ok(())
    }
}

// ----------------------------------------------------------------------------

wire_enum! {
    /// The result of an update prepare request.
    pub enum UpdatePrepareResult: u8 {
        /// Unknown result type.
        Unknown = 0xff,

        /// Success
        Success = 0x00,

        /// Unspecified error
        Error = 0x01,

        /// Invalid segment and/or location
        InvalidSegmentAndLocation = 0x02,
    }
}

/// A parsed update prepare response.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct UpdatePrepareResponse {
    /// The segment and location.
    pub segment_and_location: SegmentAndLocation,

    /// The maximum chunk length per write.
    pub max_chunk_length: u16,

    /// The result of the update prepare request.
    pub result: UpdatePrepareResult,
}

/// The length of a update prepare response on the wire, in bytes.
pub const UPDATE_PREPARE_RESPONSE_LEN: usize = 4;

impl<'a> FromWire<'a> for UpdatePrepareResponse {
    fn from_wire<R: Read<'a>>(mut r: R) -> Result<Self, FromWireError> {
        let sal_u8 = r.read_be::<u8>()?;
        let segment_and_location = SegmentAndLocation::from_wire_value(sal_u8).ok_or(FromWireError::OutOfRange)?;
        let max_chunk_length = r.read_be::<u16>()?;
        let result_u8 = r.read_be::<u8>()?;
        let result = UpdatePrepareResult::from_wire_value(result_u8).ok_or(FromWireError::OutOfRange)?;
        Ok(Self {
            segment_and_location,
            max_chunk_length,
            result,
        })
    }
}

impl ToWire for UpdatePrepareResponse {
    fn to_wire<W: Write>(&self, mut w: W) -> Result<(), ToWireError> {
        w.write_be(self.segment_and_location.to_wire_value())?;
        w.write_be(self.max_chunk_length)?;
        w.write_be(self.result.to_wire_value())?;
        Ok(())
    }
}

// ----------------------------------------------------------------------------

/// A parsed write chunk request.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct WriteChunkRequest {
    /// The segment and location.
    pub segment_and_location: SegmentAndLocation,

    /// The offset within the segment,
    pub offset: u32,
}

/// The length of a write chunk request on the wire, in bytes.
pub const WRITE_CHUNK_REQUEST_LEN: usize = 5;

impl<'a> FromWire<'a> for WriteChunkRequest {
    fn from_wire<R: Read<'a>>(mut r: R) -> Result<Self, FromWireError> {
        let sal_u8 = r.read_be::<u8>()?;
        let segment_and_location = SegmentAndLocation::from_wire_value(sal_u8).ok_or(FromWireError::OutOfRange)?;
        let offset = r.read_be::<u32>()?;
        Ok(Self {
            segment_and_location,
            offset,
        })
    }
}

impl ToWire for WriteChunkRequest {
    fn to_wire<W: Write>(&self, mut w: W) -> Result<(), ToWireError> {
        w.write_be(self.segment_and_location.to_wire_value())?;
        w.write_be(self.offset)?;
        Ok(())
    }
}

// ----------------------------------------------------------------------------

wire_enum! {
    /// The result of an update prepare request.
    pub enum WriteChunkResult: u8 {
        /// Unknown result type.
        Unknown = 0xff,

        /// Success
        Success = 0x00,

        /// Unspecified error
        Error = 0x01,

        /// Invalid segment and/or location
        InvalidSegmentAndLocation = 0x02,

        /// Invalid offset
        InvalidOffset = 0x03,

        /// Too much data
        DataTooLong = 0x04,

        /// Post-write compare failed
        CompareFailed = 0x05,
    }
}

/// A parsed update prepare response.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct WriteChunkResponse {
    /// The segment and location.
    pub segment_and_location: SegmentAndLocation,

    /// The offset within the segment,
    pub offset: u32,

    /// The result of the write chunk request.
    pub result: WriteChunkResult,
}

/// The length of an update prepare response on the wire, in bytes.
pub const WRITE_CHUNK_RESPONSE_LEN: usize = 6;

impl<'a> FromWire<'a> for WriteChunkResponse {
    fn from_wire<R: Read<'a>>(mut r: R) -> Result<Self, FromWireError> {
        let sal_u8 = r.read_be::<u8>()?;
        let segment_and_location = SegmentAndLocation::from_wire_value(sal_u8).ok_or(FromWireError::OutOfRange)?;
        let offset = r.read_be::<u32>()?;
        let result_u8 = r.read_be::<u8>()?;
        let result = WriteChunkResult::from_wire_value(result_u8).ok_or(FromWireError::OutOfRange)?;
        Ok(Self {
            segment_and_location,
            offset,
            result,
        })
    }
}

impl ToWire for WriteChunkResponse {
    fn to_wire<W: Write>(&self, mut w: W) -> Result<(), ToWireError> {
        w.write_be(self.segment_and_location.to_wire_value())?;
        w.write_be(self.offset)?;
        w.write_be(self.result.to_wire_value())?;
        Ok(())
    }
}
