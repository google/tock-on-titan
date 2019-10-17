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

/// A fake h1b::hil::flash::Flash implementation with enough functionality to
/// test the NvCounter capsule. Unlike the fake flash module in h1b, this
/// implements the Flash HIL (rather than the Hardware trait), only supports the
/// NvCounter pages, and uses run-length encoding so it can support the
/// NvCounter's write patterns using a reasonable amount of stack space.

pub struct FakeFlash<'c> {
    buffer: core::cell::Cell<Option<&'c mut [u32]>>,
    high_page: FakePage,
    low_page: FakePage,
    error_time: core::cell::Cell<Option<ErrorTime>>,
}

impl<'c> FakeFlash<'c> {
    pub fn new() -> FakeFlash<'c> {
        FakeFlash {
            buffer: Default::default(),
            high_page: FakePage::new(),
            low_page: FakePage::new(),
            error_time: Default::default(),
        }
    }

    pub fn configure_error(&self, error_config: Option<ErrorTime>) {
        self.error_time.set(error_config);
    }

    pub fn retrieve_buffer(&self) -> Option<&'c mut [u32]> {
        self.buffer.take()
    }
}

impl<'c> h1b::hil::flash::Flash<'c> for FakeFlash<'c> {
    fn erase(&self, page: usize) -> ReturnCode {
        if let Some(error_time) = self.error_time.get() {
            return start_return_code(error_time);
        }
        match page {
            254 => self.high_page.erase(),
            255 => self.low_page.erase(),
            _ => ReturnCode::FAIL,
        }
    }

    fn read(&self, offset: usize) -> ReturnCode {
        // We ignore error_time here because Flash::read() only fails if offset
        // is out of range. This makes it easier for tests to simulate write()
        // errors realistically.
        match offset_to_page(offset) {
            None => ReturnCode::ESIZE,
            Some(Page::High) => ReturnCode::SuccessWithValue {
                value: self.high_page.read(offset - HIGH_PAGE_START) as usize,
            },
            Some(Page::Low) => ReturnCode::SuccessWithValue {
                value: self.low_page.read(offset - LOW_PAGE_START) as usize,
            },
        }
    }

    fn write(&self, target: usize, data: &'c mut [u32]) -> (ReturnCode, Option<&'c mut [u32]>) {
        if let Some(error_time) = self.error_time.get() {
            return match error_time {
                ErrorTime::Fast => (kernel::ReturnCode::FAIL, Some(data)),
                ErrorTime::Callback => {
                    self.buffer.set(Some(data));
                    (kernel::ReturnCode::SUCCESS, None)
                },
            };
        }
        // Note: this will fail if the write crosses pages, which is fine for
        // this use case. That may be true of the real flash anyway.
        match offset_to_page(target) {
            None => return (ReturnCode::ESIZE, Some(data)),
            Some(Page::High) => self.high_page.write(target - HIGH_PAGE_START, data),
            Some(Page::Low) => self.low_page.write(target - LOW_PAGE_START, data),
        }
        self.buffer.set(Some(data));
        (ReturnCode::SUCCESS, None)
    }

    // No-op -- the tests call erase_done and write_done directly.
    fn set_client(&self, _client: &'c dyn h1b::hil::flash::Client<'c>) {}
}

