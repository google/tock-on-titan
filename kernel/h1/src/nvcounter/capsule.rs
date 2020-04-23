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

use ::kernel::ReturnCode;
use super::internal::*;
use super::traits::{Client,NvCounter};
use crate::hil;

/// NvCounter implementation using flash memory.

// Uses two pages of flash, flipping four bits to zero in each page at a time.
// The low page is struck or erased for every increment operation. When the low
// page wraps around, the high page is struck once, the low page is erased, then
// the high page is struck again. This allows the counter to resume if the
// increment operation is interrupted, providing atomicity.
//
// When initialized, the counter is set to 0. The low page counts 4096 times
// before being reset on the 4097th count. The high page count 2048 times before
// the counter maxes out (because it is struck twice each time the low page
// rolls over). Therefore the maximum counter value is:
// (Counts per low page reset) * (high counts before saturated) + max low count
// 4097 * 2048 + 4096 = 8394752
//
// initialize() sequence:
//   Init1: Erase low
//   Init2: Erase high
//
// Normal increment() sequence:
//   Incr1: Write low
//
// Rollover increment() sequence:
//   Rollover1: Write high (even -> odd)
//   Rollover2: Erase low [in background]
//   Rollover3: Write high (odd -> even) [in background]
//
// Possible values of task, high page count, and low page count when each step
// completes, before the task value is updated:
//   Step       Task        High  Low
//   Init1      Initialize  *     0
//   Init2      Initialize  0     0
//   Incr1      Increment   Even  >0
//   Rollover1  Increment   Odd   Max
//   Rollover2  *           Odd   0
//   Rollover3  *           Even  0

pub struct FlashCounter<'c, F: hil::flash::Flash<'c> + 'c> {
    client: ::core::cell::Cell<Option<&'c dyn Client>>,
    flash: &'c F,
    write_buffer: core::cell::Cell<Option<&'c mut [u32; 1]>>,

    // What operation the client is currently waiting on. Note that when
    // executing steps Rollover2 and Rollover3, this reflects the task the
    // client wants the counter to do *next*.
    task: ::core::cell::Cell<Option<Task>>,
}

impl<'c, F: hil::flash::Flash<'c> + 'c> FlashCounter<'c, F> {
    pub fn new(buffer: &'c mut [u32; 1], flash: &'c F) -> Self {
        FlashCounter {
            client: ::core::cell::Cell::new(None),
            flash,
            write_buffer: core::cell::Cell::new(Some(buffer)),
            task: ::core::cell::Cell::new(None),
        }
    }
}

impl <'c, F: hil::flash::Flash<'c> + 'c> NvCounter<'c> for FlashCounter<'c, F> {
    fn initialize(&self) -> ReturnCode {
        // For now, we only support doing a single operation at a time.
        if self.task.get().is_some() { return ReturnCode::EBUSY; }
        // Try to start the erase (step Init1). If a flash operation is ongoing
        // (which can occur with task == None in states Rollover2 and
        // Rollover3), we will get back EBUSY. In that case, return success, as
        // the erase will begin when the current operation completes. The client
        // will receive a callback when the erase completes.
        match self.flash.erase(Page::Low as usize) {
            ReturnCode::SUCCESS | ReturnCode::EBUSY => {
                self.task.set(Some(Task::Initialize));
                ReturnCode::SUCCESS
            },
            other_code => other_code,
        }
    }

    fn read_and_increment(&self) -> ReturnCode {
        // For now, we only support doing a single operation at a time.
        if self.task.get().is_some() { return ReturnCode::EBUSY; }
        let high_count = read_page_count(Page::High, self.flash);
        let low_count = read_page_count(Page::Low, self.flash);

        // Utility to minimize repetition.
        let success = || {
            self.task.set(Some(Task::Increment));
            ReturnCode::SuccessWithValue {
                value: counter_value(high_count, low_count) as usize
            }
        };

        // Detect the current step. Because we previously confirmed no
        // operations were running, the flash is either idle, running Rollover2,
        // or running Rollover3.
        match (high_count & 1, low_count) {
            (1, 0) => {
                // High is odd and low was erased, so we are either running
                // Rollover3 or need to run Rollover3.
                if let Some(buffer) = self.write_buffer.take() {
                    // Rollover3 is not running.
                    let (code, buffer) = start_increment(
                        Page::High,
                        high_count,
                        self.flash,
                        buffer,
                    );
                    self.write_buffer.set(buffer);
                    return match code {
                        ReturnCode::SUCCESS | ReturnCode::EBUSY => success(),
                        code => code,
                    };
                } else {
                    // Rollover3 is running.
                    return success();
                }
            },
            (1, _) => {
                // We are running or need to run step Rollover2.
                match self.flash.erase(Page::Low as usize) {
                    ReturnCode::SUCCESS | ReturnCode::EBUSY => return success(),
                    error_code => return error_code,
                }
            },
            _ => {
                // If the low page is maxed out, we need to start step
                // Rollover1. Otherwise start step Incr1.
                let (code, buffer) = start_increment(
                    Page::Low,
                    low_count,
                    self.flash,
                    self.write_buffer.take().unwrap()
                );
                self.write_buffer.set(buffer);
                match code {
                    ReturnCode::SUCCESS | ReturnCode::EBUSY => return success(),
                    ReturnCode::ESIZE => {
                        // The low page is maxed out, start step Rollover1.
                        let (return_code, buffer) = start_increment(
                            Page::High, high_count, self.flash, self.write_buffer.take().unwrap());
                        self.write_buffer.set(buffer);
                        match return_code {
                            ReturnCode::SUCCESS | ReturnCode::EBUSY => return success(),
                            error_code => return error_code,
                        }
                    },
                    error_code => return error_code,
                }
            },
        }
    }

    fn set_client(&self, client: &'c dyn Client) {
        self.client.set(Some(client));
    }
}

