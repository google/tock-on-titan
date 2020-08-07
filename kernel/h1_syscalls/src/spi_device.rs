use core::cell::Cell;
use core::convert::TryFrom;

use h1::hil::spi_device::AddressConfig;
use h1::hil::spi_device::SpiDevice;
use h1::hil::spi_device::SpiDeviceClient;

use kernel::AppId;
use kernel::AppSlice;
use kernel::Callback;
use kernel::Driver;
use kernel::Grant;
use kernel::ReturnCode;
use kernel::Shared;

use spiutils::driver::HandlerMode;
use spiutils::protocol::flash::AddressMode;
use spiutils::protocol::flash::OpCode;
use spiutils::protocol::wire::FromWireError;
use spiutils::protocol::wire::WireEnum;

pub const DRIVER_NUM: usize = 0x40030;

#[derive(Default)]
pub struct AppData {
    tx_buffer: Option<AppSlice<Shared, u8>>,
    rx_buffer: Option<AppSlice<Shared, u8>>,
    data_received_callback: Option<Callback>,
    address_mode_handling: Cell<HandlerMode>,
    address_mode_changed_callback: Option<Callback>,
}

/// The virtual base address of the external flash
const EXT_FLASH_VIRTUAL_BASE: u32 = 0;

/// The size of the external flash
const EXT_FLASH_SIZE: u32 = 32 * 1024 * 1024;

/// The physical base address in the external flash
const EXT_FLASH_PHYSICAL_BASE: u32 = 0;

pub struct SpiDeviceSyscall<'a> {
    device: &'a dyn SpiDevice,
    apps: Grant<AppData>,
    current_user: Cell<Option<AppId>>,
}

impl<'a> SpiDeviceSyscall<'a> {
    pub fn new(device: &'a dyn SpiDevice,
               container: Grant<AppData>) -> SpiDeviceSyscall<'a> {
        // Temporary hard-coded address configuration
        let address_config = AddressConfig {
            flash_virtual_base: EXT_FLASH_VIRTUAL_BASE,
            flash_physical_base: EXT_FLASH_PHYSICAL_BASE,
            flash_physical_size: EXT_FLASH_SIZE,
            ram_virtual_base: EXT_FLASH_VIRTUAL_BASE + EXT_FLASH_SIZE,
            virtual_size: EXT_FLASH_SIZE * 2,
        };
        device.configure_addresses(address_config);

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

    fn set_address_mode(&self, caller_id: AppId, address_mode: AddressMode) -> ReturnCode {
        self.apps.enter(caller_id, |_app_data, _| {
            self.device.set_address_mode(address_mode);

            ReturnCode::SUCCESS
        }).unwrap_or(ReturnCode::ENOMEM)
    }

    fn get_address_mode(&self, caller_id: AppId) -> ReturnCode {
        self.apps.enter(caller_id, |_app_data, _| {
            ReturnCode::SuccessWithValue { value: self.device.get_address_mode() as usize }
        }).unwrap_or(ReturnCode::ENOMEM)
    }

    fn set_address_mode_handling(&self, caller_id: AppId, address_mode_handling: HandlerMode) -> ReturnCode {
        self.apps.enter(caller_id, |app_data, _| {
            app_data.address_mode_handling.set(address_mode_handling);
            ReturnCode::SUCCESS
        }).unwrap_or(ReturnCode::ENOMEM)
    }

    fn process_spi_cmd(&self, app_data: &AppData, spi_cmd: u8) -> Result<HandlerMode, FromWireError> {
        let op_code = OpCode::from_wire_value(spi_cmd).ok_or(FromWireError::OutOfRange)?;

        match op_code {
            OpCode::Enter4ByteAddressMode | OpCode::Exit4ByteAddressMode =>
                match app_data.address_mode_handling.get() {
                    HandlerMode::KernelSpace => {
                        let address_mode = match op_code {
                            OpCode::Enter4ByteAddressMode => AddressMode::FourByte,
                            OpCode::Exit4ByteAddressMode => AddressMode::ThreeByte,
                            _ => return Err(FromWireError::OutOfRange)
                        };
                        let mut has_address_mode_changed = false;
                        if self.device.get_address_mode() != address_mode {
                            self.device.set_address_mode(address_mode);
                            has_address_mode_changed = true;
                        }
                        self.device.clear_busy();
                        if has_address_mode_changed {
                            app_data.address_mode_changed_callback.map(
                                |mut cb| cb.schedule(usize::from(address_mode), 0, 0));
                        }
                        Ok(HandlerMode::KernelSpace)
                    }
                    handler_mode => Ok(handler_mode)
                },
            _ => Ok(HandlerMode::UserSpace)
        }
    }

}

impl<'a> SpiDeviceClient for SpiDeviceSyscall<'a> {
    fn data_available(&self, is_busy: bool) {
        //debug!("data_available");
        self.current_user.get().map(|current_user| {
            let _ = self.apps.enter(current_user, move |app_data, _| {
                let mut rx_len = 0;
                let mut handler_mode = HandlerMode::UserSpace;
                let mut maybe_spi_cmd : Option<u8> = None;
                if let Some(ref mut rx_buffer) = app_data.rx_buffer {
                    rx_len = self.device.get_received_data(rx_buffer.as_mut());
                    if rx_len > 0 {
                        maybe_spi_cmd = Some(rx_buffer.as_ref()[0]);
                    }
                } else {
                    // Just grab the first byte
                    let mut spi_cmd_buf = [0];
                    let spi_cmd_buf_len = self.device.get_received_data(&mut spi_cmd_buf);
                    if spi_cmd_buf_len > 0 {
                        maybe_spi_cmd = Some(spi_cmd_buf[0]);
                    }
                }

                // Handle some special op code straight in kernel space
                if let Some(spi_cmd) = maybe_spi_cmd {
                    handler_mode = match self.process_spi_cmd(app_data, spi_cmd) {
                        Ok(mode) => mode,
                        Err(_) => HandlerMode::UserSpace,
                    }
                }

                if handler_mode == HandlerMode::UserSpace {
                    app_data.data_received_callback.map(
                        |mut cb| cb.schedule(rx_len, usize::from(is_busy), 0));
                }
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
            0 /* Data received
                 Callback arguments:
                 arg1: number of received bytes
                 arg2: whether BUSY bit is set (0: false, otherwise: true) */ => {
                self.apps.enter(app_id, |app_data, _| {
                    app_data.data_received_callback = callback;
                    ReturnCode::SUCCESS
                }).unwrap_or(ReturnCode::ENOMEM)
            },
            1 /* Address mode changed
                 Callback arguments:
                 arg1: new AddressMode as usize */ => {
                self.apps.enter(app_id, |app_data, _| {
                    app_data.address_mode_changed_callback = callback;
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
            3 /* Set address mode
                 arg1: AddressMode as usize */ => {
                let address_mode = match AddressMode::try_from(arg1) {
                    Ok(val) => val,
                    Err(_) => return ReturnCode::EINVAL
                };
                self.set_address_mode(caller_id, address_mode)
            },
            4 /* Get address mode
                 returns: AddressMode as usize */ => {
                self.get_address_mode(caller_id)
            }
            5 /* Configure address mode handling
                 (OpCode::Enter4ByteAddressMode and OpCode::Exit4ByteAddressMode)
                 arg1: HandlerMode as usize */ => {
                let handler_mode = match HandlerMode::try_from(arg1) {
                    Ok(val) => val,
                    Err(_) => return ReturnCode::EINVAL
                };
                self.set_address_mode_handling(caller_id, handler_mode)
            }
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
