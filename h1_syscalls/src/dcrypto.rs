// Copyright 2018 Google LLC
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

use core::cell::Cell;
use h1::crypto::dcrypto::{Dcrypto, DcryptoClient, ProgramFault};
use kernel::{AppId, Callback, Driver, ReturnCode, Shared, AppSlice};
use kernel::common::cells::MapCell;

pub const DRIVER_NUM: usize = 0x40004;

pub struct App {
    program: Option<AppSlice<Shared, u8>>,
    data_buffer: Option<AppSlice<Shared, u8>>,
    callback: Option<Callback>,
}

impl Default for App {
    fn default() -> App {
        App {
            program: None,
            data_buffer: None,
            callback: None
        }
    }
}

pub struct DcryptoDriver<'a> {
    device: &'a dyn Dcrypto<'a>,
    app: MapCell<App>,
    busy: Cell<bool>,
}

impl<'a> DcryptoDriver<'a> {
    pub fn new(device: &'a mut dyn Dcrypto<'a>) -> DcryptoDriver<'a> {
        DcryptoDriver {
            device: device,
            app: MapCell::new(App::default()),
            busy: Cell::new(false),
       }
    }

    fn run_program(&self, app: &mut App, instruction: u32) -> ReturnCode {
        if app.data_buffer.is_none() || app.program.is_none() {
            return ReturnCode::ENOMEM;
        }

        let mut rval: ReturnCode;
        let data_slice = app.data_buffer.take().unwrap();
        let program_slice = app.program.take().unwrap();
        {
            // In user space, len is in bytes. For the device, however,
            // len is in terms of words, with partial words being truncated.
            // So divide by 4.
            let data = data_slice.as_ref();
            let data_len = data.len() / 4;
            let program = program_slice.as_ref();
            let program_len = program.len() / 4;

            rval = self.device.write_data(data, 0, data_len as u32);

            if rval == ReturnCode::SUCCESS {
                rval = self.device.write_instructions(program, 0, program_len as u32);
            }
        };
        app.data_buffer = Some(data_slice);
        app.program = Some(program_slice);

        if rval != ReturnCode::SUCCESS {
            return rval;
        }
        rval = self.device.call_imem(instruction);
        if rval != ReturnCode::SUCCESS {
            return rval;
        }
        ReturnCode::SUCCESS
    }
}

impl<'a> Driver for DcryptoDriver<'a> {
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
            },
            _ => ReturnCode::ENOSUPPORT
        }
    }

    fn command(&self, command_num: usize, instruction: usize, _: usize, _: AppId) -> ReturnCode {
        match command_num {
            0 /* Check if present */ => ReturnCode::SUCCESS,
            1 /* run program */ => {
                if self.busy.get() {
                    ReturnCode::EBUSY
                } else {
                    self.app.map_or(ReturnCode::EBUSY, |app| {
                        self.busy.set(true);
                        self.run_program(app, instruction as u32)
                    })
                }
            }
            _ => ReturnCode::ENOSUPPORT,
        }
    }

    fn allow(&self, _: AppId,
             minor_num: usize,
             slice: Option<AppSlice<Shared, u8>>
    ) -> ReturnCode {
        match minor_num {
            0 => {
                // Data memory
                self.app
                    .map(|app_data| {
                        app_data.data_buffer = slice;
                        ReturnCode::SUCCESS
                    })
                    .unwrap_or(ReturnCode::FAIL)
            }
            1 => {
                // Input Buffer
                self.app
                    .map(|app_data| {
                        app_data.program = slice;
                        ReturnCode::SUCCESS
                    })
                    .unwrap_or(ReturnCode::FAIL)
            }
            _ => ReturnCode::ENOSUPPORT,
        }
    }
}

impl<'a> DcryptoClient<'a> for DcryptoDriver<'a> {
    fn execution_complete(&self, error: ReturnCode, fault: ProgramFault) {
        self.busy.set(false);
        self.app.map(move |app| {
            app.callback.map(|mut callback| {
                let mut data_slice = app.data_buffer.take().unwrap();
                {
                    let data = data_slice.as_mut();
                    // In user space, len is in bytes. For the device,
                    // however, len is in terms of words, with partial
                    // words being truncated.  So divide by 4.
                    let len = (data.len() / 4) as u32;
                    self.device.read_data(data, 0, len);
                    callback.schedule(usize::from(error), usize::from(fault), 0);
                }
                app.data_buffer = Some(data_slice);
            });
        });
    }

    fn reset_complete(&self, _error: ReturnCode) {
        panic!("ERROR: Dcrypto driver reset_complete invoked, but should never be called.");
    }

    fn secret_wipe_complete(&self, _error: ReturnCode) {
        panic!("ERROR: Dcrypto driver secret_wipe_complete invoked, but should never be called.");
    }


}
