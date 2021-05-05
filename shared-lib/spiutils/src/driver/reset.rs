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

//! Data related to resets.

use crate::io::Read;
use crate::io::Write;
use crate::protocol::wire::FromWireError;
use crate::protocol::wire::FromWire;
use crate::protocol::wire::ToWireError;
use crate::protocol::wire::ToWire;

/// The source of the last reset.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct ResetSource {
    /// Power on reset
    pub power_on_reset: bool,

    /// Low power exit
    pub low_power_reset: bool,

    /// Watchdog reset
    pub watchdog_reset: bool,

    /// Lockup reset
    pub lockup_reset: bool,

    /// SYSRESET
    pub sysreset: bool,

    /// Software initiated reset through PMU_GLOBAL_RESET
    pub software_reset: bool,

    /// Fast burnout circuit
    pub fast_burnout_circuit: bool,

    /// Security breach reset
    pub security_breach_reset: bool,
}

/// The length of a ResetSource on the wire, in bytes.
pub const RESET_SOURCE_LEN: usize = 8;

impl<'a> FromWire<'a> for ResetSource {
    fn from_wire<R: Read<'a>>(mut r: R) -> Result<Self, FromWireError> {
        let power_on_reset = r.read_be::<u8>()? != 0;
        let low_power_reset = r.read_be::<u8>()? != 0;
        let watchdog_reset = r.read_be::<u8>()? != 0;
        let lockup_reset = r.read_be::<u8>()? != 0;
        let sysreset = r.read_be::<u8>()? != 0;
        let software_reset = r.read_be::<u8>()? != 0;
        let fast_burnout_circuit = r.read_be::<u8>()? != 0;
        let security_breach_reset = r.read_be::<u8>()? != 0;
        Ok(Self {
            power_on_reset,
            low_power_reset,
            watchdog_reset,
            lockup_reset,
            sysreset,
            software_reset,
            fast_burnout_circuit,
            security_breach_reset,
        })
    }
}

impl ToWire for ResetSource {
    fn to_wire<W: Write>(&self, mut w: W) -> Result<(), ToWireError> {
        w.write_be(if self.power_on_reset { 1 } else { 0u8 })?;
        w.write_be(if self.low_power_reset { 1 } else { 0u8 })?;
        w.write_be(if self.watchdog_reset { 1 } else { 0u8 })?;
        w.write_be(if self.lockup_reset { 1 } else { 0u8 })?;
        w.write_be(if self.sysreset { 1 } else { 0u8 })?;
        w.write_be(if self.software_reset { 1 } else { 0u8 })?;
        w.write_be(if self.fast_burnout_circuit { 1 } else { 0u8 })?;
        w.write_be(if self.security_breach_reset { 1 } else { 0u8 })?;
        Ok(())
    }
}
