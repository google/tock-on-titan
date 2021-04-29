use core::cell::Cell;
use core::convert::TryFrom;

use h1::hil::spi_device::SpiDevice;
use h1::hil::spi_device::SpiDeviceClient;

use kernel::AppId;
use kernel::AppSlice;
use kernel::Callback;
use kernel::Driver;
use kernel::Grant;
use kernel::ReturnCode;
use kernel::Shared;

use spiutils::driver::spi_device::AddressConfig;
use spiutils::driver::spi_device::HandlerMode;
use spiutils::protocol::flash::AddressMode;
use spiutils::protocol::flash::OpCode;
use spiutils::protocol::wire::FromWire;
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

    fn send_data(&self, caller_id: AppId, clear_busy: bool, clear_write_enable: bool) -> ReturnCode {
        self.apps.enter(caller_id, |app_data, _| {
            if let Some(ref tx_buffer) = app_data.tx_buffer {
                //debug!("send_data: clear_busy={:?}", clear_busy);
                let return_code = self.device.put_send_data(tx_buffer.as_ref());
                if isize::from(return_code) < 0 { return return_code; }

                if clear_write_enable { self.device.clear_write_enable(); }
                if clear_busy { self.device.clear_busy(); }
                return ReturnCode::SUCCESS;
            }

            ReturnCode::ENOMEM
        }).unwrap_or(ReturnCode::ENOMEM)
    }

    fn clear_status(&self, caller_id: AppId, clear_busy: bool, clear_write_enable: bool) -> ReturnCode {
        self.apps.enter(caller_id, |_app_data, _| {
            if clear_write_enable { self.device.clear_write_enable(); }
            if clear_busy { self.device.clear_busy(); }

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

    fn process_spi_cmd(&self, app_data: &AppData, spi_cmd: u8, maybe_spi_data: Option<u8>) -> Result<HandlerMode, FromWireError> {
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
            OpCode::WriteStatusRegister =>
                if let Some(spi_data) = maybe_spi_data {
                    if self.device.is_write_enable_set() {
                        self.device.set_status(spi_data);
                        self.device.clear_write_enable();
                    }
                    self.device.clear_busy();
                    Ok(HandlerMode::KernelSpace)
                } else {
                    Ok(HandlerMode::UserSpace)
                }
            _ => Ok(HandlerMode::UserSpace)
        }
    }

    fn set_jedec_id(&self, caller_id: AppId) -> ReturnCode {
        self.apps.enter(caller_id, |app_data, _| {
            if let Some(ref tx_buffer) = app_data.tx_buffer {
                self.device.set_jedec_id(tx_buffer.as_ref())
            } else {
                ReturnCode::ENOMEM
            }
        }).unwrap_or(ReturnCode::ENOMEM)
    }

    fn set_sfdp(&self, caller_id: AppId) -> ReturnCode {
        self.apps.enter(caller_id, |app_data, _| {
            if let Some(ref tx_buffer) = app_data.tx_buffer {
                self.device.set_sfdp(tx_buffer.as_ref())
            } else {
                ReturnCode::ENOMEM
            }
        }).unwrap_or(ReturnCode::ENOMEM)
    }

    fn configure_addresses(&self, caller_id: AppId) -> ReturnCode {
        self.apps.enter(caller_id, |app_data, _| {
            if let Some(ref tx_buffer) = app_data.tx_buffer {
                let maybe_address_config = AddressConfig::from_wire(tx_buffer.as_ref());
                if maybe_address_config.is_err() {
                    return ReturnCode::EINVAL;
                }

                self.device.configure_addresses(maybe_address_config.unwrap());

                ReturnCode::SUCCESS
            } else {
                ReturnCode::ENOMEM
            }
        }).unwrap_or(ReturnCode::ENOMEM)
    }
}

impl<'a> SpiDeviceClient for SpiDeviceSyscall<'a> {
    fn data_available(&self, is_busy: bool, is_write_enabled: bool) {
        //debug!("data_available");
        self.current_user.get().map(|current_user| {
            let _ = self.apps.enter(current_user, move |app_data, _| {
                let mut rx_len = 0;
                let mut handler_mode = HandlerMode::UserSpace;
                let mut maybe_spi_cmd : Option<u8> = None;
                let mut maybe_spi_data : Option<u8> = None;
                if let Some(ref mut rx_buffer) = app_data.rx_buffer {
                    rx_len = self.device.get_received_data(rx_buffer.as_mut());
                    if rx_len > 0 {
                        maybe_spi_cmd = Some(rx_buffer.as_ref()[0]);
                    }
                    if rx_len > 1 {
                        maybe_spi_data = Some(rx_buffer.as_ref()[1]);
                    }
                } else {
                    // Just grab the first two bytes
                    let mut spi_cmd_buf = [!0, !0];
                    let spi_cmd_buf_len = self.device.get_received_data(&mut spi_cmd_buf);
                    if spi_cmd_buf_len > 0 {
                        maybe_spi_cmd = Some(spi_cmd_buf[0]);
                    }
                    if spi_cmd_buf_len > 1 {
                        maybe_spi_data = Some(spi_cmd_buf[1]);
                    }
                }

                // Handle some special op code straight in kernel space
                if let Some(spi_cmd) = maybe_spi_cmd {
                    //debug!("spi_cmd: {:?}", spi_cmd);
                    handler_mode = match self.process_spi_cmd(app_data, spi_cmd, maybe_spi_data) {
                        Ok(mode) => mode,
                        Err(_) => HandlerMode::UserSpace,
                    }
                }

                //debug!("handler_mode: {:?}", handler_mode);
                if handler_mode == HandlerMode::UserSpace {
                    app_data.data_received_callback.map(
                        |mut cb| cb.schedule(rx_len, usize::from(is_busy), usize::from(is_write_enabled)));
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
                 arg2: whether BUSY bit is set (0: false, otherwise: true)
                 arg3: whether WRITE ENABLE bit is set (0: false, otherwise: true) */ => {
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

    fn command(&self, command_num: usize, arg1: usize, arg2: usize, caller_id: AppId) -> ReturnCode {
        //debug!("command: num={}", command_num);
        if self.current_user.get() == None {
            self.current_user.set(Some(caller_id));
        }
        match command_num {
            0 /* Check if present */ => ReturnCode::SUCCESS,
            1 /* Put send data using data from TX buffer
                 arg1: Whether to clear busy (0: false, != 0: true)
                 arg2: Whether to clear write enable (0: false, != 0: true) */ => {
                self.send_data(caller_id, arg1 != 0, arg2 != 0)
            },
            2 /* Clear status
                 arg1: Whether to clear busy (0: false, != 0: true)
                 arg2: Whether to clear write enable (0: false, != 0: true) */ => {
                self.clear_status(caller_id, arg1 != 0, arg2 != 0)
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
            6 /* Set JEDEC ID using data from TX buffer */ => {
                self.set_jedec_id(caller_id)
            }
            7 /* Set SFDP using data from TX buffer */ => {
                self.set_sfdp(caller_id)
            }
            8 /* Configure addresses using data from TX buffer */ => {
                self.configure_addresses(caller_id)
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
