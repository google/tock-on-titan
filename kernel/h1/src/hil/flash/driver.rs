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
use core::cmp;

use ::kernel::common::cells::TakeCell;
use ::kernel::hil::time::Alarm;
use ::kernel::ReturnCode;
use super::hardware::Bank;
use super::hardware::Hardware;
use super::smart_program::SmartProgramState;

/// The H1 flash driver. The hardware interface (either the real flash modules
/// or the fake) is injected to support testing. This will not configure the
/// globalsec flash regions -- that must be done independently.
pub struct FlashImpl<'d, A: Alarm<'d> + 'd, H: Hardware + 'd> {
    alarm: &'d A,
    client: Cell<Option<&'d dyn super::flash::Client<'d>>>,
    write_data: TakeCell<'d, [u32]>,
    write_pos: Cell<usize>,
    write_len: Cell<usize>,
    write_bank: Cell<Bank>,
    // Target address within the bank.
    write_bank_target: Cell<usize>,
    // Hardware interface. Uses shared references rather than mutable references
    // because the fake interface used in the unit tests is shared with the unit
    // tests.
    hw: &'d H,

    // Smart programming state machine, if an operation is ongoing.
    smart_program_state: Cell<Option<SmartProgramState>>,
    opcode: Cell<u32>,
}

// Public API for FlashImpl.
impl<'d, A: Alarm<'d>, H: Hardware> FlashImpl<'d, A, H> {
    /// Constructs a driver for the given hardware interface. Unsafe because
    /// constructing multiple drivers for the same hardware seems like a bad
    /// idea. The caller must set the driver as the hardware's client before
    /// executing any flash operations.
    pub unsafe fn new(alarm: &'d A, hw: &'d H) -> Self {
        FlashImpl {
            alarm,
            client: Cell::new(None),
            write_data: TakeCell::empty(),
            write_pos: Cell::new(0),
            write_len: Cell::new(0),
            write_bank: Cell::new(Bank::Zero),
            write_bank_target: Cell::new(0),
            hw,
            smart_program_state: Cell::new(None),
            opcode: Cell::new(0)
        }
    }
}

const MAX_WRITE_SIZE: usize = 32; // Maximum single write is 32 words
const WORDS_PER_BANK: usize = 0x10000; // 64ki words per bank

// Computes the flash Bank for the specified target location in words
// from the beginning of flash.
fn get_bank_from_target(target: usize) -> Option<Bank> {
    if target < WORDS_PER_BANK {
        Some(Bank::Zero)
    } else if target < 2 * WORDS_PER_BANK {
        Some(Bank::One)
    } else {
        None
    }
}

impl<'d, A: Alarm<'d>, H: Hardware> super::flash::Flash<'d> for FlashImpl<'d, A, H> {
    fn erase(&self, page: usize) -> ReturnCode {
        if self.program_in_progress() { return ReturnCode::EBUSY; }
        let target: usize = page * super::WORDS_PER_PAGE;

        let maybe_bank = get_bank_from_target(target);
        if maybe_bank.is_none() {
            return ReturnCode::EINVAL;
        }

        self.write_bank.set(maybe_bank.unwrap());
        self.write_bank_target.set(target % WORDS_PER_BANK);
        self.smart_program(ERASE_OPCODE, /*max_attempts*/ 45, /*final_pulse_needed*/ false,
                           /*timeout_nanoseconds*/ 3_353_267, self.write_bank.get(),
                           /*bank_target*/ self.write_bank_target.get(), /*size*/ 1);

        ReturnCode::SUCCESS
    }

    fn read(&self, word: usize) -> ReturnCode {
        self.hw.read(word)
    }

    fn write(&self, target: usize, data: &'d mut [u32]) -> (ReturnCode, Option<&'d mut [u32]>) {
        let write_len = cmp::min(data.len(), MAX_WRITE_SIZE);

        if data.len() > MAX_WRITE_SIZE { return (ReturnCode::ESIZE, Some(data)); }
        if self.program_in_progress() { return (ReturnCode::EBUSY, Some(data)); }

        let maybe_bank = get_bank_from_target(target);
        if maybe_bank.is_none() {
            return (ReturnCode::EINVAL, Some(data));
        }

        self.write_pos.set(0);
        self.write_bank.set(maybe_bank.unwrap());
        self.write_bank_target.set(target % WORDS_PER_BANK);
        self.write_len.set(write_len);
        self.hw.set_write_data(&data[0..write_len]);
        self.write_data.replace(data);

        self.smart_program(WRITE_OPCODE, /*max_attempts*/ 8, /*final_pulse_needed*/ true,
                           /*timeout_nanoseconds*/ 48734 + write_len as u32 * 3734,
                           self.write_bank.get(), self.write_bank_target.get(), write_len);

        (ReturnCode::SUCCESS, None)
    }

    fn set_client(&'d self, client: &'d dyn super::flash::Client<'d>) {
        self.client.set(Some(client));
    }
}

// -----------------------------------------------------------------------------
// Implementation details below.
// -----------------------------------------------------------------------------

pub const ERASE_OPCODE: u32 = 0x31415927;
pub const WRITE_OPCODE: u32 = 0x27182818;

impl<'d, A: Alarm<'d>, H: Hardware> ::kernel::hil::time::AlarmClient for FlashImpl<'d, A, H> {
    fn alarm(&self) {
        if let Some(state) = self.smart_program_state.take() {
            let state = state.step(
                self.alarm, self.hw, self.opcode.get(), self.write_bank.get());
            if let Some(code) = state.return_code() {
                if let Some(client) = self.client.get() {
                    if self.opcode.get() == WRITE_OPCODE {
                        let subwrite_end = self.write_pos.get() + self.write_len.get();
                        let fullwrite_end = self.write_data.map_or(0, |d| d.len());
                        if subwrite_end >= fullwrite_end || code != ReturnCode::SUCCESS {
                            client.write_done(self.write_data.take().unwrap(),
                                              code);
                        } else {
                            let next_len = cmp::min(MAX_WRITE_SIZE, fullwrite_end - subwrite_end);
                            let next_end = subwrite_end + next_len;
                            let target = self.write_bank_target.get() + subwrite_end;
                            self.write_pos.set(subwrite_end);
                            self.write_data.map(|d|
                                                self.hw.set_write_data(&d[subwrite_end..next_end]));
                            self.smart_program(WRITE_OPCODE, /*max_attempts*/ 8, /*final_pulse_needed*/ true,
                                               /*timeout_nanoseconds*/ 48734 + next_len as u32 * 3734,
                                               self.write_bank.get(), target, next_len);
                        }
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

impl<'d, A: Alarm<'d>, H: Hardware> FlashImpl<'d, A, H> {
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
    /// `bank_target` specifies the target address relative to the selected bank.
    fn smart_program(&self, opcode: u32, max_attempts: u8, final_pulse_needed: bool,
                     timeout_nanoseconds: u32, bank: Bank, bank_target: usize, size: usize)
    {
        // Use the offset relative to the flash bank.
        self.hw.set_transaction(bank_target, size - 1);
        self.smart_program_state.set(Some(
            SmartProgramState::init(max_attempts, final_pulse_needed, timeout_nanoseconds)
                .step(self.alarm, self.hw, opcode, bank)));
        self.opcode.set(opcode);
    }
}
