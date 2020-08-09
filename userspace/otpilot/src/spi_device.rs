// Copyright 2020 lowRISC contributors.
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
use core::convert::TryFrom;
use core::fmt::Write;

use libtock::console::Console;
use libtock::result::TockError;
use libtock::result::TockResult;
use libtock::shared_memory::SharedMemory;
use libtock::syscalls;
use libtock::syscalls::raw::yieldk;

use spiutils::driver::HandlerMode;
use spiutils::protocol::flash::AddressMode;

pub const MAX_READ_BUFFER_SIZE: usize = 512;

#[allow(dead_code)]
pub const MAX_WRITE_BUFFER_SIZE: usize = 2048;

pub trait SpiDevice {
    /// Check if received a transaction.
    fn have_transaction(&self) -> bool;

    /// Clear the current received transaction.
    fn clear_transaction(&self);

    /// Wait for a transaction by yielding.
    fn wait_for_transaction(&self);

    /// Get the buffer slice of received data.
    fn get_read_buffer(&self) -> &[u8];

    /// Whether the transaction has the BUSY bit set.
    fn is_busy_set(&self) -> bool;

    /// Whether the transaction has the WRITE ENABLE bit set.
    fn is_write_enable_set(&self) -> bool;

    /// Clear the BUSY and/or the WRITE ENABLE bits.
    fn clear_status(&self, clear_busy: bool, clear_write_enable: bool) -> TockResult<()>;

    /// Send data to be made available to the SPI host and clear the BUSY
    /// and/or WRITE_ENABLE bits if requested.
    fn send_data(&self, write_buffer: &mut[u8], clear_busy: bool, clear_write_enable: bool)
    -> TockResult<()>;

    /// Configure the engine's address mode.
    fn set_address_mode(&self, address_mode: AddressMode) -> TockResult<()>;

    /// Get the engine's address mode.
    fn get_address_mode(&self) -> AddressMode;

    /// Set handling mode for address mode changes.
    fn set_address_mode_handling(&self, address_mode_handling: HandlerMode) -> TockResult<()>;
}

// Get the static SpiDevice object.
pub fn get() -> &'static dyn SpiDevice {
    get_impl()
}

const DRIVER_NUMBER: usize = 0x40030;

mod command_nr {
    pub const CHECK_IF_PRESENT: usize = 0;
    pub const SEND_DATA: usize = 1;
    pub const CLEAR_STATUS: usize = 2;
    pub const SET_ADDRESS_MODE: usize = 3;
    pub const GET_ADDRESS_MODE: usize = 4;
    pub const SET_ADDRESS_MODE_HANDLING: usize = 5;
}

mod subscribe_nr {
    pub const DATA_RECEIVED: usize = 0;
    pub const ADDRESS_MODE_CHANGED: usize = 1;
}

mod allow_nr {
    pub const WRITE_BUFFER: usize = 0;
    pub const READ_BUFFER: usize = 1;
}

struct SpiDeviceImpl {
    /// The receive buffer. Should be equal or larger than HW buffer.
    read_buffer: [u8; MAX_READ_BUFFER_SIZE],

    /// Shared memory object to allow kernel to access read_buffer.
    read_buffer_share: Cell<Option<SharedMemory<'static>>>,

    /// Number of received bytes.
    received_len: Cell<usize>,

    /// Whether the BUSY bit was set for the last received transaction.
    is_busy_set: Cell<bool>,

    /// Whether the WRITE ENABLE bit was set for the last received transaction.
    is_write_enable_set: Cell<bool>,

    /// The current address mode
    address_mode: Cell<AddressMode>,
}

static mut SPI_DEVICE: SpiDeviceImpl = SpiDeviceImpl {
    read_buffer: [0; MAX_READ_BUFFER_SIZE],
    read_buffer_share: Cell::new(None),
    received_len: Cell::new(0),
    is_busy_set: Cell::new(false),
    is_write_enable_set: Cell::new(false),
    address_mode: Cell::new(AddressMode::ThreeByte),
};

static mut IS_INITIALIZED: bool = false;

fn get_impl() -> &'static SpiDeviceImpl {
    unsafe {
        if !IS_INITIALIZED {
            if SPI_DEVICE.initialize().is_err() {
                panic!("Could not initialize SPI Device");
            }
            IS_INITIALIZED = true;
        }
        &SPI_DEVICE
    }
}