impl <'c, F: hil::flash::Flash<'c> + 'c> hil::flash::Client<'c> for FlashCounter<'c, F> {
    fn erase_done(&self, code: ReturnCode) {
        // If task is None, then a failure means we have nothing else to do
        // until called again. If task is Initialize, then this failure means
        // the initialization failed. If task is Increment, then this must be a
        // step Rollover2 failure which prevents the increment from working.
        //
        // Therefore, any erase failure ends the current task.
        if code != ReturnCode::SUCCESS {
            // Note the use of .take(): we want the existing value of self.task,
            // but we want to reset it before calling initialize_done in case
            // initialize_done recurses back into FlashCounter.
            match (self.task.take(), self.client.get()) {
                (Some(Task::Initialize), Some(client)) => client.initialize_done(ReturnCode::FAIL),
                (Some(Task::Increment), Some(client)) => client.increment_done(ReturnCode::FAIL),
                _ => {},
            }
            return;
        }

        // The erase steps are Init1, Init2, and Rollover2. At the end of these
        // steps, the low page is always fully erased. Therefore, if an
        // initialization was requested, we only need to do Init2 or call the
        // callback.
        if self.task.get() == Some(Task::Initialize) {
            if page_empty(Page::High, self.flash) {
                // Initialization is done.
                self.task.set(None);
                if let Some(client) = self.client.get() {
                    client.initialize_done(ReturnCode::SUCCESS);
                }
                return;
            }

            match self.flash.erase(Page::High as usize) {
                ReturnCode::SUCCESS => return,
                error => {
                    self.task.set(None);
                    if let Some(client) = self.client.get() {
                        client.initialize_done(error);
                    }
                },
            }

            return;
        }

        // Step Rollover2 finished and we need to run step Rollover3.
        let (_, buffer) = start_increment(
            Page::High,
            read_page_count(Page::High, self.flash),
            self.flash,
            self.write_buffer.take().unwrap()
        );
        if let Some(returned_buffer) = buffer {
            self.write_buffer.set(Some(returned_buffer));
            if self.task.take() == Some(Task::Increment) {
                if let Some(client) = self.client.get() {
                    client.increment_done(ReturnCode::FAIL);
                }
            }
        }
    }

    fn write_done(&self, data: &'c mut [u32], code: ReturnCode) {
        use core::convert::TryInto;
        self.write_buffer.set(Some(data.try_into().unwrap()));

        // The writes are steps Incr1, Rollover1, and Rollover3. If the current
        // task is increment, then this write was necessary; signal failure.
        if code != ReturnCode::SUCCESS && self.task.get() == Some(Task::Increment) {
            self.task.set(None);
            if let Some(client) = self.client.get() {
                client.increment_done(code);
            }
            return;
        }

        // Detect whether we just finished step Rollover3 with nothing further
        // to do.
        if self.task.get().is_none() { return; }

        // If we are being asked to initialize, jump to step Init1. This can
        // only happen from step Rollover3, but that isn't important here.
        if self.task.get() == Some(Task::Initialize) {
            match self.flash.erase(Page::Low as usize * WORDS_PER_PAGE) {
                ReturnCode::SUCCESS => return,
                error => {
                    self.task.set(None);
                    if let Some(client) = self.client.get() {
                        client.initialize_done(error);
                    }
                },
            }
            return;
        }

        // At this point, the task is increment. After steps Rollover1 and
        // Incr1, the low page will always have a nonzero count, so we can check
        // if this is step Rollover3 by looking at the low page of flash.
        if page_empty(Page::Low, self.flash) {
            // Step Rollover3 with a further increment requested, perform step
            // Incr1.
            let (increment_code, buffer) = start_increment(
                Page::Low,
                read_page_count(Page::Low, self.flash),
                self.flash,
                self.write_buffer.take().unwrap(),
            );
            self.write_buffer.set(buffer);
            if increment_code != ReturnCode::SUCCESS {
                self.task.set(None);
                if let Some(client) = self.client.get() {
                    client.increment_done(increment_code);
                }
            }
            return;
        }

        // At this point, either step Incr1 or Rollover1 finished successfully.
        self.task.set(None);

        // If the step that finished was step Rollover1, we need to perform step
        // Rollover2.
        if low_page_full(self.flash) && read_page_count(Page::High, self.flash) & 1 != 0 {
            // Rollover1 just finished, start step Rollover2.
            self.flash.erase(Page::Low as usize);
        }

        // Call the client last, in case it calls back into the counter capsule.
        if let Some(client) = self.client.get() {
            client.increment_done(ReturnCode::SUCCESS);
        }
    }
}
