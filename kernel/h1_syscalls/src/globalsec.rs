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

use core::cell::Cell;
use h1::hil::globalsec::GlobalSec;
use kernel::{AppId, Callback, Driver, Grant, ReturnCode, Shared, AppSlice};
use spiutils::io::Cursor;
use spiutils::protocol::wire::ToWire;

pub const DRIVER_NUM: usize = 0x40060;

#[derive(Default)]
pub struct AppData {
    buffer: Option<AppSlice<Shared, u8>>,
}

pub struct GlobalSecSyscall<'a> {
    globalsec: &'a dyn GlobalSec,
    apps: Grant<AppData>,
    current_user: Cell<Option<AppId>>,
}

impl<'a> GlobalSecSyscall<'a> {
    pub fn new(globalsec: &'a dyn GlobalSec,
               container: Grant<AppData>) -> GlobalSecSyscall<'a> {
        GlobalSecSyscall {
            globalsec: globalsec,
            apps: container,
            current_user: Cell::new(None),
        }
    }

    fn get_runtime_segment_info(&self, caller_id: AppId) -> ReturnCode {
        self.apps.enter(caller_id, |app_data, _| {
            if let Some(ref mut buffer) = app_data.buffer {
                let cursor = Cursor::new(buffer.as_mut());
                if self.globalsec.get_runtime_segment_info().to_wire(cursor).is_err() {
                    return ReturnCode::ENOMEM;
                }
            }
            ReturnCode::SUCCESS
        }).unwrap_or(ReturnCode::ENOMEM)
    }
}

impl<'a> Driver for GlobalSecSyscall<'a> {
    fn subscribe(&self,
                 subscribe_num: usize,
                 _callback: Option<Callback>,
                 _app_id: AppId,
    ) -> ReturnCode {
        match subscribe_num {
            _ => ReturnCode::ENOSUPPORT
        }
    }

    fn command(&self, command_num: usize, _arg1: usize, _arg2: usize, caller_id: AppId)
        -> ReturnCode {
        if self.current_user.get() == None {
            self.current_user.set(Some(caller_id));
        }
        match command_num {
            0 /* Check if present */ => ReturnCode::SUCCESS,
            1 /* Get runtime segment info */ => self.get_runtime_segment_info(caller_id),
            _ => ReturnCode::ENOSUPPORT
        }
    }

    fn allow(&self,
             app_id: AppId,
             minor_num: usize,
             slice: Option<AppSlice<Shared, u8>>
    ) -> ReturnCode {
        match minor_num {
            0 => {
                // Buffer for data exchange
                self.apps
                    .enter(app_id, |app_data, _| {
                        app_data.buffer = slice;
                        ReturnCode::SUCCESS
                    })
                    .unwrap_or(ReturnCode::FAIL)
            }
            _ => ReturnCode::ENOSUPPORT,
        }
    }
}
