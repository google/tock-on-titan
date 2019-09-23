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


//! Provides userspace with access to a H1B USB peripheral.


use core::cell::Cell;
use kernel::{AppId, AppSlice, Callback, Driver, Grant, ReturnCode, Shared};
use usb::{UsbHidU2f, UsbHidU2fClient};

pub const DRIVER_NUM: usize = 0x20008;

pub const U2F_CMD_CHECK:    usize = 0;
pub const U2F_CMD_TRANSMIT: usize = 1;
pub const U2F_CMD_RECEIVE:  usize = 2;

pub const U2F_ALLOW_TRANSMIT: usize = 1;
pub const U2F_ALLOW_RECEIVE:  usize = 2;

pub const U2F_SUBSCRIBE_TRANSMIT_DONE: usize = 1;
pub const U2F_SUBSCRIBE_RECEIVE_DONE:  usize = 2;
pub const U2F_SUBSCRIBE_RECONNECT:     usize = 3;

#[derive(Default)]
pub struct App {
    tx_callback: Option<Callback>,
    rx_callback: Option<Callback>,
    connection_callback: Option<Callback>,
    tx_buffer: Option<AppSlice<Shared, u8>>,
    rx_buffer: Option<AppSlice<Shared, u8>>,
}

pub struct U2fSyscallDriver<'a> {
    u2f_endpoints: &'a dyn UsbHidU2f<'a>,
    apps: Grant<App>,
    busy: Cell<bool>,
}

impl<'a> U2fSyscallDriver<'a> {
    pub fn new(u2f: &'a dyn UsbHidU2f<'a>, grant: Grant<App>) -> U2fSyscallDriver<'a> {
        U2fSyscallDriver {
            u2f_endpoints: u2f,
            apps: grant,
            busy: Cell::new(false)
        }
    }
}

impl<'a> UsbHidU2fClient<'a> for U2fSyscallDriver<'a> {
    fn reconnected(&self) {
        for cntr in self.apps.iter() {
            cntr.enter(|app, _| {
                app.connection_callback.map(|mut cb| {
                    cb.schedule(0, 0, 0);
                });
            });
        }
    }

    fn frame_received(&self) {
        for cntr in self.apps.iter() {
            cntr.enter(|app, _| {
                if app.rx_buffer.is_some() {
                    let mut buf = app.rx_buffer.take().unwrap();
                    self.u2f_endpoints.get_slice(buf.as_mut());
                    app.rx_buffer = Some(buf);
                }
                app.rx_callback.map(|mut cb| cb.schedule(0, 0, 0));
            });
        }
    }

    fn frame_transmitted(&self) {
        for cntr in self.apps.iter() {
            cntr.enter(|app, _| {
                app.tx_callback.map(|mut cb| {
                    cb.schedule(0, 0, 0);
                });
            });
        }
    }
}

impl<'a> Driver for U2fSyscallDriver<'a> {
    fn allow(
        &self,
        appid: AppId,
        allow_num: usize,
        slice: Option<AppSlice<Shared, u8>>,
    ) -> ReturnCode {
        match allow_num {
            U2F_ALLOW_TRANSMIT => self.apps.enter(appid, |app, _| {
                app.tx_buffer = slice;
                ReturnCode::SUCCESS
            }).unwrap_or_else(|err| err.into()),
            U2F_ALLOW_RECEIVE => self.apps.enter(appid, |app, _| {
                app.rx_buffer = slice;
                ReturnCode::SUCCESS
            }).unwrap_or_else(|err| err.into()),
            _ => ReturnCode::ENOSUPPORT,
        }
    }

    /// The USB driver supports 3 callbacks:
    ///    - 0: Transmit complete
    ///    - 1: Receive complete
    ///    - 2: Reconnected
    fn subscribe(
        &self,
        subscribe_num: usize,
        callback: Option<Callback>,
        app_id: AppId,
    ) -> ReturnCode {
        match subscribe_num {
            U2F_SUBSCRIBE_TRANSMIT_DONE => self.apps.enter(app_id, |app, _| {
                app.tx_callback = callback;
                ReturnCode::SUCCESS
            }).unwrap_or_else(|err| err.into()),

            U2F_SUBSCRIBE_RECEIVE_DONE => self.apps.enter(app_id, |app, _| {
                app.rx_callback = callback;
                ReturnCode::SUCCESS
            }).unwrap_or_else(|err| err.into()),

            U2F_SUBSCRIBE_RECONNECT => self.apps.enter(app_id, |app, _| {
                app.connection_callback = callback;
                ReturnCode::SUCCESS
            }).unwrap_or_else(|err| err.into()),

            _ => ReturnCode::ENOSUPPORT,
        }
    }

    fn command(&self, command_num: usize, _data: usize, _: usize, appid: AppId) -> ReturnCode {
        match command_num {
            U2F_CMD_CHECK => ReturnCode::SUCCESS, // Existence check
            U2F_CMD_TRANSMIT => self.apps.enter(appid, |app, _| { // Send packet
                if app.tx_callback.is_some() && app.tx_buffer.is_some() {
                    //print!("U2F transmit: waiting for transmit ready.\n");
                    while !self.u2f_endpoints.transmit_ready() {}
                    if self.u2f_endpoints.transmit_ready() {
                        app.tx_buffer.take().map_or(ReturnCode::ERESERVE, |buf| {
                            let rcode = self.u2f_endpoints.put_slice(buf.as_ref());
                            app.tx_buffer = Some(buf);
                            //print!("U2F transmit: returning to userspace.\n");
                            rcode
                        })
                    }
                    else {
                        print!("U2F syscall: tried to transmit but not ready. Return EBUSY.\n");
                        ReturnCode::EBUSY
                    }
                } else {
                    ReturnCode::ERESERVE
                }
            }).unwrap_or_else(|err| err.into()),
            // Because the device cannot control when the host will send OUT packets,
            // having a receive command doesn't make sense. Instead, received OUT packets
            // are callbacks. The command number is reserved in case a future refactoring
            // calls for using commands in the receive path. -pal 12/19/18
            U2F_CMD_RECEIVE => {
                self.u2f_endpoints.enable_rx()
            },
            _ => ReturnCode::ENOSUPPORT,
        }
    }
}
