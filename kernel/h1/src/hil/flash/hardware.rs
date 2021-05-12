// Copyright 2019 Google LLC
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

use kernel::ReturnCode;

/// The bank to perform an operation on.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Bank {
    Unknown,
    Zero,
    One,
}

impl Default for Bank {
    fn default() -> Self { Bank::Unknown }
}

/// The interface between the flash driver and the (real or fake) flash module.
pub trait Hardware {
    /// Returns true if an operation is running, false otherwise.
    fn is_programming(&self) -> bool;

    /// Read a single word from the flash (non-blocking). offset is in units of
    /// words and is relative to the start of flash.
    fn read(&self, offset: usize) -> ReturnCode;

    /// Reads the flash error code.
    fn read_error(&self) -> u16;

    /// Set flash transaction parameters (word offset and size). The word offset
    /// is relative to the start of a flash bank and the size is one less than the
    /// number of words to copy.
    fn set_transaction(&self, bank_offset: usize, size: usize);

    /// Fill the flash controller's write buffer. data must have a length no
    /// larger than 32.
    fn set_write_data(&self, data: &[u32]);

    /// Kick off a smart program execution on the specified `Bank` using the data
    /// configured via `set_transaction`.
    fn trigger(&self, opcode: u32, bank: Bank);
}
