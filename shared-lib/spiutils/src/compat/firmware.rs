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

//! Data structures related to firmware.

use crate::io::Read;
use crate::io::Write;
use crate::protocol::wire::FromWireError;
use crate::protocol::wire::FromWire;
use crate::protocol::wire::ToWireError;
use crate::protocol::wire::ToWire;

use core::mem;

// ----------------------------------------------------------------------------

/// The offset of the BuildInfo from the start of the firmware segment.
/// This offset must match the original `SignedHeader` C-struct used in
/// actual firmware images.
pub const BUILD_INFO_OFFSET: usize = 860;

/// The length of a BuildInfo on the wire, in bytes.
pub const BUILD_INFO_LEN: usize = 3 * mem::size_of::<u32>() + mem::size_of::<u64>();

/// Firmware build information.
/// The fields and serialization of this struct must match the original
/// `SignedHeader` C-struct used in actual firmware images.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct BuildInfo {
    /// Time epoch
    pub epoch: u32,

    /// Major version
    pub major: u32,

    /// Minor version
    pub minor: u32,

    /// Timestamp
    pub timestamp: u64,
}

impl<'a> FromWire<'a> for BuildInfo {
    fn from_wire<R: Read<'a>>(mut r: R) -> Result<Self, FromWireError> {
        let epoch = r.read_le::<u32>()?;
        let major = r.read_le::<u32>()?;
        let minor = r.read_le::<u32>()?;
        let timestamp = r.read_le::<u64>()?;
        Ok(Self {
            epoch,
            major,
            minor,
            timestamp,
        })
    }
}

impl ToWire for BuildInfo {
    fn to_wire<W: Write>(&self, mut w: W) -> Result<(), ToWireError> {
        w.write_le(self.epoch)?;
        w.write_le(self.major)?;
        w.write_le(self.minor)?;
        w.write_le(self.timestamp)?;
        Ok(())
    }
}
