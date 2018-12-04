//! Provides userspace with access to a H1B USB peripheral.


use core::cell::Cell;
use kernel::{AppId, AppSlice, Callback, Driver, Grant, ReturnCode, Shared};
use usb::{UsbHidU2f, UsbHidU2fClient};

pub const DRIVER_NUM: usize = 0x20008;

#[derive(Default)]
pub struct App {
    callback: Option<Callback>,
    buffer: Option<AppSlice<Shared, u8>>,
}

pub struct U2fSyscallDriver<'a> {
    u2f_transport: &'a UsbHidU2f<'a>,
    apps: Grant<App>,
    busy: Cell<bool>,
}

impl<'a> U2fSyscallDriver<'a> {
    pub fn new(u2f: &'a UsbHidU2f<'a>, grant: Grant<App>) -> U2fSyscallDriver<'a> {
        U2fSyscallDriver {
            u2f_transport: u2f,
            apps: grant,
            busy: Cell::new(false)
        }
    }
}

impl<'a> UsbHidU2fClient<'a> for U2fSyscallDriver<'a> {
    fn reconnected(&self) {}
    fn frame_received(&self) {}
    fn frame_transmitted(&self) {}
}

impl<'a> Driver for U2fSyscallDriver<'a> {
    fn allow(
        &self,
        appid: AppId,
        allow_num: usize,
        slice: Option<AppSlice<Shared, u8>>,
    ) -> ReturnCode {
        match allow_num {
            0 => self.apps.enter(appid, |app, _| {
                app.buffer = slice;
                ReturnCode::SUCCESS
            }).unwrap_or_else(|err| err.into()),
            _ => ReturnCode::ENOSUPPORT,
        }
    }


    fn subscribe(
        &self,
        subscribe_num: usize,
        callback: Option<Callback>,
        app_id: AppId,
    ) -> ReturnCode {
        match subscribe_num {
            0 => self.apps.enter(app_id, |app, _| {
                app.callback = callback;
                ReturnCode::SUCCESS
            }).unwrap_or_else(|err| err.into()),

            _ => ReturnCode::ENOSUPPORT,
        }
    }

    fn command(&self, command_num: usize, _data: usize, _: usize, appid: AppId) -> ReturnCode {
        match command_num {
            0 => ReturnCode::SUCCESS, // Existence check
            1 => self.apps.enter(appid, |app, _| { // Send packet
                if app.callback.is_some() && app.buffer.is_some() {
                    if self.u2f_transport.transmit_ready() {
                        app.buffer.take().map_or(ReturnCode::ERESERVE, |buf| {
                            let rcode = self.u2f_transport.put_slice(buf.as_ref());
                            app.buffer = Some(buf);
                            rcode
                        })
                    }
                    else {
                        ReturnCode::EBUSY
                    }
                } else {
                    ReturnCode::ERESERVE
                }
            }).unwrap_or_else(|err| err.into()),
            _ => ReturnCode::ENOSUPPORT,
        }
    }
}
