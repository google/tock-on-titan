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

use crate::compat::firmware::BuildInfo;
use crate::compat::firmware::BUILD_INFO_LEN;
use crate::driver::firmware::SegmentInfo;
use crate::driver::firmware::SEGMENT_INFO_LEN;
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
        /// Request to prepare for an update
        UpdatePrepareRequest = 0x01,

        /// Response to PrepareRequest
        UpdatePrepareResponse = 0x02,

        /// Request to write a chunk of firmware
        WriteChunkRequest = 0x03,

        /// Response to WriteChunkRequest
        WriteChunkResponse = 0x04,

        /// Request information on inactive segments
        InactiveSegmentsInfoRequest = 0x05,

        /// Response to InactiveSegmentsInfoRequest
        InactiveSegmentsInfoResponse = 0x06,

        /// Request to reboot
        RebootRequest = 0x07,

        /// Response to RebootRequest
        RebootResponse = 0x08,
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

wire_enum! {
    /// Identifier for a segment and location.
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

/// A parsed inactive segments info request.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct InactiveSegmentsInfoRequest {
}

/// The length of an inactive segments info request on the wire, in bytes.
pub const INACTIVE_SEGMENTS_INFO_REQUEST_LEN: usize = 0;

impl Message<'_> for InactiveSegmentsInfoRequest {
    const TYPE: ContentType = ContentType::InactiveSegmentsInfoRequest;
}

impl<'a> FromWire<'a> for InactiveSegmentsInfoRequest {
    fn from_wire<R: Read<'a>>(mut _r: R) -> Result<Self, FromWireError> {
        Ok(Self {})
    }
}

impl ToWire for InactiveSegmentsInfoRequest {
    fn to_wire<W: Write>(&self, mut _w: W) -> Result<(), ToWireError> {
        Ok(())
    }
}

// ----------------------------------------------------------------------------

/// A parsed inactive segments info response.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct InactiveSegmentsInfoResponse {
    /// The inactive RO.
    pub ro: SegmentInfo,

    /// The inactive RW.
    pub rw: SegmentInfo,
}

/// The length of an inactive segments info response on the wire, in bytes.
pub const INACTIVE_SEGMENTS_INFO_RESPONSE_LEN: usize = 2 * SEGMENT_INFO_LEN;

impl Message<'_> for InactiveSegmentsInfoResponse {
    const TYPE: ContentType = ContentType::InactiveSegmentsInfoResponse;
}

impl<'a> FromWire<'a> for InactiveSegmentsInfoResponse {
    fn from_wire<R: Read<'a>>(mut r: R) -> Result<Self, FromWireError> {
        let ro = SegmentInfo::from_wire(&mut r)?;
        let rw = SegmentInfo::from_wire(&mut r)?;
        Ok(Self {
            ro,
            rw,
        })
    }
}

impl ToWire for InactiveSegmentsInfoResponse {
    fn to_wire<W: Write>(&self, mut w: W) -> Result<(), ToWireError> {
        self.ro.to_wire(&mut w)?;
        self.rw.to_wire(&mut w)?;
        Ok(())
    }
}

// ----------------------------------------------------------------------------

/// A parsed firmware info message.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct FirmwareInfo {
    /// The segment and location.
    pub segment_and_location: SegmentAndLocation,

    /// The build information.
    pub build_info: BuildInfo,
}

/// The length of a firmware info struct on the wire, in bytes.
pub const FIRMWARE_INFO_LEN: usize = 1 + BUILD_INFO_LEN;

impl<'a> FromWire<'a> for FirmwareInfo {
    fn from_wire<R: Read<'a>>(mut r: R) -> Result<Self, FromWireError> {
        let sal_u8 = r.read_be::<u8>()?;
        let segment_and_location = SegmentAndLocation::from_wire_value(sal_u8).ok_or(FromWireError::OutOfRange)?;
        let build_info = BuildInfo::from_wire(r)?;
        Ok(Self {
            segment_and_location,
            build_info,
        })
    }
}

impl ToWire for FirmwareInfo {
    fn to_wire<W: Write>(&self, mut w: W) -> Result<(), ToWireError> {
        w.write_be(self.segment_and_location.to_wire_value())?;
        self.build_info.to_wire(w)?;
        Ok(())
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

impl Message<'_> for UpdatePrepareRequest {
    const TYPE: ContentType = ContentType::UpdatePrepareRequest;
}

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

impl Message<'_> for UpdatePrepareResponse {
    const TYPE: ContentType = ContentType::UpdatePrepareResponse;
}

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
pub struct WriteChunkRequest<'a> {
    /// The segment and location.
    pub segment_and_location: SegmentAndLocation,

    /// The offset within the segment.
    pub offset: u32,

    /// The data to write
    pub data: &'a [u8],
}

