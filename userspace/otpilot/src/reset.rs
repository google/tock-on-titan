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

use spiutils::driver::reset::ResetSource;
use spiutils::driver::reset::RESET_SOURCE_LEN;
use spiutils::protocol::wire::FromWire;

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

mod allow_nr {
    pub const BUFFER: usize = 0;
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
        let mut buffer = [0u8; RESET_SOURCE_LEN];

        {
            // We want this to go out of scope after executing the command
            let _buffer_share = syscalls::allow(DRIVER_NUMBER, allow_nr::BUFFER, &mut buffer)?;

            syscalls::command(DRIVER_NUMBER, command_nr::GET_RESET_SOURCE, 0, 0)?;
        }

        let maybe_reset_source = ResetSource::from_wire(buffer.as_ref());
        if maybe_reset_source.is_err() {
            return Err(TockError::Format);
        }

        Ok(maybe_reset_source.unwrap())
    }

}
