use core::cell::Cell;
use h1::hil::spi_host::SpiHost;
use kernel::{AppId, Callback, Driver, Grant, ReturnCode, Shared, AppSlice};

pub const DRIVER_NUM: usize = 0x40020;

#[derive(Default)]
pub struct AppData {
}

pub struct SpiHostSyscall<'a> {
    device: &'a dyn SpiHost,
    apps: Grant<AppData>,
    current_user: Cell<Option<AppId>>,
}

impl<'a> SpiHostSyscall<'a> {
    pub fn new(device: &'a dyn SpiHost,
               container: Grant<AppData>) -> SpiHostSyscall<'a> {
        SpiHostSyscall {
            device: device,
            apps: container,
            current_user: Cell::new(None),
        }
    }

    fn spi_device_spi_host_passthrough(&self, caller_id: AppId, enable: bool) -> ReturnCode {
        self.apps.enter(caller_id, |_app_data, _| {
            self.device.spi_device_spi_host_passthrough(enable);
            ReturnCode::SUCCESS
        }).unwrap_or(ReturnCode::ENOMEM)
    }
}

impl<'a> Driver for SpiHostSyscall<'a> {
    fn subscribe(&self,
                 subscribe_num: usize,
                 _callback: Option<Callback>,
                 _app_id: AppId,
    ) -> ReturnCode {
        match subscribe_num {
            _ => ReturnCode::ENOSUPPORT
        }
    }

    fn command(&self, command_num: usize, arg1: usize, _arg2: usize, caller_id: AppId) -> ReturnCode {
        if self.current_user.get() == None {
            self.current_user.set(Some(caller_id));
        }
        match command_num {
            0 /* Check if present */ => ReturnCode::SUCCESS,
            1 /* Enable/disable SPI device <-> SPI host passthrough
                 arg1: 0: disable, != 0: enable) */ => {
                self.spi_device_spi_host_passthrough(caller_id, arg1 != 0)
            },
            _ => ReturnCode::ENOSUPPORT
        }
    }

    fn allow(&self,
             _app_id: AppId,
             minor_num: usize,
             _slice: Option<AppSlice<Shared, u8>>
    ) -> ReturnCode {
        match minor_num {
            _ => ReturnCode::ENOSUPPORT,
        }
    }
}