/// The length of a write chunk request on the wire, in bytes.
pub const WRITE_CHUNK_REQUEST_LEN: usize = 5;

impl<'a> Message<'a> for WriteChunkRequest<'a> {
    const TYPE: ContentType = ContentType::WriteChunkRequest;
}

impl<'a> FromWire<'a> for WriteChunkRequest<'a> {
    fn from_wire<R: Read<'a>>(mut r: R) -> Result<Self, FromWireError> {
        let sal_u8 = r.read_be::<u8>()?;
        let segment_and_location = SegmentAndLocation::from_wire_value(sal_u8).ok_or(FromWireError::OutOfRange)?;
        let offset = r.read_be::<u32>()?;
        let data_len = r.remaining_data();
        let data = r.read_bytes(data_len)?;
        Ok(Self {
            segment_and_location,
            offset,
            data,
        })
    }
}

impl ToWire for WriteChunkRequest<'_> {
    fn to_wire<W: Write>(&self, mut w: W) -> Result<(), ToWireError> {
        w.write_be(self.segment_and_location.to_wire_value())?;
        w.write_be(self.offset)?;
        w.write_bytes(self.data)?;
        Ok(())
    }
}

// ----------------------------------------------------------------------------

wire_enum! {
    /// The result of a write chunk request.
    pub enum WriteChunkResult: u8 {
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

/// A parsed write chunk response.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct WriteChunkResponse {
    /// The segment and location.
    pub segment_and_location: SegmentAndLocation,

    /// The offset within the segment,
    pub offset: u32,

    /// The result of the write chunk request.
    pub result: WriteChunkResult,
}

/// The length of an write chunk response on the wire, in bytes.
pub const WRITE_CHUNK_RESPONSE_LEN: usize = 6;

impl Message<'_> for WriteChunkResponse {
    const TYPE: ContentType = ContentType::WriteChunkResponse;
}

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

// ----------------------------------------------------------------------------

wire_enum! {
    /// When to perform the reboot.
    pub enum RebootTime: u8 {
        /// Reboot immediately
        Immediate = 0x00,

        /// Reboot after a delay or when the BMC resets.
        Delayed = 0x01,
    }
}

/// A parsed reboot request.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct RebootRequest {
    /// When to reboot.
    pub time: RebootTime,
}

/// The length of a reboot request on the wire, in bytes.
pub const REBOOT_REQUEST_LEN: usize = 1;

impl Message<'_> for RebootRequest {
    const TYPE: ContentType = ContentType::RebootRequest;
}

impl<'a> FromWire<'a> for RebootRequest {
    fn from_wire<R: Read<'a>>(mut r: R) -> Result<Self, FromWireError> {
        let time_u8 = r.read_be::<u8>()?;
        let time = RebootTime::from_wire_value(time_u8).ok_or(FromWireError::OutOfRange)?;
        Ok(Self {
            time,
        })
    }
}

impl ToWire for RebootRequest {
    fn to_wire<W: Write>(&self, mut w: W) -> Result<(), ToWireError> {
        w.write_be(self.time.to_wire_value())?;
        Ok(())
    }
}

// ----------------------------------------------------------------------------

wire_enum! {
    /// The result of a reboot request.
    pub enum RebootResult: u8 {
        /// Success
        Success = 0x00,

        /// Unspecified error
        Error = 0x01,
    }
}

/// A parsed reboot response.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct RebootResponse {
    /// When to reboot from the request.
    pub time: RebootTime,

    /// The result of the reboot request.
    pub result: RebootResult,
}

/// The length of a reboot response on the wire, in bytes.
pub const REBOOT_RESPONSE_LEN: usize = 2;

impl Message<'_> for RebootResponse {
    const TYPE: ContentType = ContentType::RebootResponse;
}

impl<'a> FromWire<'a> for RebootResponse {
    fn from_wire<R: Read<'a>>(mut r: R) -> Result<Self, FromWireError> {
        let time_u8 = r.read_be::<u8>()?;
        let time = RebootTime::from_wire_value(time_u8).ok_or(FromWireError::OutOfRange)?;
        let result_u8 = r.read_be::<u8>()?;
        let result = RebootResult::from_wire_value(result_u8).ok_or(FromWireError::OutOfRange)?;
        Ok(Self {
            time,
            result,
        })
    }
}

impl ToWire for RebootResponse {
    fn to_wire<W: Write>(&self, mut w: W) -> Result<(), ToWireError> {
        w.write_be(self.time.to_wire_value())?;
        w.write_be(self.result.to_wire_value())?;
        Ok(())
    }
}
