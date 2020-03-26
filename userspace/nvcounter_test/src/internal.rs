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

use crate::fake_flash::{ErrorTime, FakeFlash, HIGH_PAGE_START};
use h1::hil::flash::flash::Flash;
use h1::nvcounter::internal::*;
use kernel::ReturnCode::SuccessWithValue;
use test::require;

#[test]
fn test_read_page_count() -> bool {
    let flash = FakeFlash::new();
    require!(read_page_count(Page::High, &flash) == 0);
    let mut buffer = [0x3FFFFFFF];
    flash.write(HIGH_PAGE_START, &mut buffer);
    require!(read_page_count(Page::High, &flash) == 1);
    let mut buffer = [0x003CFFFF];
    flash.write(HIGH_PAGE_START, &mut buffer);
    require!(read_page_count(Page::High, &flash) == 3);
    // Simulate a partial write.
    let mut buffer = [0x002CFFFF];
    flash.write(HIGH_PAGE_START, &mut buffer);
    require!(read_page_count(Page::High, &flash) == 4);
    // Simulate a bit flip
    let mut buffer = [0xFF7FFFFF];
    flash.write(HIGH_PAGE_START + 100, &mut buffer);
    require!(read_page_count(Page::High, &flash) == 808);
    true
}

#[test]
fn test_start_increment() -> bool {
    let flash = FakeFlash::new();
    // Simulate a bit flip
    let mut buffer = [0xFF7FFFFF];
    flash.write(HIGH_PAGE_START + 100, &mut buffer);

    let mut buffer = [0];
    start_increment(Page::High, 808, &flash, &mut buffer);
    require!(flash.read(HIGH_PAGE_START + 101) == SuccessWithValue { value: 0x3CFFFFFF });

    // Simulate a write error, make sure the correct return code and buffer are
    // returned.
    flash.configure_error(Some(ErrorTime::Fast));
    let mut buffer = [0];
    let (return_code, buffer) = start_increment(Page::High, 809, &flash, &mut buffer);
    require!(return_code == kernel::ReturnCode::FAIL);
    require!(buffer.is_some());

    true
}

// Marked ignore because this takes on the order of a minute.
#[test]
#[ignore]
fn test_full_count() -> bool {
    use core::convert::TryInto;
    let mut buffer = [0];
    let mut buffer_ref = Some(&mut buffer);
    let flash = FakeFlash::new();
    for i in 0..COUNTS_PER_PAGE {
        require!(read_page_count(Page::Low, &flash) == i);
        start_increment(Page::Low, i, &flash, buffer_ref.take().unwrap());
        buffer_ref = flash.retrieve_buffer().map(|b| b.try_into().unwrap());
    }
    require!(read_page_count(Page::Low, &flash) == COUNTS_PER_PAGE);
    let (return_code, buffer) = start_increment(
        Page::Low, COUNTS_PER_PAGE, &flash, buffer_ref.take().unwrap());
    require!(return_code == kernel::ReturnCode::ESIZE);
    require!(buffer.is_some());
    true
}
