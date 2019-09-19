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

#[derive(Clone,Copy,Default)]
struct LogEntry {
    /// The value of the operation. u32::MAX indicates this was an erase,
    /// otherwise it was a write.
    value: u32,

    /// The operation's offset. This is in units of words from the start of
    /// flash.
    offset: usize,
}

/// A fake version of H1B's flash modules. Starts initialized with all 1's as if
/// the flash had just been erased. To keep memory usage small, this supports a
/// limited number of operations before panicking.
pub struct FakeHw {
    error: core::cell::Cell<u16>,
    // Currently-executing opcode; 0 if no transaction is ongoing.
    opcode: core::cell::Cell<u32>,

    transaction_offset: core::cell::Cell<usize>,
    transaction_size: core::cell::Cell<usize>,
    write_data: [core::cell::Cell<u32>; 32],

    // Changes that have been successfully applied to the flash. Replayed during
    // simulated reads to determine the value of a cell.
    log: [core::cell::Cell<LogEntry>; 5],
    log_len: core::cell::Cell<usize>,
}

impl FakeHw {
    pub fn new() -> Self {
        Self {
            error:              Default::default(),
            opcode:             Default::default(),
            transaction_offset: Default::default(),
            transaction_size:   Default::default(),
            write_data:         Default::default(),
            log:                Default::default(),
            log_len:            Default::default(),
        }
    }

    /// Simulates the flash module finishing an operation.
    pub fn finish_operation(&self) {
        if self.opcode.get() == super::driver::ERASE_OPCODE {
            // An erase is recorded as a single log entry.
            self.transaction_size.set(1);
        }

        // Check if we will overflow the log. If this will overfill the log then
        // indicate an error.
        if self.log_len.get() + self.transaction_size.get() > self.log.len() {
            // "Program failed" error.
            self.inject_result(0x8);
            return;
        }

        // Attempting to set a 0 bit to a 1 during a write causes the flash
        // module to emit a "program failed" error. To emulate this behavior, we
        // scan backwards through the transaction log until we find an erase,
        // checking if each write is compatible with this new write.
        for entry_cell in self.log[0..self.log_len.get()].iter().rev() {
            let entry = entry_cell.get();

            // Check if it is an erase.
            if entry.value == core::u32::MAX { break; }

            // Check if this log entry is in the current operation's range.
            if entry.offset >= self.transaction_offset.get() &&
               entry.offset < self.transaction_offset.get() + self.transaction_size.get() {
                // It overlaps; check whether this write has a bit set that the
                // previous write did not.
                let new_value =
                    self.write_data[entry.offset - self.transaction_offset.get()].get();
                if new_value & !entry.value != 0 {
                    // This operation tried to flip a bit back to 1, so trigger
                    // an error.
                    self.inject_result(0x8);
                    return;
                }
            }
        }

        for i in 0..self.transaction_size.get() {
            self.log[self.log_len.get()].set(LogEntry {
                value:
                    if self.opcode.get() == super::driver::ERASE_OPCODE {
                        core::u32::MAX
                    } else {
                        self.write_data[i].get()
                    },
                offset: self.transaction_offset.get() + i,
            });
            self.log_len.set(self.log_len.get() + 1);
        }
        self.opcode.set(0);
    }

    /// Injects a smart program result. 0 for a successful validation, nonzero
    /// for an error.
    pub fn inject_result(&self, error: u16) {
        self.error.set(error);
        self.opcode.set(0);
    }
}

impl super::hardware::Hardware for FakeHw {
    fn is_programming(&self) -> bool {
        self.opcode.get() != 0
    }

    fn read(&self, offset: usize) -> kernel::ReturnCode {
        // Replay the operation log in reverse to find the current value.
        for entry in self.log[0..self.log_len.get()].iter().rev() {
            let entry = entry.get();
            if entry.value == core::u32::MAX {
                // Erase
                if offset >= entry.offset && offset < entry.offset + 512 {
                    return kernel::ReturnCode::SuccessWithValue { value: core::u32::MAX as usize };
                }
            } else {
                // Write
                if offset == entry.offset {
                    return kernel::ReturnCode::SuccessWithValue { value: entry.value as usize };
                }
            }
        }

        // Pretend that flash was initialized to all ones.
        kernel::ReturnCode::SuccessWithValue { value: core::u32::MAX as usize }
    }

    fn read_error(&self) -> u16 {
        // The error register is self-clearing.
        let out = self.error.get();
        self.error.set(0);
        out
    }

    fn set_transaction(&self, offset: usize, size: usize) {
        self.transaction_offset.set(offset);
        self.transaction_size.set(size + 1);
    }

    fn set_write_data(&self, data: &[u32]) {
        for (i, &v) in data.iter().enumerate() { self.write_data[i].set(v); }
    }

    fn trigger(&self, opcode: u32) {
        self.opcode.set(opcode);
    }
}