impl SpiDeviceImpl {
    // Initialize a static instance.
    // Registers buffers and callbacks in the kernel.
    fn initialize(&'static mut self) -> TockResult<()> {
        syscalls::command(DRIVER_NUMBER, command_nr::CHECK_IF_PRESENT, 0, 0)?;

        syscalls::subscribe_fn(
            DRIVER_NUMBER,
            subscribe_nr::ADDRESS_MODE_CHANGED,
            SpiDeviceImpl::address_mode_changed_trampoline,
            0)?;

        let address_mode_val = syscalls::command(DRIVER_NUMBER, command_nr::GET_ADDRESS_MODE, 0, 0)?;
        self.address_mode.set(match AddressMode::try_from(address_mode_val) {
            Ok(val) => val,
            Err(_) => return Err(TockError::Format),
        });

        self.read_buffer_share.set(Some(syscalls::allow(DRIVER_NUMBER, allow_nr::READ_BUFFER,
            &mut self.read_buffer)?));

        syscalls::subscribe_fn(
            DRIVER_NUMBER,
            subscribe_nr::DATA_RECEIVED,
            SpiDeviceImpl::data_received_trampoline,
            0)?;

        Ok(())
    }

    extern "C"
    fn data_received_trampoline(arg1: usize, arg2: usize, arg3: usize, _data: usize) {
        get_impl().data_received(arg1, arg2, arg3);
    }

    fn data_received(&self, arg1: usize, arg2: usize, arg3: usize) {
        // arg1: number of received bytes
        // arg2: whether BUSY bit is set
        // arg3: whether WRITE ENABLE bit is set
        self.received_len.set(arg1);
        self.is_busy_set.set(arg2 != 0);
        self.is_write_enable_set.set(arg3 != 0);
    }

    extern "C"
    fn address_mode_changed_trampoline(arg1: usize, arg2: usize, arg3: usize, _data: usize) {
        get_impl().address_mode_changed(arg1, arg2, arg3);
    }

    fn address_mode_changed(&self, arg1: usize, _: usize, _: usize) {
        // arg1: new AddressMode
        match AddressMode::try_from(arg1) {
            Ok(val) => self.address_mode.set(val),
            Err(_) => ()
        }

        let mut console = Console::new();
        writeln!(console, "address_mode_changed: {:?}", arg1);
    }
}

impl SpiDevice for SpiDeviceImpl {
    fn have_transaction(&self) -> bool {
        self.received_len.get() != 0
    }

    fn clear_transaction(&self) {
        self.received_len.set(0);
    }

    fn wait_for_transaction(&self) {
        self.clear_transaction();
        while !self.have_transaction() { unsafe { yieldk(); } }
    }

    fn get_read_buffer(&self) -> &[u8] {
        &(self.read_buffer[0..self.received_len.get()])
    }

    fn is_busy_set(&self) -> bool {
        self.is_busy_set.get()
    }

    fn is_write_enable_set(&self) -> bool {
        self.is_write_enable_set.get()
    }

    fn clear_status(&self, clear_busy: bool, clear_write_enable: bool) -> TockResult<()> {
        syscalls::command(DRIVER_NUMBER, command_nr::CLEAR_STATUS,
            if clear_busy { 1 } else { 0 },
            if clear_write_enable { 1 } else { 0 })?;
        Ok(())
    }

    fn send_data(&self, write_buffer: &mut[u8], clear_busy: bool, clear_write_enable: bool) -> TockResult<()> {
        // We want this to go out of scope after executing the command
        let _write_buffer_share = syscalls::allow(DRIVER_NUMBER, allow_nr::WRITE_BUFFER, write_buffer)?;

        syscalls::command(DRIVER_NUMBER, command_nr::SEND_DATA,
            if clear_busy { 1 } else { 0 },
            if clear_write_enable { 1 } else { 0 })?;

        Ok(())
    }

    fn set_address_mode(&self, address_mode: AddressMode) -> TockResult<()> {
        syscalls::command(DRIVER_NUMBER, command_nr::SET_ADDRESS_MODE, address_mode as usize, 0)?;
        self.address_mode.set(address_mode);
        Ok(())
    }

    fn get_address_mode(&self) -> AddressMode {
        return self.address_mode.get()
    }

    fn set_address_mode_handling(&self, address_mode_handling: HandlerMode) -> TockResult<()> {
        syscalls::command(DRIVER_NUMBER, command_nr::SET_ADDRESS_MODE_HANDLING, address_mode_handling as usize, 0)?;

        Ok(())
    }
}
