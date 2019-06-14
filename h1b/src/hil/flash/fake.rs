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
pub struct FakeHw<'a> {
    client: core::cell::Cell<Option<&'a super::hardware::Client>>,
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

impl<'a> FakeHw<'a> {
    pub fn new() -> Self {
        Self {
            client:             Default::default(),
            error:              Default::default(),
            opcode:             Default::default(),
            transaction_offset: Default::default(),
            transaction_size:   Default::default(),
            write_data:         Default::default(),
            log:                Default::default(),
            log_len:            Default::default(),
        }
    }

    /// Simulates the flash module successfully finishing an operation.
    pub fn finish_operation(&self) {
        if self.opcode.get() == super::driver::ERASE_OPCODE {
            // An erase is recorded as a single log entry.
            self.transaction_size.set(1);
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
        if let Some(client) = self.client.get() { client.interrupt(); }
    }

    /// Simulates a flash error.
    pub fn inject_error(&self, error: u16) {
        self.error.set(error);
        self.opcode.set(0);
        if let Some(client) = self.client.get() { client.interrupt(); }
    }
}

impl<'a> super::hardware::Hardware<'a> for FakeHw<'a> {
    fn is_programming(&self) -> bool {
        self.opcode.get() != 0
    }

    fn read(&self, offset: usize) -> u32 {
        // Pretend that flash was initialized to all ones.
        let mut value = 0xFFFFFFFF;
        // Replay the operation log to find the current value.
        for entry in self.log[0..self.log_len.get()].iter().map(|e| e.get()) {
            if entry.value == core::u32::MAX {
                // Erase
                if offset >= entry.offset && offset < entry.offset + 512 {
                    value = core::u32::MAX;
                }
            } else {
                // Write
                if offset == entry.offset {
                    value &= entry.value;
                }
            }
        }
        value
    }

    fn read_error(&self) -> u16 {
        // The error register is self-clearing.
        let out = self.error.get();
        self.error.set(0);
        out
    }

    fn set_client(&self, client: &'a super::hardware::Client) {
        self.client.set(Some(client));
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
