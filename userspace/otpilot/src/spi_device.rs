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

use libtock::result::TockResult;
use libtock::shared_memory::SharedMemory;
use libtock::syscalls;
use libtock::syscalls::raw::yieldk;

pub const MAX_READ_BUFFER_SIZE : usize = 512;

#[allow(dead_code)]
pub const MAX_WRITE_BUFFER_SIZE : usize = 2048;

pub trait SpiDevice {
    // Check if received a transaction.
    fn have_transaction(&self) -> bool;

    // Wait for a transaction by yielding.
    fn wait_for_transaction(&self);

    // Get the buffer slice of received data.
    fn get_read_buffer(&self) -> &[u8];

    // Whether the transaction has the BUSY bit set.
    fn is_busy_set(&self) -> bool;

    // Clear the BUSY bit if set.
    fn clear_busy(&self) -> TockResult<()>;

    // Send data to be made available to the SPI host and clear the BUSY bit if requested.
    fn send_data(&self, write_buffer: &mut[u8], clear_busy: bool) -> TockResult<()>;
}

// Get the static SpiDevice object.
pub fn get() -> &'static dyn SpiDevice {
    get_impl()
}

const DRIVER_NUMBER: usize = 0x40030;

mod command_nr {
    pub const CHECK_IF_PRESENT: usize = 0;
    pub const SEND_DATA: usize = 1;
    pub const CLEAR_BUSY: usize = 2;
}

mod subscribe_nr {
    pub const DATA_RECEIVED: usize = 0;
}

mod allow_nr {
    pub const WRITE_BUFFER: usize = 0;
    pub const READ_BUFFER: usize = 1;
}

struct SpiDeviceImpl {
    // The receive buffer. Should be equal or larger than HW buffer.
    read_buffer: [u8; MAX_READ_BUFFER_SIZE],

    // Shared memory object to allow kernel to access read_buffer.
    read_buffer_share: Cell<Option<SharedMemory<'static>>>,

    // Number of received bytes.
    received_len: Cell<usize>,

    // Whether the BUSY bit was set for the last received transaction.
    is_busy_set: Cell<bool>,
}

static mut SPI_DEVICE: SpiDeviceImpl = SpiDeviceImpl {
    read_buffer: [0; MAX_READ_BUFFER_SIZE],
    read_buffer_share: Cell::new(None),
    received_len: Cell::new(0),
    is_busy_set: Cell::new(false),
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
        self.read_buffer_share.set(Some(syscalls::allow(DRIVER_NUMBER, allow_nr::READ_BUFFER,
            &mut self.read_buffer)?));

        // Register callback so that we can receive data immediately.
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

    fn data_received(&self, arg1: usize, arg2: usize, _: usize) {
        // arg1: number of received bytes
        // arg2: whether BUSY bit is set
        self.received_len.set(arg1);
        self.is_busy_set.set(arg2 != 0);
    }
}

impl SpiDevice for SpiDeviceImpl {
    fn have_transaction(&self) -> bool {
        self.received_len.get() != 0
    }

    fn wait_for_transaction(&self) {
        while !self.have_transaction() { unsafe { yieldk(); } }
    }

    fn get_read_buffer(&self) -> &[u8] {
        &(self.read_buffer[0..self.received_len.get()])
    }

    fn is_busy_set(&self) -> bool {
        self.is_busy_set.get()
    }

    fn clear_busy(&self) -> TockResult<()> {
        self.received_len.set(0);
        syscalls::command(DRIVER_NUMBER, command_nr::CLEAR_BUSY, 0, 0)?;
        Ok(())
    }

    fn send_data(&self, write_buffer: &mut[u8], clear_busy: bool) -> TockResult<()> {
        self.received_len.set(0);

        // We want this to go out of scope after executing the command
        let _write_buffer_share = syscalls::allow(DRIVER_NUMBER, allow_nr::WRITE_BUFFER, write_buffer)?;

        syscalls::command(DRIVER_NUMBER, command_nr::SEND_DATA, if clear_busy { 1 } else { 0 }, 0)?;

        Ok(())
    }
}
