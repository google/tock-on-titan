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
use h1b::hil::personality::{Client, Personality};
use kernel::{AppId, Callback, Driver, Grant, ReturnCode, Shared, AppSlice};
use kernel::common::cells::OptionalCell;

pub const DRIVER_NUM: usize = 0x5000b;


const COMMAND_CHECK: usize             = 0;
const COMMAND_READ: usize              = 1;
const COMMAND_WRITE: usize             = 2;
const ALLOW_BUFFER: usize              = 0;
const SUBSCRIBE_WRITE_DONE: usize      = 0;

#[derive(Default)]
pub struct AppData {
    data: Option<AppSlice<Shared, u8>>,
    callback: Option<Callback>,
}

pub struct PersonalitySyscall<'a> {
    device: &'a personality::PersonalityDriver<'a>,
    apps: Grant<AppData>,
    busy: Cell<bool>,
    current_user: OptionalCell<AppId>
}

impl<'a> PersonalitySyscall<'a> {
    pub fn new(device: &'a mut personality::PersonalityDriver<'a>,
               container: Grant<AppData>) -> PersonalitySyscall<'a> {
        PersonalitySyscall {
            device: device,
            apps: container,
            busy: Cell::new(false),
            current_user: OptionalCell::empty()

        }
    }
}

impl<'a> Driver for PersonalitySyscall<'a> {
    fn subscribe(&self,
                 subscribe_num: usize,
                 callback: Option<Callback>,
                 app_id: AppId,
    ) -> ReturnCode {
        match subscribe_num {
            SUBSCRIBE_WRITE_DONE => {
                self.apps.enter(app_id, |app_data, _| {
                    app_data.callback = callback;
                });
                ReturnCode::SUCCESS
            }
            _ => ReturnCode::ENOSUPPORT
        }
    }

    fn command(&self, command_num: usize, _: usize, _: usize, app_id: AppId) -> ReturnCode {
        match command_num {
            COMMAND_CHECK => ReturnCode::SUCCESS,
            COMMAND_READ  => {
                if self.busy.get() {
                    ReturnCode::EBUSY
                } else {
                    self.apps.enter(app_id, |app_data, _| {
                        if app_data.data.is_none() {return ReturnCode::ENOMEM;}
                        let mut data_slice = app_data.data.take().unwrap();
                        self.device.get_u8(data_slice.as_mut());
                        app_data.data = Some(data_slice);
                        ReturnCode::SUCCESS
                    }).unwrap_or(ReturnCode::ENOMEM)

                }
            },
            COMMAND_WRITE => {
                if self.busy.get() {
                    ReturnCode::EBUSY
                } else {
                    self.apps.enter(app_id, |app_data, _| {
                        if app_data.data.is_none() {return ReturnCode::ENOMEM;}

                        let mut data_slice = app_data.data.take().unwrap();
                        self.device.set_u8(data_slice.as_mut());
                        self.current_user.replace(app_id);
                        app_data.data = Some(data_slice);
                        ReturnCode::SUCCESS
                    }).unwrap_or(ReturnCode::ENOMEM)
                }
            },
            _ => ReturnCode::ENOSUPPORT
        }
    }

    fn allow(&self,
             app_id: AppId,
             minor_num: usize,
             slice: Option<AppSlice<Shared, u8>>,
    ) -> ReturnCode {
        match minor_num {
            ALLOW_BUFFER => {
                self.apps.enter(app_id, |app_data, _| {
                    app_data.data = slice;
                    ReturnCode::SUCCESS
                })
               .unwrap_or(ReturnCode::ENOMEM)
            },
            _ => ReturnCode::ENOSUPPORT,
        }
    }

}

impl<'a> Client<'a> for PersonalitySyscall<'a> {

    fn set_done(&self, rval: ReturnCode) {
        self.current_user.map(|current_user| {
            self.apps.enter(*current_user, |app_data, _| {
                self.current_user.clear();
                app_data.callback.map(|mut cb| cb.schedule(From::from(rval), 0, 0));
            });
        });
    }

    fn set_u8_done(&self, rval: ReturnCode) {
        debug!("PersonalitySyscall::set_u8_done called");
        self.current_user.map(|current_user| {
            self.apps.enter(*current_user, |app_data, _| {
                self.current_user.clear();
                debug!("Calling set_u8_done callback on app");
                app_data.callback.map(|mut cb| cb.schedule(From::from(rval), 0, 0));
            });
        });
    }
}
