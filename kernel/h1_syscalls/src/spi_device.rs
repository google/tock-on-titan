use core::cell::Cell;
use h1::hil::spi_device::{SpiDevice, SpiDeviceClient};
use kernel::{AppId, Callback, Driver, Grant, ReturnCode, Shared, AppSlice};

pub const DRIVER_NUM: usize = 0x40030;

#[derive(Default)]
pub struct AppData {
    tx_buffer: Option<AppSlice<Shared, u8>>,
    rx_buffer: Option<AppSlice<Shared, u8>>,
    data_received_callback: Option<Callback>,
}

pub struct SpiDeviceSyscall<'a> {
    device: &'a dyn SpiDevice,
    apps: Grant<AppData>,
    current_user: Cell<Option<AppId>>,
}

impl<'a> SpiDeviceSyscall<'a> {
    pub fn new(device: &'a dyn SpiDevice,
               container: Grant<AppData>) -> SpiDeviceSyscall<'a> {
        SpiDeviceSyscall {
            device: device,
            apps: container,
            current_user: Cell::new(None),
        }
    }

    fn send_data(&self, caller_id: AppId, clear_busy: bool) -> ReturnCode {
        self.apps.enter(caller_id, |app_data, _| {
            if let Some(ref tx_buffer) = app_data.tx_buffer {
                //debug!("send_data: clear_busy={:?}", clear_busy);
                let return_code = self.device.put_send_data(tx_buffer.as_ref());
                if isize::from(return_code) < 0 { return return_code; }

                if clear_busy { self.device.clear_busy(); }
                return ReturnCode::SUCCESS;
            }

            ReturnCode::ENOMEM
        }).unwrap_or(ReturnCode::ENOMEM)
    }

    fn clear_busy(&self, caller_id: AppId) -> ReturnCode {
        self.apps.enter(caller_id, |_app_data, _| {
            //debug!("clear_busy");
            self.device.clear_busy();

            ReturnCode::SUCCESS
        }).unwrap_or(ReturnCode::ENOMEM)
    }
}

impl<'a> SpiDeviceClient for SpiDeviceSyscall<'a> {
    fn data_available(&self, is_busy: bool) {
        //debug!("data_available");
        self.current_user.get().map(|current_user| {
            let _ = self.apps.enter(current_user, move |app_data, _| {
                let mut rx_len = 0;
                if let Some(ref mut rx_buffer) = app_data.rx_buffer {
                    rx_len = self.device.get_received_data(rx_buffer.as_mut());
                }
                app_data.data_received_callback.map(
                    |mut cb| cb.schedule(rx_len, usize::from(is_busy), 0));
            });
        });
    }
}

impl<'a> Driver for SpiDeviceSyscall<'a> {
    fn subscribe(&self,
                 subscribe_num: usize,
                 callback: Option<Callback>,
                 app_id: AppId,
    ) -> ReturnCode {
        //debug!("subscribe: num={}, callback={}",
        //    subscribe_num, if callback.is_some() { "Some" } else { "None" });
        match subscribe_num {
            0 => { // Data received
                self.apps.enter(app_id, |app_data, _| {
                    app_data.data_received_callback = callback;
                    ReturnCode::SUCCESS
                }).unwrap_or(ReturnCode::ENOMEM)
            },
            _ => ReturnCode::ENOSUPPORT
        }
    }

    fn command(&self, command_num: usize, arg1: usize, _: usize, caller_id: AppId) -> ReturnCode {
        //debug!("command: num={}", command_num);
        if self.current_user.get() == None {
            self.current_user.set(Some(caller_id));
        }
        match command_num {
            0 /* Check if present */ => ReturnCode::SUCCESS,
            1 /* Put send data
                 arg1: Whether to also clear busy (0: false, != 0: true) */ => {
                self.send_data(caller_id, arg1 != 0)
            },
            2 /* Clear busy */ => {
                self.clear_busy(caller_id)
            },
            _ => ReturnCode::ENOSUPPORT
        }
    }

    fn allow(&self,
             app_id: AppId,
             minor_num: usize,
             slice: Option<AppSlice<Shared, u8>>
    ) -> ReturnCode {
        //debug!("allow: num={}, slice={}",
        //    minor_num, if slice.is_some() { "Some" } else { "None" });
        match minor_num {
                0 => {
                    // TX Buffer
                    self.apps
                        .enter(app_id, |app_data, _| {
                            if let Some(s) = slice {
                                app_data.tx_buffer = Some(s);
                            } else {
                                app_data.tx_buffer = slice;
                            }
                            ReturnCode::SUCCESS
                        })
                        .unwrap_or(ReturnCode::FAIL)
                }
                1 => {
                    // RX Buffer
                    self.apps
                        .enter(app_id, |app_data, _| {
                            if let Some(s) = slice {
                                app_data.rx_buffer = Some(s);
                            } else {
                                app_data.rx_buffer = slice;
                            }
                            ReturnCode::SUCCESS
                        })
                        .unwrap_or(ReturnCode::FAIL)
                }
            _ => ReturnCode::ENOSUPPORT,
        }
    }
}
