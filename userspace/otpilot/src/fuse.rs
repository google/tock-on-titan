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

use core::mem;
use libtock::result::TockResult;
use libtock::syscalls;

pub trait Fuse {
    /// Get Dev ID.
    fn get_dev_id(&self) -> TockResult<u64>;
}

// Get the static Fuse object.
pub fn get() -> &'static dyn Fuse {
    get_impl()
}

const DRIVER_NUMBER: usize = 0x40050;

mod command_nr {
    pub const CHECK_IF_PRESENT: usize = 0;
    pub const GET_DEV_ID: usize = 1;
}

mod allow_nr {
    pub const DEV_ID_BUFFER: usize = 0;
}

struct FuseImpl {}

static mut FUSE: FuseImpl = FuseImpl {};

static mut IS_INITIALIZED: bool = false;

fn get_impl() -> &'static FuseImpl {
    unsafe {
        if !IS_INITIALIZED {
            if FUSE.initialize().is_err() {
                panic!("Could not initialize Fuse");
            }
            IS_INITIALIZED = true;
        }
        &FUSE
    }
}

impl FuseImpl {
    fn initialize(&'static mut self) -> TockResult<()> {
        syscalls::command(DRIVER_NUMBER, command_nr::CHECK_IF_PRESENT, 0, 0)?;

        Ok(())
    }
}

impl Fuse for FuseImpl {
    fn get_dev_id(&self) -> TockResult<u64> {
        let mut dev_id_buffer = [0u8; mem::size_of::<u64>()];

        {
            // We want this to go out of scope after executing the command
            let _dev_id_buffer_share = syscalls::allow(DRIVER_NUMBER, allow_nr::DEV_ID_BUFFER, &mut dev_id_buffer)?;

            syscalls::command(DRIVER_NUMBER, command_nr::GET_DEV_ID, 0, 0)?;
        }

        Ok(u64::from_be_bytes(dev_id_buffer))
    }
}
