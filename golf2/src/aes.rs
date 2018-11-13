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
use h1b::crypto::aes::AesEngine;
use h1b::hil::aes::{AesClient, Interrupt, KeySize};
use kernel::{AppId, Callback, Driver, Grant, ReturnCode, Shared, AppSlice};

pub const DRIVER_NUM: usize = 0x40000;

#[derive(Default)]
struct Callbacks {
    done_cipher: Option<Callback>,
    done_key_expansion: Option<Callback>,
    done_wipe_secrets: Option<Callback>,
}

#[derive(Default)]
pub struct AppData {
    key: Option<AppSlice<Shared, u8>>,
    input_buffer: Option<AppSlice<Shared, u8>>,
    output_buffer: Option<AppSlice<Shared, u8>>,
    callbacks: Callbacks,
}

pub struct AesDriver<'a> {
    device: &'a AesEngine,
    apps: Grant<AppData>,
    current_user: Cell<Option<AppId>>,
    bytes_encrypted: Cell<usize>,
}

impl<'a> AesDriver<'a> {
    pub fn new(device: &'a mut AesEngine, container: Grant<AppData>) -> AesDriver<'a> {
        AesDriver {
            device: device,
            apps: container,
            current_user: Cell::new(None),
            bytes_encrypted: Cell::new(0),
        }
    }

    fn setup(&self, caller_id: AppId, key_size: usize) -> ReturnCode {
        self.apps
            .enter(caller_id, |app_data, _| {
                let key_size = match key_size {
                    0 => KeySize::KeySize128,
                    1 => KeySize::KeySize192,
                    2 => KeySize::KeySize256,
                    _ => return ReturnCode::EINVAL,
                };

                let raw_key = match app_data.key {
                    Some(ref slice) => slice,
                    None => return ReturnCode::EINVAL,
                };

                match (key_size, raw_key.len()) {
                    (KeySize::KeySize128, 16) => {}
                    (KeySize::KeySize192, 24) => {}
                    (KeySize::KeySize256, 32) => {}
                    _ => {
                        debug!("Key size is wrong. Given {}, expected {:?}",
                                 raw_key.len() * 8,
                                 key_size);
                        return ReturnCode::EINVAL;
                    }
                }

                let mut key = [0; 8];
                for (i, word) in raw_key.as_ref().chunks(4).enumerate() {
                    key[i] = word.iter()
                        .map(|b| *b as u32)
                        .enumerate()
                        .fold(0, |accm, (i, byte)| accm | (byte << (i * 8)));
                }

                if self.current_user.get().is_some() {
                    return ReturnCode::EBUSY;
                }
                self.current_user.set(Some(caller_id));

                self.device.setup(key_size, &key);

                ReturnCode::SUCCESS
            })
            .unwrap_or_else(|err| err.into())
    }

    fn set_encrypt_mode(&self, caller_id: AppId, do_encrypt: usize) -> ReturnCode {
        self.apps
            .enter(caller_id, |_, _| {
                match self.current_user.get() {
                    Some(cur) if cur.idx() == caller_id.idx() => {}
                    _ => return ReturnCode::EBUSY,
                }

                self.device.set_encrypt_mode(do_encrypt != 0);

                ReturnCode::SUCCESS
            })
            .unwrap_or(ReturnCode::FAIL)
    }

    fn crypt(&self, caller_id: AppId) -> ReturnCode {
        self.apps
            .enter(caller_id, |app_data, _| {
                match self.current_user.get() {
                    Some(cur) if cur.idx() == caller_id.idx() => {}
                    _ => return ReturnCode::EBUSY,
                }

                let input_buffer = match app_data.input_buffer {
                    Some(ref slice) => slice,
                    None => return ReturnCode::EINVAL,
                };
                let count = self.device.crypt(input_buffer.as_ref());
                self.bytes_encrypted.set(count);
                return ReturnCode::SUCCESS;
            }).unwrap_or_else(|err| err.into())
    }

    fn read_data(&self, caller_id: AppId) -> Result<isize, ReturnCode> {
        self.apps
            .enter(caller_id, |app_data, _| {
                match self.current_user.get() {
                    Some(cur) if cur.idx() == caller_id.idx() => {}
                    _ => return Err(ReturnCode::EBUSY),
                }

                let output_buffer = match app_data.output_buffer {
                    Some(ref mut slice) => slice,
                    None => return Err(ReturnCode::ENOMEM),
                };

                Ok(self.device.read_data(output_buffer.as_mut()) as isize)
            })
            .unwrap_or(Err(ReturnCode::FAIL))
    }

    fn finish(&self, caller_id: AppId) -> ReturnCode {
        self.apps
            .enter(caller_id, |_, _| {
                match self.current_user.get() {
                    Some(cur) if cur.idx() == caller_id.idx() => {}
                    _ => return ReturnCode::EBUSY,
                }

                self.current_user.set(None);

                self.device.finish();

                ReturnCode::SUCCESS
            })
            .unwrap_or(ReturnCode::FAIL)
    }

    fn register(&self,
                interrupt: Interrupt,
                callback: Option<Callback>,
                app_id: AppId,
    ) -> ReturnCode {
        self.apps
            .enter(app_id, |app_data, _| {
                let ref mut cb = app_data.callbacks;
                match interrupt {
                    Interrupt::DoneCipher => cb.done_cipher = callback,
                    Interrupt::DoneKeyExpansion => cb.done_key_expansion = callback,
                    Interrupt::DoneWipeSecrets => cb.done_wipe_secrets = callback,
                    _ => return ReturnCode::ENOSUPPORT,
                }

                ReturnCode::SUCCESS
            })
            .unwrap_or_else(|err| err.into())
    }
}

impl<'a> Driver for AesDriver<'a> {
    fn subscribe(&self,
                 subscribe_num: usize,
                 callback: Option<Callback>,
                 app_id: AppId,
    ) -> ReturnCode {
        match subscribe_num {
            0 => self.register(Interrupt::DoneCipher, callback, app_id),
            1 => self.register(Interrupt::DoneKeyExpansion, callback, app_id),
            2 => self.register(Interrupt::DoneWipeSecrets, callback, app_id),
            _ => ReturnCode::ENOSUPPORT
        }
    }

