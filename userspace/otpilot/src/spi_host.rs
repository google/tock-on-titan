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

pub const MAX_READ_BUFFER_LENGTH: usize = 128;

pub trait SpiHost {
    // Start a new transaction.
    // write_buffer: Data to send. It must be: write_buffer.len() >= read_write_length.
    // read_write_length: Number of bytes in transaction. Must be > 0.
    fn read_write_bytes(&self, write_buffer: &mut[u8], read_write_length: usize) -> TockResult<()>;

    // Check if the last read_write is done.
    fn is_read_write_done(&self) -> bool;

    // Wait for the last read_write to complete by yielding.
    fn wait_read_write_done(&self);

    // Get the read buffer slice.
    fn get_read_buffer(&self) -> &[u8];
}

// Get the static SpiHost object.
pub fn get() -> &'static dyn SpiHost {
    get_impl()
}

const DRIVER_NUMBER: usize = 0x20001;

mod command_nr {
    pub const CHECK_IF_PRESENT: usize = 0;
    pub const READ_WRITE_BYTES: usize = 2;
}

mod subscribe_nr {
    pub const READ_WRITE_COMPLETE: usize = 0;
}

mod allow_nr {
    pub const READ_BUFFER: usize = 0;
    pub const WRITE_BUFFER: usize = 1;
}

struct SpiHostImpl {
    // The receive buffer. Should be equal or larger than HW buffer.
    read_buffer: [u8; MAX_READ_BUFFER_LENGTH],

    // Shared memory object to allow kernel to access read_buffer.
    read_buffer_share: Cell<Option<SharedMemory<'static>>>,

    // Requested length for running transaction.
    read_write_length: Cell<usize>,

    // Whether the transaction is complete.
    read_write_done: Cell<bool>,
}

static mut SPI_HOST: SpiHostImpl = SpiHostImpl {
    read_buffer: [0; MAX_READ_BUFFER_LENGTH],
    read_buffer_share: Cell::new(None),
    read_write_length: Cell::new(0),
    read_write_done: Cell::new(false),
};

static mut IS_INITIALIZED: bool = false;

fn get_impl() -> &'static SpiHostImpl {
    unsafe {
        if !IS_INITIALIZED {
            if SPI_HOST.initialize().is_err() {
                panic!("Could not initialize SPI Host");
            }
            IS_INITIALIZED = true;
        }
        &SPI_HOST
    }
}

impl SpiHostImpl {
    fn initialize(&'static mut self) -> TockResult<()> {
        syscalls::command(DRIVER_NUMBER, command_nr::CHECK_IF_PRESENT, 0, 0)?;
        self.read_buffer_share.set(Some(syscalls::allow(DRIVER_NUMBER, allow_nr::READ_BUFFER,
            &mut self.read_buffer)?));

        Ok(())
    }

    fn register_read_write_done_callback(&self) -> TockResult<()> {
        syscalls::subscribe_fn(
            DRIVER_NUMBER,
            subscribe_nr::READ_WRITE_COMPLETE,
            SpiHostImpl::read_write_done_trampoline,
            0)?;

        Ok(())
    }

    extern "C"
    fn read_write_done_trampoline(arg1: usize, arg2: usize, arg3: usize, _data: usize) {
        get_impl().read_write_done(arg1, arg2, arg3);
    }

    fn read_write_done(&self, _: usize, _: usize, _: usize) {
        self.read_write_done.set(true);
    }
}

impl SpiHost for SpiHostImpl {
    // Start a new transaction.
    // write_buffer: Data to send. It must be: write_buffer.len() >= read_write_length.
    // read_write_length: Number of bytes in transaction. Must be > 0.
    fn read_write_bytes(&self, write_buffer: &mut[u8], read_write_length: usize) ->  TockResult<()> {
        // We want this to go out of scope after executing the command
        let _write_buffer_share = syscalls::allow(DRIVER_NUMBER, allow_nr::WRITE_BUFFER, write_buffer)?;

        self.read_write_length.set(read_write_length);
        self.read_write_done.set(false);

        // We need to re-register before each read_write_bytes command.
        self.register_read_write_done_callback()?;

        syscalls::command(DRIVER_NUMBER, command_nr::READ_WRITE_BYTES, self.read_write_length.get(), 0)?;

        Ok(())
    }

    // Check if the last transaction is done.
    fn is_read_write_done(&self) -> bool {
        self.read_write_done.get()
    }

    // Wait for the last transaction to complete. Runs yieldk().
    fn wait_read_write_done(&self) {
        while !self.is_read_write_done() { unsafe { yieldk(); } }
    }

    // Get the receive buffer slice for the last transaction.
    fn get_read_buffer(&self) -> &[u8] {
        &(self.read_buffer[0..self.read_write_length.get()])
    }
}
