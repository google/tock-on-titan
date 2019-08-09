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

use core::cell::Cell;
use ::kernel::hil::time::Alarm;
use ::kernel::ReturnCode;
use super::hardware::Hardware;
use super::smart_program::SmartProgramState;

pub const ERASE_OPCODE: u32 = 0x31415927;
pub const WRITE_OPCODE: u32 = 0x27182818;

/// The H1B flash driver. The hardware interface (either the real flash modules
/// or the fake) is injected to support testing. This will not configure the
/// globalsec flash regions -- that must be done independently.
pub struct Flash<'d, A: Alarm + 'd, H: Hardware<'d> + 'd> {
    alarm: &'d A,
    client: Cell<Option<&'d Client>>,

    // Hardware interface. Uses shared references rather than mutable references
    // because the fake interface used in the unit tests is shared with the unit
    // tests.
    hw: &'d H,

    // Smart programming state machine, if an operation is ongoing.
    smart_program_state: Cell<Option<SmartProgramState>>,
    opcode: Cell<u32>,
}

/// A client of the Flash driver -- receives callbacks when flash operations
/// complete.
pub trait Client {
    fn erase_done(&self, ReturnCode);
    fn write_done(&self, ReturnCode);
}

// Public API for Flash.
impl<'d, A: Alarm, H: Hardware<'d>> Flash<'d, A, H> {
    /// Constructs a driver for the given hardware interface. Unsafe because
    /// constructing multiple drivers for the same hardware seems like a bad
    /// idea. The caller must set the driver as the hardware's client before
    /// executing any flash operations.
    pub unsafe fn new(alarm: &'d A, hw: &'d H) -> Self {
        Flash {
            alarm,
            client: Cell::new(None),
            hw,
            smart_program_state: Cell::new(None),
            opcode: Cell::new(0)
        }
    }

    /// Erases the specified flash page, setting it to all ones.
    pub fn erase(&self, page: usize) -> ReturnCode {
        if self.program_in_progress() { return ReturnCode::EBUSY; }
        self.smart_program(ERASE_OPCODE, 45, page * super::WORDS_PER_PAGE, 1);
        ReturnCode::SUCCESS
    }

    /// Reads the given word from flash.
    pub fn read(&self, word: usize) -> u32 {
        self.hw.read(word)
    }

    /// Writes a buffer (of up to 32 words) into the given location in flash.
    /// The target location is specific as an offset from the beginning of flash
    /// in units of words.
    pub fn write(&self, target: usize, data: &[u32]) -> ReturnCode {
        if data.len() > 32 { return ReturnCode::ESIZE; }
        if self.program_in_progress() { return ReturnCode::EBUSY; }

        self.hw.set_write_data(data);
        self.smart_program(WRITE_OPCODE, 9, target, data.len());
        ReturnCode::SUCCESS
    }

    /// Links this driver to its client.
    pub fn set_client(&self, client: &'d Client) {
        self.client.set(Some(client));
    }
}

impl<'d, A: Alarm, H: Hardware<'d>> Flash<'d, A, H> {
    /// Returns true if an operation is in progress and false otherwise.
    fn program_in_progress(&self) -> bool {
        // SmartProgramState is not Copy, so we can't use Cell::get() or
        // Cell::update(). However, Option is Default, so we can swap it out and
        // in. Ideally the optimizer can optimize the moves away entirely.
        let smart_program_state = self.smart_program_state.take();
        let in_progress = smart_program_state.is_some();
        self.smart_program_state.set(smart_program_state);
        in_progress
    }

    /// Begins the smart programming procedure. Note that size must be >= 1 to
    /// avoid underflow (use an arbitrary positive value for erases).
    fn smart_program(&self, opcode: u32, max_attempts: u8, target: usize, size: usize) {
        self.hw.set_transaction(target, size - 1);
        self.smart_program_state.set(Some(
            SmartProgramState::init(max_attempts)
                .step(self.alarm, self.hw, opcode, /*is_timeout:*/ false)));
        self.opcode.set(opcode);
    }

    /// Step the smart program state machine.
    fn step(&self, is_timeout: bool) {
        if let Some(state) = self.smart_program_state.take() {
            let state = state.step(
                self.alarm, self.hw, self.opcode.get(), is_timeout);
            if let Some(code) = state.return_code() {
                if let Some(client) = self.client.get() {
                    if self.opcode.get() == WRITE_OPCODE {
                        client.write_done(code);
                    } else {
                        client.erase_done(code);
                    }
                }
            } else {
                self.smart_program_state.set(Some(state));
            }
        }
    }
}

impl<'d, A: Alarm, H: Hardware<'d>> super::hardware::Client for Flash<'d, A, H> {
    fn interrupt(&self) {
        self.step(false);
    }
}

impl<'d, A: Alarm, H: Hardware<'d>> ::kernel::hil::time::Client for Flash<'d, A, H> {
    fn fired(&self) {
        self.step(true);
    }
}
