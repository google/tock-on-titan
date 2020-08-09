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

use libtock::result::TockResult;
use libtock::syscalls;

pub trait SpiHostH1 {
    /// Enable/disable SPI passthrough.
    fn set_passthrough(&self, enabled: bool) -> TockResult<()>;

    /// Enable/disable wait for BUSY bit to clear before completing transactions.
    fn set_wait_busy_clear_in_transactions(&self, enabled: bool) -> TockResult<()>;
}

// Get the static SpiHostH1 object.
pub fn get() -> &'static dyn SpiHostH1 {
    get_impl()
}

const DRIVER_NUMBER: usize = 0x40020;

mod command_nr {
    pub const CHECK_IF_PRESENT: usize = 0;
    pub const ENABLE_DISABLE_PASSTHROUGH: usize = 1;
    pub const ENABLE_DISABLE_WAIT_BUSY_CLEAR_IN_TRANSACTIONS: usize = 2;
}

struct SpiHostH1Impl {}

static mut SPI_HOST_H1: SpiHostH1Impl = SpiHostH1Impl {};

static mut IS_INITIALIZED: bool = false;

fn get_impl() -> &'static SpiHostH1Impl {
    unsafe {
        if !IS_INITIALIZED {
            if SPI_HOST_H1.initialize().is_err() {
                panic!("Could not initialize SPI Host H1");
            }
            IS_INITIALIZED = true;
        }
        &SPI_HOST_H1
    }
}

impl SpiHostH1Impl {
    fn initialize(&'static mut self) -> TockResult<()> {
        syscalls::command(DRIVER_NUMBER, command_nr::CHECK_IF_PRESENT, 0, 0)?;

        Ok(())
    }
}

impl SpiHostH1 for SpiHostH1Impl {
    fn set_passthrough(&self, enabled: bool) -> TockResult<()> {
        syscalls::command(DRIVER_NUMBER, command_nr::ENABLE_DISABLE_PASSTHROUGH, if enabled { 1 } else { 0 }, 0)?;

        Ok(())
    }

    fn set_wait_busy_clear_in_transactions(&self, enabled: bool) -> TockResult<()> {
        syscalls::command(DRIVER_NUMBER, command_nr::ENABLE_DISABLE_WAIT_BUSY_CLEAR_IN_TRANSACTIONS, if enabled { 1 } else { 0 }, 0)?;

        Ok(())
    }
}
