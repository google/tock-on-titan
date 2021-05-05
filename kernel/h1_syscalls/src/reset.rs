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

use h1::hil::reset::Reset;
use kernel::{AppId, AppSlice, Callback, Driver, ReturnCode, Shared};

pub const DRIVER_NUM: usize = 0x40070;

pub struct ResetSyscall<'a> {
    reset: &'a dyn Reset,
}

impl<'a> ResetSyscall<'a> {
    pub fn new(reset: &'a dyn Reset) -> ResetSyscall<'a> {
        ResetSyscall {
            reset: reset,
        }
    }

    fn reset(&self) -> ReturnCode {
        self.reset.reset();

        // This should never return. If it did, something went wrong.
    }

    fn get_reset_source(&self) -> ReturnCode {
        ReturnCode::SuccessWithValue { value: self.reset.get_reset_source() as usize }
    }
}

impl<'a> Driver for ResetSyscall<'a> {
    fn subscribe(&self,
                 _subscribe_num: usize,
                 _callback: Option<Callback>,
                 _app_id: AppId,
    ) -> ReturnCode {
        ReturnCode::ENOSUPPORT
    }

    fn command(&self, command_num: usize, _arg1: usize, _arg2: usize, _caller_id: AppId)
        -> ReturnCode {
        match command_num {
            0 /* Check if present */ => ReturnCode::SUCCESS,
            1 /* Reset chip. */ => self.reset(),
            2 /* Get reset source */ => self.get_reset_source(),
            _ => ReturnCode::ENOSUPPORT
        }
    }

    fn allow(&self,
             _app_id: AppId,
             _minor_num: usize,
             _slice: Option<AppSlice<Shared, u8>>
    ) -> ReturnCode {
        ReturnCode::ENOSUPPORT
    }
}