#[test]
fn test_fake_flash() -> bool {
    use h1b::hil::flash::Flash;
    use kernel::ReturnCode::{FAIL,SUCCESS,SuccessWithValue};
    let flash = FakeFlash::new();
    require!(flash.erase(254) == SUCCESS);
    require!(flash.erase(255) == SUCCESS);
    require!(flash.read(HIGH_PAGE_START) == SuccessWithValue { value: 0xFFFFFFFF });
    require!(flash.read(LOW_PAGE_START + 511) == SuccessWithValue { value: 0xFFFFFFFF });
    let mut buffer = [0];
    require!(flash.write(HIGH_PAGE_START, &mut buffer) == (SUCCESS, None));
    require!(flash.read(HIGH_PAGE_START) == SuccessWithValue { value: 0 });
    require!(flash.read(HIGH_PAGE_START + 1) == SuccessWithValue { value: 0xFFFFFFFF });

    flash.configure_error(Some(ErrorTime::Fast));
    require!(flash.erase(254) == FAIL);
    require!(flash.erase(255) == FAIL);
    require!(flash.read(HIGH_PAGE_START) == SuccessWithValue { value: 0 });
    require!(flash.read(LOW_PAGE_START + 511) == SuccessWithValue { value: 0xFFFFFFFF });
    let mut buffer = [0];
    require!(flash.write(HIGH_PAGE_START, &mut buffer).0 == FAIL);
    require!(flash.read(HIGH_PAGE_START) == SuccessWithValue { value: 0 });
    require!(flash.read(HIGH_PAGE_START + 1) == SuccessWithValue { value: 0xFFFFFFFF });

    flash.configure_error(Some(ErrorTime::Callback));
    require!(flash.erase(254) == SUCCESS);
    require!(flash.erase(255) == SUCCESS);
    require!(flash.read(HIGH_PAGE_START) == SuccessWithValue { value: 0 });
    require!(flash.read(LOW_PAGE_START + 511) == SuccessWithValue { value: 0xFFFFFFFF });
    let mut buffer = [3];
    require!(flash.write(HIGH_PAGE_START, &mut buffer) == (SUCCESS, None));
    require!(flash.read(HIGH_PAGE_START) == SuccessWithValue { value: 0 });
    require!(flash.read(HIGH_PAGE_START + 1) == SuccessWithValue { value: 0xFFFFFFFF });

    true
}

// -----------------------------------------------------------------------------
// Implementation details below
// -----------------------------------------------------------------------------

use h1b::nvcounter::internal::{Page, WORDS_PER_PAGE};
use kernel::ReturnCode;
use test::require;

pub const HIGH_PAGE_START: usize = WORDS_PER_PAGE * Page::High as usize;
pub const LOW_PAGE_START: usize = WORDS_PER_PAGE * Page::Low as usize;
const NUM_RUNS: usize = 5;

fn offset_to_page(offset: usize) -> Option<Page> {
    if offset < HIGH_PAGE_START { return None; }
    if offset < LOW_PAGE_START { return Some(Page::High); }
    if offset < WORDS_PER_PAGE * (1 + Page::Low as usize) {
        return Some(Page::Low);
    }
    None
}

#[test]
fn test_offset_to_page() -> bool {
    // Last word before high page
    require!(offset_to_page(254 * 512 - 1) == None);
    // First word of high page
    require!(offset_to_page(254 * 512) == Some(Page::High));
    // Last word of high page
    require!(offset_to_page(255 * 512 - 1) == Some(Page::High));
    // First word of low page
    require!(offset_to_page(255 * 512) == Some(Page::Low));
    // Last word of flash
    require!(offset_to_page(256 * 512 - 1) == Some(Page::Low));
    // One beyond the end of flash
    require!(offset_to_page(256 * 512) == None);
    // Overflow check
    require!(offset_to_page(usize::max_value()) == None);
    true
}

#[derive(Clone,Copy,PartialEq)]
pub enum ErrorTime {
    Fast,      // Writes and erases fail to start.
    Callback,  // Writes and erases fail asynchronously.
}

// Returns the return code for attempting to start an action.
fn start_return_code(error_time: ErrorTime) -> kernel::ReturnCode {
    match error_time {
        ErrorTime::Fast => kernel::ReturnCode::FAIL,
        ErrorTime::Callback => kernel::ReturnCode::SUCCESS,
    }
}

struct FakePage {
    // Run length and values.
    lens: core::cell::Cell<[u16; NUM_RUNS]>,
    values: core::cell::Cell<[u32; NUM_RUNS]>,
}

impl FakePage {
    pub fn new() -> FakePage {
        FakePage {
            lens: core::cell::Cell::new([WORDS_PER_PAGE as u16, 0, 0, 0, 0]),
            values: core::cell::Cell::new([0xFFFFFFFF; NUM_RUNS]),
        }
    }

    pub fn erase(&self) -> ReturnCode {
        self.lens.set([WORDS_PER_PAGE as u16, 0, 0, 0, 0]);
        self.values.set([0xFFFFFFFF; NUM_RUNS]);
        kernel::ReturnCode::SUCCESS
    }

