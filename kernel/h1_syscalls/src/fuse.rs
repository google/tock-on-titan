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
use h1::hil::fuse::Fuse;
use kernel::{AppId, Callback, Driver, Grant, ReturnCode, Shared, AppSlice};

pub const DRIVER_NUM: usize = 0x40050;

#[derive(Default)]
pub struct AppData {
    dev_id_buffer: Option<AppSlice<Shared, u8>>,
}

pub struct FuseSyscall<'a> {
    fuse: &'a dyn Fuse,
    apps: Grant<AppData>,
    current_user: Cell<Option<AppId>>,
}

impl<'a> FuseSyscall<'a> {
    pub fn new(fuse: &'a dyn Fuse,
               container: Grant<AppData>) -> FuseSyscall<'a> {
        FuseSyscall {
            fuse: fuse,
            apps: container,
            current_user: Cell::new(None),
        }
    }

    fn get_dev_id(&self, caller_id: AppId) -> ReturnCode {
        self.apps.enter(caller_id, |app_data, _| {
            if let Some(ref mut dev_id_buffer) = app_data.dev_id_buffer {
                let dev_id = self.fuse.get_dev_id();
                for (idx, &byte) in dev_id.to_be_bytes().iter().enumerate() {
                    match dev_id_buffer.as_mut().get_mut(idx) {
                        None => return ReturnCode::ENOMEM,
                        Some(value) => *value = byte,
                    }
                }
            }
            ReturnCode::SUCCESS
        }).unwrap_or(ReturnCode::ENOMEM)
    }
}

impl<'a> Driver for FuseSyscall<'a> {
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
            1 /* Get Dev ID and write to Dev ID buffer in BE notation. */ => {
                self.get_dev_id(caller_id)
            },
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
                // Buffer for Dev ID (64 bit in BE notation)
                self.apps
                    .enter(app_id, |app_data, _| {
                        if let Some(s) = slice {
                            app_data.dev_id_buffer = Some(s);
                        } else {
                            app_data.dev_id_buffer = slice;
                        }
                        ReturnCode::SUCCESS
                    })
                    .unwrap_or(ReturnCode::FAIL)
            }
            _ => ReturnCode::ENOSUPPORT,
        }
    }
}
