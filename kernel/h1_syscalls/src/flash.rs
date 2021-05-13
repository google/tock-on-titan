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
use core::cmp::min;

use h1::hil::flash::Client;
use h1::hil::flash::Flash;

use kernel::AppId;
use kernel::AppSlice;
use kernel::Callback;
use kernel::Driver;
use kernel::Grant;
use kernel::ReturnCode;
use kernel::Shared;

pub const DRIVER_NUM: usize = 0x40040;

const BYTES_PER_WORD: usize = core::mem::size_of::<u32>();

#[derive(Default)]
pub struct AppData {
    write_buffer: Option<AppSlice<Shared, u8>>,
    read_buffer: Option<AppSlice<Shared, u8>>,
    operation_done_callback: Option<Callback>,
}

pub struct FlashSyscalls<'a> {
    device: &'a dyn Flash<'a>,
    write_buffer: core::cell::Cell<Option<&'a mut [u32]>>,
    apps: Grant<AppData>,
    current_user: Cell<Option<AppId>>,
}

impl<'a> FlashSyscalls<'a> {
    pub fn new(device: &'a dyn Flash<'a>,
               write_buffer: &'a mut [u32],
               container: Grant<AppData>) -> FlashSyscalls<'a> {
        FlashSyscalls {
            device: device,
            write_buffer: core::cell::Cell::new(Some(write_buffer)),
            apps: container,
            current_user: Cell::new(None),
        }
    }

    fn erase(&self, caller_id: AppId, page: usize) -> ReturnCode {
        self.apps.enter(caller_id, |_app_data, _| {
            let return_code = self.device.erase(page);
            return_code
        }).unwrap_or(ReturnCode::ENOMEM)
    }

    fn read(&self, caller_id: AppId, offset: usize, read_len: usize) -> ReturnCode {
        // We can only start at words boundaries.
        if offset % BYTES_PER_WORD != 0 {
            return ReturnCode::EINVAL;
        }

        self.apps.enter(caller_id, |app_data, _| {
            if let Some(ref mut read_buffer) = app_data.read_buffer {
                let length = min(read_buffer.len(), read_len);
                for idx in (0..length).step_by(BYTES_PER_WORD) {
                    match self.device.read((offset + idx) / BYTES_PER_WORD) {
                        ReturnCode::SuccessWithValue { value: read_val } => {
                            let val = read_val as u32;
                            for (byte_idx, &byte) in val.to_le_bytes().iter().enumerate() {
                                if idx + byte_idx < length {
                                    read_buffer.as_mut()[idx + byte_idx] = byte;
                                }
                            }
                        }
                        ReturnCode::SUCCESS => {
                            // A read should result in a SuccessWithValue or a failure.
                            // If we get plain SUCCESS, something is seriously wrong.
                            // So let the caller know
                            return ReturnCode::FAIL
                        }
                        failure => {
                            // Everything else must be some kind of failure
                            return failure
                        }
                    }
                }
                return ReturnCode::SUCCESS
            }

            ReturnCode::ENOMEM
        }).unwrap_or(ReturnCode::ENOMEM)
    }

    fn write(&self, caller_id: AppId, target: usize, write_len: usize) -> ReturnCode {
        // We cannot write partial words.
        if target % BYTES_PER_WORD != 0 || write_len % BYTES_PER_WORD != 0 {
            return ReturnCode::EINVAL;
        }

        self.apps.enter(caller_id, |app_data, _| {
            if let Some(ref app_write_buffer) = app_data.write_buffer {
                if let Some(buffer) = self.write_buffer.take() {
                    // Figure minimum of static write_buffer, app's write_buffer and write_length
                    let words = min(buffer.len(), min(app_write_buffer.len(), write_len) / BYTES_PER_WORD);

                    // Then copy apps's write_buffer into static write_buffer
                    for word in 0..words {
                        let app_buf = app_write_buffer.as_ref();
                        let offset = word * BYTES_PER_WORD;
                        buffer[word] = u32::from_le_bytes([app_buf[offset],
                            app_buf[offset + 1],
                            app_buf[offset + 2],
                            app_buf[offset + 3]]);
                    }

                    let (return_code, buffer) = self.device.write(target / BYTES_PER_WORD, &mut buffer[..words]);
                    self.write_buffer.set(buffer);
                    return return_code
                }
            }

            ReturnCode::ENOMEM
        }).unwrap_or(ReturnCode::ENOMEM)
    }
}

impl<'a> Client<'a> for FlashSyscalls<'a> {
    fn erase_done(&self, return_code: ReturnCode) {
        self.current_user.get().map(|current_user| {
            let _ = self.apps.enter(current_user, move |app_data, _| {
                app_data.operation_done_callback.map(
                    |mut cb| cb.schedule(usize::from(return_code), 0, 0));
            });
        });
    }

    fn write_done(&self, write_buffer: &'a mut [u32], return_code: ReturnCode) {
        self.write_buffer.set(Some(write_buffer));
        self.current_user.get().map(|current_user| {
            let _ = self.apps.enter(current_user, move |app_data, _| {
                app_data.operation_done_callback.map(
                    |mut cb| cb.schedule(usize::from(return_code), 0, 0));
            });
        });
    }
}

impl<'a> Driver for FlashSyscalls<'a> {
    fn subscribe(&self,
                 subscribe_num: usize,
                 callback: Option<Callback>,
                 app_id: AppId,
    ) -> ReturnCode {
        match subscribe_num {
            0 /* Operation done
                 Callback arguments:
                 arg1: kernel::ReturnCode */ => {
                self.apps.enter(app_id, |app_data, _| {
                    app_data.operation_done_callback = callback;
                    ReturnCode::SUCCESS
                }).unwrap_or(ReturnCode::ENOMEM)
            },
            _ => ReturnCode::ENOSUPPORT
        }
    }

    fn command(&self, command_num: usize, arg1: usize, arg2: usize, caller_id: AppId) -> ReturnCode {
        if self.current_user.get() == None {
            self.current_user.set(Some(caller_id));
        }
        match command_num {
            0 /* Check if present */ => ReturnCode::SUCCESS,
            1 /* Erase page
                 arg1: page # to erase */ => {
                self.erase(caller_id, arg1)
            },
            2 /* Write data
                 arg1: target offset in flash
                 arg2: number of bytes to write */ => {
                self.write(caller_id, arg1, arg2)
            },
            3 /* Read data
                 arg1: offset in flash
                 arg2: number of bytes to read */ => {
                self.read(caller_id, arg1, arg2)
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
                    // Write Buffer
                    self.apps
                        .enter(app_id, |app_data, _| {
                            app_data.write_buffer = slice;
                            ReturnCode::SUCCESS
                        })
                        .unwrap_or(ReturnCode::FAIL)
                }
                1 => {
                    // Read Buffer
                    self.apps
                        .enter(app_id, |app_data, _| {
                            app_data.read_buffer = slice;
                            ReturnCode::SUCCESS
                        })
                        .unwrap_or(ReturnCode::FAIL)
                }
            _ => ReturnCode::ENOSUPPORT,
        }
    }
}
