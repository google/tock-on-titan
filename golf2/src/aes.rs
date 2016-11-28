use core::cell::Cell;
use hotel::crypto::aes::AesEngine;
use hotel::hil::aes::{AesClient, Interrupt, KeySize};
use hotel::hil::common::SyscallError;
use kernel::{AppId, Callback, Driver, Container, Shared, AppSlice};

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
    apps: Container<AppData>,
    current_user: Cell<Option<AppId>>,
}

impl<'a> AesDriver<'a> {
    pub fn new(device: &'a mut AesEngine, container: Container<AppData>) -> AesDriver<'a> {
        AesDriver {
            device: device,
            apps: container,
            current_user: Cell::new(None),
        }
    }

    fn setup(&self, caller_id: AppId, key_size: usize) -> Result<isize, SyscallError> {
        self.apps
            .enter(caller_id, |app_data, _| {
                let key_size = match key_size {
                    0 => KeySize::KeySize128,
                    1 => KeySize::KeySize192,
                    2 => KeySize::KeySize256,
                    _ => return Err(SyscallError::InvalidArgument),
                };

                let key = match app_data.key {
                    Some(ref slice) => slice,
                    None => return Err(SyscallError::InvalidArgument),
                };

                if self.current_user.get().is_some() {
                    return Err(SyscallError::ResourceBusy);
                }
                self.current_user.set(Some(caller_id));

                try!(self.device
                    .setup(key_size, key.as_ref())
                    .map_err(|_| SyscallError::InternalError));

                Ok(0)
            })
            .unwrap_or(Err(SyscallError::InternalError))
    }

    fn set_encrypt_mode(&self, caller_id: AppId, do_encrypt: usize) -> Result<isize, SyscallError> {
        self.apps
            .enter(caller_id, |_, _| {
                match self.current_user.get() {
                    Some(cur) if cur.idx() == caller_id.idx() => {}
                    _ => return Err(SyscallError::ResourceBusy),
                }

                try!(self.device
                    .set_encrypt_mode(do_encrypt != 0)
                    .map_err(|_| SyscallError::InternalError));

                Ok(0)
            })
            .unwrap_or(Err(SyscallError::InternalError))
    }

    fn crypt(&self, caller_id: AppId) -> Result<isize, SyscallError> {
        self.apps
            .enter(caller_id, |app_data, _| {
                match self.current_user.get() {
                    Some(cur) if cur.idx() == caller_id.idx() => {}
                    _ => return Err(SyscallError::InvalidState),
                }

                let input_buffer = match app_data.input_buffer {
                    Some(ref slice) => slice,
                    None => return Err(SyscallError::InvalidArgument),
                };

                let size = try!(self.device
                    .crypt(input_buffer.as_ref())
                    .map_err(|_| SyscallError::InternalError));

                Ok(size as isize)
            })
            .unwrap_or(Err(SyscallError::InternalError))
    }

    fn read_data(&self, caller_id: AppId) -> Result<isize, SyscallError> {
        self.apps
            .enter(caller_id, |app_data, _| {
                match self.current_user.get() {
                    Some(cur) if cur.idx() == caller_id.idx() => {}
                    _ => return Err(SyscallError::InvalidState),
                }

                let output_buffer = match app_data.output_buffer {
                    Some(ref mut slice) => slice,
                    None => return Err(SyscallError::InvalidState),
                };

                let size = try!(self.device
                    .read_data(output_buffer.as_mut())
                    .map_err(|_| SyscallError::InternalError));

                Ok(size as isize)
            })
            .unwrap_or(Err(SyscallError::InternalError))
    }

    fn finish(&self, caller_id: AppId) -> Result<isize, SyscallError> {
        self.apps
            .enter(caller_id, |_, _| {
                match self.current_user.get() {
                    Some(cur) if cur.idx() == caller_id.idx() => {}
                    _ => return Err(SyscallError::InvalidState),
                }

                self.current_user.set(None);

                try!(self.device.finish().map_err(|_| SyscallError::InternalError));

                Ok(0)
            })
            .unwrap_or(Err(SyscallError::InternalError))
    }

    fn register(&self, interrupt: Interrupt, callback: Callback) -> isize {
        self.apps
            .enter(callback.app_id(), |app_data, _| {
                let ref mut cb = app_data.callbacks;
                match interrupt {
                    Interrupt::DoneCipher => cb.done_cipher = Some(callback),
                    Interrupt::DoneKeyExpansion => cb.done_key_expansion = Some(callback),
                    Interrupt::DoneWipeSecrets => cb.done_wipe_secrets = Some(callback),
                    _ => return -1,
                }

                0
            })
            .unwrap_or(-1)
    }
}

impl<'a> Driver for AesDriver<'a> {
    fn subscribe(&self, subscribe_num: usize, callback: Callback) -> isize {
        match subscribe_num {
            0 => self.register(Interrupt::DoneCipher, callback),
            1 => self.register(Interrupt::DoneKeyExpansion, callback),
            2 => self.register(Interrupt::DoneWipeSecrets, callback),

            _ => -1,
        }
    }

    fn command(&self, command_num: usize, arg1: usize, caller_id: AppId) -> isize {
        match command_num {
                // initialize encryption
                0 => self.setup(caller_id, arg1),
                1 => self.crypt(caller_id),
                2 => self.read_data(caller_id),
                3 => self.finish(caller_id),
                4 => self.set_encrypt_mode(caller_id, arg1),
                _ => Err(SyscallError::NotImplemented),
            }
            .unwrap_or_else(|err| err.into())
    }

    fn allow(&self, app_id: AppId, minor_num: usize, slice: AppSlice<Shared, u8>) -> isize {
        match minor_num {
                0 => {
                    // Key
                    self.apps
                        .enter(app_id, |app_data, _| {
                            app_data.key = Some(slice);
                            Ok(0)
                        })
                        .unwrap_or(Err(SyscallError::InternalError))
                }
                1 => {
                    // Input Buffer
                    self.apps
                        .enter(app_id, |app_data, _| {
                            app_data.input_buffer = Some(slice);
                            Ok(0)
                        })
                        .unwrap_or(Err(SyscallError::InternalError))
                }
                2 => {
                    // Output Buffer
                    self.apps
                        .enter(app_id, |app_data, _| {
                            app_data.output_buffer = Some(slice);
                            Ok(0)
                        })
                        .unwrap_or(Err(SyscallError::InternalError))
                }
                _ => Err(SyscallError::NotImplemented),
            }
            .unwrap_or_else(|err| err.into())
    }
}

impl<'a> AesClient for AesDriver<'a> {
    fn done_cipher(&self) {
        self.current_user.get().map(|current_user| {
            let _ = self.apps.enter(current_user, |app_data, _| {
                app_data.callbacks.done_cipher.map(|mut cb| cb.schedule(0, 0, 0));
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
