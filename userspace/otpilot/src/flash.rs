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

use core::cell::Cell;

use libtock::result::TockResult;
use libtock::syscalls;
use libtock::syscalls::raw::yieldk;

pub const MAX_BUFFER_LENGTH: usize = 128;

pub trait Flash {
    // Read from flash.
    // offset: Location relative to flash start to read from. Must be word (4 bytes) aligned.
    // buffer: Buffer for read data.
    //         It must be: len <= buffer.len() <= MAX_BUFFER_LENGTH
    // len: Number of bytes to read. Must be > 0 and a multiple of 4.
    fn read(&self, offset: usize, buffer: &mut[u8], len: usize) -> TockResult<()>;

    // Write to flash.
    // offset: Location relative to flash start to write to. Must be word (4 bytes) aligned.
    // buffer: Buffer with data to write.
    //         It must be: buffer.len() <= len <= MAX_BUFFER_LENGTH
    // len: Number of bytes to write. Must be > 0 and a multiple of 4.
    fn write(&self, offset: usize, buffer: &mut[u8], len: usize) -> TockResult<()>;

    // Erase page in flash.
    // page: Number of page to erase.
    fn erase(&self, page: usize) -> TockResult<()>;

    // Returns true if the last operation is done.
    fn is_operation_done(&self) -> bool;

    // Wait (yieldk) until the operation is done.
    fn wait_operation_done(&self);

    // Get the result of the last operation.
    fn get_operation_result(&self) -> isize;

    fn clear_operation(&self);
}

// Get the static Flash object.
pub fn get() -> &'static dyn Flash {
    get_impl()
}

const DRIVER_NUMBER: usize = 0x40040;

mod command_nr {
    pub const CHECK_IF_PRESENT: usize = 0;
    pub const ERASE_PAGE: usize = 1;
    pub const WRITE_DATA: usize = 2;
    pub const READ_DATA: usize = 3;
}

mod subscribe_nr {
    pub const OPERATION_COMPLETE: usize = 0;
}

mod allow_nr {
    pub const WRITE_BUFFER: usize = 0;
    pub const READ_BUFFER: usize = 1;
}

struct FlashImpl {
    // The result of the last operation.
    operation_result: Cell<isize>,

    // Whether the operation is complete.
    operation_done: Cell<bool>,
}

static mut FLASH: FlashImpl = FlashImpl {
    operation_result: Cell::new(-1),
    operation_done: Cell::new(false),
};

static mut IS_INITIALIZED: bool = false;

fn get_impl() -> &'static FlashImpl {
    unsafe {
        if !IS_INITIALIZED {
            if FLASH.initialize().is_err() {
                panic!("Could not initialize Flash");
            }
            IS_INITIALIZED = true;
        }
        &FLASH
    }
}

impl FlashImpl {
    fn initialize(&'static mut self) -> TockResult<()> {
        syscalls::command(DRIVER_NUMBER, command_nr::CHECK_IF_PRESENT, 0, 0)?;

        syscalls::subscribe_fn(
            DRIVER_NUMBER,
            subscribe_nr::OPERATION_COMPLETE,
            FlashImpl::operation_done_trampoline,
            0)?;

        Ok(())
    }

    extern "C"
    fn operation_done_trampoline(arg1: usize, arg2: usize, arg3: usize, _data: usize) {
        get_impl().operation_done(arg1, arg2, arg3);
    }

    fn operation_done(&self, result: usize, _: usize, _: usize) {
        self.operation_result.set(result as isize);
        self.operation_done.set(true);
    }
}

impl Flash for FlashImpl {
    fn read(&self, offset: usize, buffer: &mut[u8], len: usize) ->  TockResult<()> {
        // We want this to go out of scope after executing the command
        let _buffer_share = syscalls::allow(DRIVER_NUMBER, allow_nr::READ_BUFFER, buffer)?;

        syscalls::command(DRIVER_NUMBER, command_nr::READ_DATA, offset, len)?;

        Ok(())
    }

    fn write(&self, offset: usize, buffer: &mut[u8], len: usize) ->  TockResult<()> {
        // We want this to go out of scope after executing the command
        let _buffer_share = syscalls::allow(DRIVER_NUMBER, allow_nr::WRITE_BUFFER, buffer)?;

        self.operation_result.set(-1);
        self.operation_done.set(false);
        syscalls::command(DRIVER_NUMBER, command_nr::WRITE_DATA, offset, len)?;

        Ok(())
    }

    fn erase(&self, page: usize) ->  TockResult<()> {
        self.operation_result.set(-1);
        self.operation_done.set(false);
        syscalls::command(DRIVER_NUMBER, command_nr::ERASE_PAGE, page, 0)?;

        Ok(())
    }

    fn is_operation_done(&self) -> bool {
        self.operation_done.get()
    }

    fn wait_operation_done(&self) {
        while !self.is_operation_done() { unsafe { yieldk(); } }
    }

    fn get_operation_result(&self) -> isize {
        self.operation_result.get()
    }

    fn clear_operation(&self) {
        self.operation_done.set(false);
    }
}
