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
use h1b::crypto::aes::{AesEngine, AES128Ecb};
use kernel::{AppId, Callback, Driver, Grant, ReturnCode, Shared, AppSlice};
use kernel::common::cells::TakeCell;
use kernel::hil::symmetric_encryption;
use kernel::hil::symmetric_encryption::{AES128_BLOCK_SIZE, AES128_KEY_SIZE};

use kernel::hil::symmetric_encryption::{AES128, AES128CBC, AES128Ctr};

pub const DRIVER_NUM: usize = 0x40010;

pub static mut AES_BUF: [u8; AES128_BLOCK_SIZE] = [0; AES128_BLOCK_SIZE];

#[derive(Default)]
pub struct AppData {
    key: Option<AppSlice<Shared, u8>>,
    input_buffer: Option<AppSlice<Shared, u8>>,
    output_buffer: Option<AppSlice<Shared, u8>>,
    iv_buffer: Option<AppSlice<Shared, u8>>,
    crypto_callback: Option<Callback>,
}

pub struct AesDriver<'a> {
    device: &'a AesEngine<'a>,
    apps: Grant<AppData>,
    current_user: Cell<Option<AppId>>,
    buffer: TakeCell<'a, [u8]>,
}

impl<'a> AesDriver<'a> {
    pub fn new(device: &'a mut AesEngine<'a>,
               container: Grant<AppData>) -> AesDriver<'a> {
        AesDriver {
            device: device,
            apps: container,
            current_user: Cell::new(None),
            buffer: TakeCell::empty(),
        }
    }

    // Register a buffer, which must be of size AES128_BLOCK_SIZE; if
    // it is not the proper size, return the buffer in the
    // Option. Return None if the buffer was correct.
    pub fn initialize(&self,
                      input_buffer: &'a mut [u8]) -> Option<&'a mut [u8]>  {
        self.device.setup();

        if input_buffer.len() != AES128_BLOCK_SIZE {
            Some(input_buffer)
        } else {
            self.buffer.replace(input_buffer);
            None
        }
    }

    fn run_aes(&self, caller_id: AppId) -> ReturnCode {
        self.apps.enter(caller_id, |app_data, _| {
            if app_data.input_buffer.is_none() {
                debug!("AES: Missing input buffer.\n");
                return ReturnCode::ENOMEM;
            } else if app_data.key.is_none() {
                debug!("AES: Missing application encryption key.\n");
                return ReturnCode::ENOMEM;
            } else if self.buffer.is_none() {
                debug!("AES: Missing kernel buffer.\n");
                return ReturnCode::ENOMEM;
            }

            let key = app_data.key.take();
            let rcode = key.map_or(ReturnCode::EINVAL, |key| {
                if key.len() == AES128_KEY_SIZE {
                    self.device.set_key(key.as_ref());
                    app_data.key = Some(key);
                    ReturnCode::SUCCESS
                } else {
                    debug!("AES: application encryption key is wrong size.\n");
                    ReturnCode::EINVAL
                }
            });

            if rcode != ReturnCode::SUCCESS {
                return rcode;
            }

            // Copy application data into the kernel buffer
            self.buffer.map(|buf| {
                app_data.input_buffer.as_ref().map(|src| {
                    for (i, c) in src.as_ref()[0..AES128_BLOCK_SIZE].iter().enumerate() {
                        buf[i] = *c;
                    }
                });
            });
            let buf = self.buffer.take().unwrap();
            let opt =  AES128::crypt(self.device, None, buf, 0, AES128_BLOCK_SIZE);
            if let Some((rcode, _ibufopt, obuf)) = opt {
                debug!("Failed to invoke AES encryption: {:?}", rcode);
                self.buffer.put(Some(obuf));
                rcode
            } else {
                ReturnCode::SUCCESS
            }
        }).unwrap_or(ReturnCode::ENOMEM)
    }
}

impl<'a> symmetric_encryption::Client<'a> for AesDriver<'a> {
    fn crypt_done(&self, _source: Option<&'a mut [u8]>, output: &'a mut [u8]) {
        self.current_user.get().map(|current_user| {
            let _ = self.apps.enter(current_user, move |app_data, _| {
                if let Some(ref mut slice) = app_data.output_buffer {
                    self.device.read_data(slice.as_mut());
                }
                let val = {
                    if let Some(ref mut slice) = app_data.input_buffer {
                        self.device.read_data(slice.as_mut())
                    } else {
                        0
                    }
                };
                self.current_user.set(None);
                app_data.crypto_callback.map(|mut cb| cb.schedule(val, 0, 0));
            });
        });
        self.buffer.replace(output);
    }
}