    fn command(&self, command_num: usize, arg1: usize, _: usize, caller_id: AppId) -> ReturnCode {
        match command_num {
            
            0 /* Check if present */ => ReturnCode::SUCCESS,            
            1 /* init encryption */ => self.setup(caller_id, arg1),
            2 /* start encryption */ => {
                self.crypt(caller_id)
            }
            3 /* read data */ => {
                match self.read_data(caller_id) {
                    Ok(_) => ReturnCode::SUCCESS,
                    Err(e) => e
                }
            }
            4 /* finish encryption */ => self.finish(caller_id),
            5 /* set encryption mode */ => {
                self.set_encrypt_mode(caller_id, arg1)
            }, 
            _ => ReturnCode::ENOSUPPORT,
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
                            app_data.key = slice;
                            ReturnCode::SUCCESS
                        })
                        .unwrap_or(ReturnCode::FAIL)
                }
                1 => {
                    // Input Buffer
                    self.apps
                        .enter(app_id, |app_data, _| {
                            app_data.input_buffer = slice;
                            ReturnCode::SUCCESS
                        })
                        .unwrap_or(ReturnCode::FAIL)
                }
                2 => {
                    // Output Buffer
                    self.apps
                        .enter(app_id, |app_data, _| {
                            app_data.output_buffer = slice;
                            ReturnCode::SUCCESS
                        })
                        .unwrap_or(ReturnCode::FAIL)
                }
                _ => ReturnCode::ENOSUPPORT,
            }
    }
}

impl<'a> AesClient for AesDriver<'a> {
    fn done_cipher(&self) {
        self.current_user.get().map(|current_user| {
            let _ = self.apps.enter(current_user, |app_data, _| {
                let val = match app_data.output_buffer {
                    Some(ref mut slice) => {
                        self.device.read_data(slice.as_mut())
                    },
                    None => {0}
                };
                app_data.callbacks.done_cipher.map(|mut cb| cb.schedule(val, 0, 0));
            });
        });
    }
    fn done_key_expansion(&self) {
        self.current_user.get().map(|current_user| {
            let _ = self.apps.enter(current_user, |app_data, _| {
                app_data.callbacks.done_key_expansion.map(|mut cb| cb.schedule(0, 0, 0));
            });
        });
    }
    fn done_wipe_secrets(&self) {
        self.current_user.get().map(|current_user| {
            let _ = self.apps.enter(current_user, |app_data, _| {
                app_data.callbacks.done_wipe_secrets.map(|mut cb| cb.schedule(0, 0, 0));
            });
        });
    }
}
