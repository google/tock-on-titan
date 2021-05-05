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

use libtock::result::TockResult;
use libtock::syscalls;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct ResetSource {
    /// Power on reset
    power_on_reset: bool,

    /// Low power exit
    low_power_reset: bool,

    /// Watchdog reset
    watchdog_reset: bool,

    /// Lockup reset
    lockup_reset: bool,

    /// SYSRESET
    sysreset: bool,

    /// Software initiated reset through PMU_GLOBAL_RESET
    software_reset: bool,

    /// Fast burnout circuit
    fast_burnour_circuit: bool,

    /// Security breach reset
    security_breach_reset: bool,
}

pub trait Reset {
    /// Execute immediate chip reset.
    fn reset(&self) -> TockResult<()>;

    /// Get reset source.
    fn get_reset_source(&self) -> TockResult<ResetSource>;
}

// Get the static Reset object.
pub fn get() -> &'static dyn Reset {
    get_impl()
}

const DRIVER_NUMBER: usize = 0x40070;

mod command_nr {
    pub const CHECK_IF_PRESENT: usize = 0;
    pub const RESET: usize = 1;
    pub const GET_RESET_SOURCE: usize = 2;
}

struct ResetImpl {}

static mut RESET: ResetImpl = ResetImpl {};

static mut IS_INITIALIZED: bool = false;

fn get_impl() -> &'static ResetImpl {
    unsafe {
        if !IS_INITIALIZED {
            if RESET.initialize().is_err() {
                panic!("Could not initialize Reset");
            }
            IS_INITIALIZED = true;
        }
        &RESET
    }
}

impl ResetImpl {
    fn initialize(&'static mut self) -> TockResult<()> {
        syscalls::command(DRIVER_NUMBER, command_nr::CHECK_IF_PRESENT, 0, 0)?;

        Ok(())
    }
}

impl Reset for ResetImpl {
    fn reset(&self) -> TockResult<()> {
        syscalls::command(DRIVER_NUMBER, command_nr::RESET, 0, 0)?;

        panic!("The Reset driver call should not have returned.")
    }

    fn get_reset_source(&self) -> TockResult<ResetSource> {
        let reset_bits = syscalls::command(DRIVER_NUMBER, command_nr::GET_RESET_SOURCE, 0, 0)?;
        Ok(ResetSource {
            power_on_reset: (reset_bits & 0x1) != 0,
            low_power_reset: (reset_bits & 0x2) != 0,
            watchdog_reset: (reset_bits & 0x4) != 0,
            lockup_reset: (reset_bits & 0x8) != 0,
            sysreset: (reset_bits & 0x10) != 0,
            software_reset: (reset_bits & 0x20) != 0,
            fast_burnour_circuit: (reset_bits & 0x40) != 0,
            security_breach_reset: (reset_bits & 0x80) != 0,
        })
    }

}