impl<'a> Driver for AesDriver<'a> {
    fn subscribe(&self,
                 subscribe_num: usize,
                 callback: Option<Callback>,
                 app_id: AppId,
    ) -> ReturnCode {
        match subscribe_num {
            0 => { // Encrypt/decrypt done
                self.apps.enter(app_id, |app_data, _| {
                    app_data.crypto_callback = callback;
                    ReturnCode::SUCCESS
                }).unwrap_or(ReturnCode::ENOMEM)
            },
            _ => ReturnCode::ENOSUPPORT
        }
    }

    fn command(&self, command_num: usize, _arg1: usize, _: usize, caller_id: AppId) -> ReturnCode {
        if self.current_user.get() == None {
            self.current_user.set(Some(caller_id));
        }
        match command_num {
            0 /* Check if present */ => ReturnCode::SUCCESS,
            1 /* encrypt ECB */ => {
                self.device.set_mode_aes128ecb(true);
                self.run_aes(caller_id)
            },
            2 /* decrypt ECB */ => {
                self.device.set_mode_aes128ecb(false);
                self.run_aes(caller_id)
            }
            3 | 4 /* encrypt/decrypt CTR */ => {
                self.apps.enter(caller_id, |app_data, _| {
                    self.device.set_mode_aes128ctr(true);
                    let buffer = app_data.iv_buffer.take();
                    buffer.map_or(ReturnCode::ENOMEM, |iv| {
                        self.device.set_iv(iv.as_ref());
                        app_data.iv_buffer = Some(iv);
                        self.run_aes(caller_id)
                    })
                }).unwrap_or(ReturnCode::ENOMEM)
            }
            5 /* encrypt CBC */ => {
                self.device.set_mode_aes128cbc(true);
                self.run_aes(caller_id)
            },
            6 /* decrypt CBC */ => {
                self.device.set_mode_aes128cbc(false);
                self.run_aes(caller_id)
            },
            7 /* install key */ => {
                self.apps.enter(caller_id, |app_data, _| {
                    let key = app_data.key.take();
                    let rcode = key.map_or(ReturnCode::ENOMEM, |key| {
                        if key.len() == AES128_KEY_SIZE {
                            self.device.set_key(key.as_ref());
                        }
                        app_data.key = Some(key);
                        ReturnCode::SUCCESS
                    });
                    rcode
                }).unwrap_or(ReturnCode::ENOMEM)
            }
            _ => {
                self.current_user.set(None);
                ReturnCode::ENOSUPPORT
            }
        }
    }

    fn allow(&self,
             app_id: AppId,
             minor_num: usize,
             slice: Option<AppSlice<Shared, u8>>
    ) -> ReturnCode {
        match minor_num {
                0 => {
                    // Key
                    self.apps
                        .enter(app_id, |app_data, _| {
                            if let Some(s) = slice {
                                if s.len() != AES128_KEY_SIZE {
                                    return ReturnCode::ESIZE;
                                }
                                app_data.key = Some(s);
                            } else {
                                app_data.key = slice;
                            }

                            ReturnCode::SUCCESS
                        })
                        .unwrap_or(ReturnCode::FAIL)
                }
                1 => {
                    // Input Buffer
                    self.apps
                        .enter(app_id, |app_data, _| {
                            if let Some(s) = slice {
                                if s.len() != AES128_BLOCK_SIZE {
                                    return ReturnCode::ESIZE;
                                }
                                app_data.input_buffer = Some(s);
                            } else {
                                app_data.input_buffer = slice;
                            }
                            ReturnCode::SUCCESS
                        })
                        .unwrap_or(ReturnCode::FAIL)
                }
                2 => {
                    // Output Buffer
                    self.apps
                        .enter(app_id, |app_data, _| {
                            if let Some(s) = slice {
                                if s.len() != AES128_BLOCK_SIZE {
                                    return ReturnCode::ESIZE;
                                }
                                app_data.output_buffer = Some(s);
                            } else {
                                app_data.output_buffer = slice;
                            }
                            ReturnCode::SUCCESS
                        })
                        .unwrap_or(ReturnCode::FAIL)
                }
                3 => {
                    // Initialization vector/Counter
                    self.apps
                        .enter(app_id, |app_data, _| {
                            if let Some(s) = slice {
                                if s.len() != AES128_BLOCK_SIZE {
                                    return ReturnCode::ESIZE;
                                }
                                app_data.iv_buffer = Some(s);
                            } else {
                                app_data.iv_buffer = slice;
                            }
                            ReturnCode::SUCCESS
                        })
                        .unwrap_or(ReturnCode::FAIL)
                }
            _ => ReturnCode::ENOSUPPORT,
        }
    }
}
