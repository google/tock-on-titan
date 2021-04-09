// Copyright 2021 lowRISC contributors.
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

use core::cmp::min;
use core::cell::Cell;

use libtock::result::TockResult;
use libtock::shared_memory::SharedMemory;
use libtock::syscalls;

pub const MAX_READ_BUFFER_SIZE: usize = 512;

pub trait ConsoleReader {
    fn allow_read(&'static mut self, len: usize) -> TockResult<()>;
    fn abort_read(&self) -> TockResult<()>;
    fn have_data(&self) -> bool;
    fn get_data(&self) -> &[u8];
}

// Get the static ConsoleReader object.
pub fn get() -> &'static mut dyn ConsoleReader {
    get_impl()
}

const DRIVER_NUMBER: usize = 1;

mod command_nr {
    pub const CHECK_IF_PRESENT: usize = 0;
    pub const READ: usize = 2;
    pub const ABORT_READ: usize = 3;
}

mod subscribe_nr {
    pub const READ_DONE: usize = 2;
}

mod allow_nr {
    pub const READ_BUFFER: usize = 2;
}

pub struct ConsoleReaderImpl {
    /// The receive buffer.
    read_buffer: [u8; MAX_READ_BUFFER_SIZE],

    /// Shared memory object to allow kernel to access read_buffer.
    read_buffer_share: Cell<Option<SharedMemory<'static>>>,

    /// Number of received bytes.
    received_len: Cell<usize>,
}

static mut CONSOLE_READER: ConsoleReaderImpl = ConsoleReaderImpl {
    read_buffer: [0; MAX_READ_BUFFER_SIZE],
    read_buffer_share: Cell::new(None),
    received_len: Cell::new(0),
};

static mut IS_INITIALIZED: bool = false;

fn get_impl() -> &'static mut ConsoleReaderImpl {
    unsafe {
        if !IS_INITIALIZED {
            if CONSOLE_READER.initialize().is_err() {
                panic!("Could not initialize Console Reader");
            }
            IS_INITIALIZED = true;
        }
        &mut CONSOLE_READER
    }
}

impl ConsoleReaderImpl {
    fn initialize(&'static mut self) -> TockResult<()> {
        syscalls::command(DRIVER_NUMBER, command_nr::CHECK_IF_PRESENT, 0, 0)?;

        syscalls::subscribe_fn(
            DRIVER_NUMBER,
            subscribe_nr::READ_DONE,
            ConsoleReaderImpl::read_done_trampoline,
            0)?;

        Ok(())
    }

    extern "C"
    fn read_done_trampoline(arg1: usize, arg2: usize, arg3: usize, _data: usize) {
        get_impl().read_done(arg1, arg2, arg3);
    }

    fn read_done(&self, _arg1: usize, arg2: usize, _: usize) {
        // arg1: return code
        // arg2: number of read bytes
        self.received_len.set(arg2);
    }
}


impl ConsoleReader for ConsoleReaderImpl {
    fn allow_read(&'static mut self, len: usize) -> TockResult<()> {
        self.read_buffer_share.set(None);
        self.received_len.set(0);

        let read_len = min(self.read_buffer.len(), len);
        self.read_buffer_share.set(Some(syscalls::allow(DRIVER_NUMBER, allow_nr::READ_BUFFER,
            &mut self.read_buffer)?));
        syscalls::command(DRIVER_NUMBER, command_nr::READ, read_len, 0)?;

        Ok(())
    }

    fn abort_read(&self) -> TockResult<()> {
        syscalls::command(DRIVER_NUMBER, command_nr::ABORT_READ, 0, 0)?;

        Ok(())
    }

    fn have_data(&self) -> bool {
        self.received_len.get() > 0
    }

    fn get_data(&self) -> &[u8] {
        &self.read_buffer[0..self.received_len.get()]
    }
}
