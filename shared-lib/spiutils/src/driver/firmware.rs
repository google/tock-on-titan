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

//! Firmware kernel interface.

use crate::io::Read;
use crate::io::Write;
use crate::protocol::firmware::SegmentAndLocation;
use crate::protocol::wire::FromWireError;
use crate::protocol::wire::FromWire;
use crate::protocol::wire::ToWireError;
use crate::protocol::wire::ToWire;
use crate::protocol::wire::WireEnum;

use core::mem;

/// The length of a SegmentInfo on the wire, in bytes.
pub const SEGMENT_INFO_LEN: usize = mem::size_of::<u8>() + 4 * mem::size_of::<u32>();

/// The "unknown" segment.
pub const UNKNOWN_SEGMENT: SegmentInfo = SegmentInfo {
    identifier: SegmentAndLocation::Unknown,
    address: 0,
    size: 0,
    start_page: 0,
    page_count: 0,
};

/// Information about a segment.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct SegmentInfo {
    /// The segment and location this info struct pertains to.
    pub identifier: SegmentAndLocation,

    /// The start address of the segment.
    pub address: u32,

    /// The size of the segment.
    pub size: u32,

    /// The start page of the segment.
    pub start_page: u32,

    /// The number of pages in the segment.
    pub page_count: u32,
}

impl<'a> FromWire<'a> for SegmentInfo {
    fn from_wire<R: Read<'a>>(mut r: R) -> Result<Self, FromWireError> {
        let sal_u8 = r.read_be::<u8>()?;
        let identifier = SegmentAndLocation::from_wire_value(sal_u8).ok_or(FromWireError::OutOfRange)?;
        let address = r.read_be::<u32>()?;
        let size = r.read_be::<u32>()?;
        let start_page = r.read_be::<u32>()?;
        let page_count = r.read_be::<u32>()?;
        Ok(Self {
            identifier,
            address,
            size,
            start_page,
            page_count,
        })
    }
}

impl ToWire for SegmentInfo {
    fn to_wire<W: Write>(&self, mut w: W) -> Result<(), ToWireError> {
        w.write_be(self.identifier.to_wire_value())?;
        w.write_be(self.address)?;
        w.write_be(self.size)?;
        w.write_be(self.start_page)?;
        w.write_be(self.page_count)?;
        Ok(())
    }
}

// ----------------------------------------------------------------------------

/// The length of a RuntimeSegmentInfo on the wire, in bytes.
pub const RUNTIME_SEGMENT_INFO_LEN: usize = 4 * SEGMENT_INFO_LEN;

/// The "unknown" runtime segment.
pub const UNKNOWN_RUNTIME_SEGMENT_INFO: RuntimeSegmentInfo = RuntimeSegmentInfo {
    active_ro: UNKNOWN_SEGMENT,
    active_rw: UNKNOWN_SEGMENT,
    inactive_ro: UNKNOWN_SEGMENT,
    inactive_rw: UNKNOWN_SEGMENT,
};

/// Information about segments at runtime.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct RuntimeSegmentInfo {
    /// The active RO.
    pub active_ro: SegmentInfo,

    /// The active RW.
    pub active_rw: SegmentInfo,

    /// The inactive RO.
    pub inactive_ro: SegmentInfo,

    /// The inactive RW.
    pub inactive_rw: SegmentInfo,
}

impl<'a> FromWire<'a> for RuntimeSegmentInfo {
    fn from_wire<R: Read<'a>>(mut r: R) -> Result<Self, FromWireError> {
        let active_ro = SegmentInfo::from_wire(&mut r)?;
        let active_rw = SegmentInfo::from_wire(&mut r)?;
        let inactive_ro = SegmentInfo::from_wire(&mut r)?;
        let inactive_rw = SegmentInfo::from_wire(&mut r)?;
        Ok(Self {
            active_ro,
            active_rw,
            inactive_ro,
            inactive_rw,
        })
    }
}

impl ToWire for RuntimeSegmentInfo {
    fn to_wire<W: Write>(&self, mut w: W) -> Result<(), ToWireError> {
        self.active_ro.to_wire(&mut w)?;
        self.active_rw.to_wire(&mut w)?;
        self.inactive_ro.to_wire(&mut w)?;
        self.inactive_rw.to_wire(&mut w)?;
        Ok(())
    }
}

// ----------------------------------------------------------------------------
