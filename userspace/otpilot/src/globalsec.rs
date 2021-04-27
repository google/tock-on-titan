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

use libtock::result::TockError;
use libtock::result::TockResult;
use libtock::syscalls;
use spiutils::driver::firmware::RuntimeSegmentInfo;
use spiutils::driver::firmware::SegmentInfo;
use spiutils::driver::firmware::RUNTIME_SEGMENT_INFO_LEN;
use spiutils::driver::firmware::UNKNOWN_RUNTIME_SEGMENT_INFO;
use spiutils::protocol::wire::FromWire;

pub trait GlobalSec {
    /// Get segment information for active RO.
    fn get_active_ro(&self) -> SegmentInfo;

    /// Get segment information for active RW.
    fn get_active_rw(&self) -> SegmentInfo;

    /// Get segment information for inactive RO.
    fn get_inactive_ro(&self) -> SegmentInfo;

    /// Get segment information for inactive RW.
    fn get_inactive_rw(&self) -> SegmentInfo;
}

// Get the static GlobalSec object.
pub fn get() -> &'static dyn GlobalSec {
    get_impl()
}

const DRIVER_NUMBER: usize = 0x40060;

mod command_nr {
    pub const CHECK_IF_PRESENT: usize = 0;
    pub const GET_RUNTIME_SEGMENT_INFO: usize = 1;
}

mod allow_nr {
    pub const BUFFER: usize = 0;
}

struct GlobalSecImpl {
    runtime_segment_info: RuntimeSegmentInfo,
}

static mut GLOBALSEC: GlobalSecImpl = GlobalSecImpl {
    runtime_segment_info: UNKNOWN_RUNTIME_SEGMENT_INFO,
};

static mut IS_INITIALIZED: bool = false;

fn get_impl() -> &'static GlobalSecImpl {
    unsafe {
        if !IS_INITIALIZED {
            if GLOBALSEC.initialize().is_err() {
                panic!("Could not initialize GlobalSec");
            }
            IS_INITIALIZED = true;
        }
        &GLOBALSEC
    }
}

impl GlobalSecImpl {
    fn initialize(&'static mut self) -> TockResult<()> {
        syscalls::command(DRIVER_NUMBER, command_nr::CHECK_IF_PRESENT, 0, 0)?;

        self.runtime_segment_info = self.get_runtime_segment_info()?;

        Ok(())
    }

    fn get_runtime_segment_info(&self) -> TockResult<RuntimeSegmentInfo> {
        let mut buffer = [0u8; RUNTIME_SEGMENT_INFO_LEN];

        {
            // We want this to go out of scope after executing the command
            let _buffer_share = syscalls::allow(DRIVER_NUMBER, allow_nr::BUFFER, &mut buffer)?;

            syscalls::command(DRIVER_NUMBER, command_nr::GET_RUNTIME_SEGMENT_INFO, 0, 0)?;
        }

        let maybe_runtime_segment_info = RuntimeSegmentInfo::from_wire(buffer.as_ref());
        if maybe_runtime_segment_info.is_err() {
            return Err(TockError::Format);
        }

        Ok(maybe_runtime_segment_info.unwrap())
    }
}

impl GlobalSec for GlobalSecImpl {
    fn get_active_ro(&self) -> SegmentInfo {
        self.runtime_segment_info.active_ro
    }

    fn get_active_rw(&self) -> SegmentInfo {
        self.runtime_segment_info.active_rw
    }

    fn get_inactive_ro(&self) -> SegmentInfo {
        self.runtime_segment_info.inactive_ro
    }

    fn get_inactive_rw(&self) -> SegmentInfo {
        self.runtime_segment_info.inactive_rw
    }
}
