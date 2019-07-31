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

#![allow(dead_code)]

//! System call driver for device attestation (personality) data.

use core::cell::Cell;
use h1b::personality;
use h1b::hil::personality::Personality;
use kernel::{AppId, Callback, Driver, ReturnCode, Shared, AppSlice};
use kernel::common::cells::MapCell;

pub const DRIVER_NUM: usize = 0x5000b;

pub struct App {
    data: Option<AppSlice<Shared, u8>>,
    callback: Option<Callback>,
}

impl Default for App {
    fn default() -> App {
        App {
            data: None,
            callback: None,
        }
    }
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
            0 => {
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
            0 /* Check if present */ => ReturnCode::SUCCESS,
            1 /* Read personality */ => {
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
            2 /* Write personality */ => {
                if self.busy.get() {
                    ReturnCode::EBUSY
                } else {
                    self.app.map_or(ReturnCode::EBUSY, |app| {
                        if app.data.is_none() {return ReturnCode::ENOMEM;}

                        let data_slice = app.data.take().unwrap();
                        self.device.set_u8(data_slice.as_ref());
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
            0 => {
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
