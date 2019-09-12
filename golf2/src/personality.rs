// Copyright 2019 Google LLC
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

//! System call driver for device attestation (personality) data. This
//! is per-device data that will be stored durably on the device; current
//! implementations store it in RAM.
//!
//! The driver implements 3 commands:
//!   0. check if the driver is present (ReturnCode::SUCCESS if so)
//!   1. read personality data into a user buffer.
//!   2. durably write personality data from a user buffer, completion signaled
//!      by a callback.
//!
//! The driver implements 1 allow:
//!   0. userspace buffer used for read and write (commands 1 and 2).
//!
//! The driver implements 1 subscribe:
//!   0. callback for when a durable write completes.

use core::cell::Cell;
use h1b::personality;
use h1b::hil::personality::Personality;
use kernel::{AppId, Callback, Driver, ReturnCode, Shared, AppSlice};
use kernel::common::cells::MapCell;

pub const DRIVER_NUM: usize = 0x5000b;


const COMMAND_CHECK: usize             = 0;
const COMMAND_READ: usize              = 1;
const COMMAND_WRITE: usize             = 2;
const ALLOW_BUFFER: usize              = 0;
const SUBSCRIBE_WRITE_DONE: usize      = 0;

#[derive(Default)]
pub struct App {
    data: Option<AppSlice<Shared, u8>>,
    callback: Option<Callback>,
}

pub struct PersonalitySyscall<'a> {
    device: &'a personality::PersonalityDriver<'a>,
    app: MapCell<App>,
    busy: Cell<bool>
}

impl<'a> PersonalitySyscall<'a> {
    pub fn new(device: &'a mut personality::PersonalityDriver<'a>) -> PersonalitySyscall<'a> {
        PersonalitySyscall {
            device: device,
            app: MapCell::new(App::default()),
            busy: Cell::new(false)
        }
    }
}

impl<'a> Driver for PersonalitySyscall<'a> {
    fn subscribe(&self,
                 subscribe_num: usize,
                 callback: Option<Callback>,
                 _app_id: AppId,
    ) -> ReturnCode {
        match subscribe_num {
            SUBSCRIBE_WRITE_DONE => {
                self.app.map(|app| {
                    app.callback = callback;
                });
                ReturnCode::SUCCESS
            }
            _ => ReturnCode::ENOSUPPORT
        }
    }

    fn command(&self, command_num: usize, _: usize, _: usize, _: AppId) -> ReturnCode {
        match command_num {
            COMMAND_CHECK => ReturnCode::SUCCESS,
            COMMAND_READ  => {
                if self.busy.get() {
                    ReturnCode::EBUSY
                } else {
                    self.app.map_or(ReturnCode::EBUSY, |app| {
                        if app.data.is_none() {return ReturnCode::ENOMEM;}

                        let mut data_slice = app.data.take().unwrap();
                        self.device.get_u8(data_slice.as_mut());
                        app.data = Some(data_slice);
                        ReturnCode::SUCCESS
                    })

                }
            },
            COMMAND_WRITE => {
                if self.busy.get() {
                    ReturnCode::EBUSY
                } else {
                    self.app.map_or(ReturnCode::EBUSY, |app| {
                        if app.data.is_none() {return ReturnCode::ENOMEM;}

                        let mut data_slice = app.data.take().unwrap();
                        self.device.set_u8(data_slice.as_mut());
                        app.data = Some(data_slice);
                        ReturnCode::SUCCESS
                    })
                }
            },
            _ => ReturnCode::ENOSUPPORT
        }
    }

    fn allow(&self, _: AppId,
             minor_num: usize,
             slice: Option<AppSlice<Shared, u8>>,
    ) -> ReturnCode {
        match minor_num {
            ALLOW_BUFFER => {
                self.app.map(|app_data| {
                    app_data.data = slice;
                    ReturnCode::SUCCESS
                })
               .unwrap_or(ReturnCode::FAIL)
            },
            _ => ReturnCode::ENOSUPPORT,
        }
    }

}