    // Performs a read of this page. offset is in words, relative to the start
    // of this page.
    pub fn read(&self, offset: usize) -> u32 {
        let mut start = 0;
        let lens = self.lens.get();
        for i in 0..NUM_RUNS {
            // Points one past the end of the current run, so that this run's
            // indices are [start, end).
            let end = start + lens[i] as usize;
            if end > offset { return self.values.get()[i]; }
            start = end;
        }
        // It should not get to here.
        0
    }

    fn write(&self, offset: usize, data: &[u32]) {
        let mut cur_run = 0;
        let mut start = 0;
        let mut builder = RleBuilder::new();
        for i in 0..WORDS_PER_PAGE {
            if i >= offset && i < offset + data.len() {
                builder.append(data[i - offset]);
                continue;
            }
            // Advance the run until we see a run containing index i.
            while start + self.lens.get()[cur_run] as usize <= i {
                start += self.lens.get()[cur_run] as usize;
                cur_run += 1;
            }
            builder.append(self.values.get()[cur_run]);
        }
        let (lens, values) = builder.build();
        self.lens.set(lens);
        self.values.set(values);
    }
}

#[test]
fn test_fake_page() -> bool {
    let page = FakePage::new();
    page.erase();
    require!(page.read(0) == 0xFFFFFFFF);
    require!(page.read(123) == 0xFFFFFFFF);
    require!(page.read(511) == 0xFFFFFFFF);
    page.write(0, &[0x3CFFFFFF]);
    require!(page.read(0) == 0x3CFFFFFF);
    require!(page.read(1) == 0xFFFFFFFF);
    page.write(0, &[0x00FFFFFF]);
    require!(page.read(0) == 0x00FFFFFF);
    require!(page.read(1) == 0xFFFFFFFF);
    page.write(0, &[0, 0, 0, 0, 0, 0]);
    require!(page.read(0) == 0);
    require!(page.read(1) == 0);
    require!(page.read(2) == 0);
    require!(page.read(3) == 0);
    require!(page.read(4) == 0);
    require!(page.read(5) == 0);
    require!(page.read(6) == 0xFFFFFFFF);
    page.write(2, &[1, 1]);
    require!(page.read(0) == 0);
    require!(page.read(1) == 0);
    require!(page.read(2) == 1);
    require!(page.read(3) == 1);
    require!(page.read(4) == 0);
    require!(page.read(5) == 0);
    require!(page.read(6) == 0xFFFFFFFF);
    page.write(3, &[2, 2, 2, 2, 2]);
    require!(page.read(0) == 0);
    require!(page.read(1) == 0);
    require!(page.read(2) == 1);
    require!(page.read(3) == 2);
    require!(page.read(4) == 2);
    require!(page.read(5) == 2);
    require!(page.read(6) == 2);
    require!(page.read(7) == 2);
    require!(page.read(8) == 0xFFFFFFFF);
    true
}

// Utility to build the run-length-encoded representation one piece at a time.
// Used by FakePage::write
struct RleBuilder {
    cur_run: usize,
    lens: [u16; NUM_RUNS],
    values: [u32; NUM_RUNS],
}

impl RleBuilder {
    pub fn new() -> RleBuilder {
        RleBuilder {
            cur_run: 0,
            lens: [0; NUM_RUNS],
            values: [0; NUM_RUNS],
        }
    }

    pub fn append(&mut self, value: u32) {
        if value == self.values[self.cur_run] {
            self.lens[self.cur_run] += 1;
        } else if self.lens[0] == 0 {
            // This is the first append call on this builder.
            self.lens[0] = 1;
            self.values[0] = value;
        } else {
            self.cur_run += 1;
            self.lens[self.cur_run] = 1;
            self.values[self.cur_run] = value;
        }
    }

    pub fn build(self) -> ([u16; NUM_RUNS], [u32; NUM_RUNS]) {
        (self.lens, self.values)
    }
}

#[test]
fn test_rle_builder() -> bool {
    let mut builder = RleBuilder::new();
    for _ in 0..123 { builder.append(3);  }
    for _ in 0..278 { builder.append(14); }
    for _ in 0..111 { builder.append(15); }
    let (lens, values) = builder.build();
    require!(lens == [123, 278, 111, 0, 0]);
    require!(values[0..3] == [3, 14, 15]);
    true
}
