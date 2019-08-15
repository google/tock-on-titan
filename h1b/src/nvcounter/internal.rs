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

use crate::hil;
use kernel::ReturnCode;

const COUNTS_PER_WORD: u32 = 8;
pub const WORDS_PER_PAGE: usize = 512;
pub const COUNTS_PER_PAGE: u32 = COUNTS_PER_WORD * WORDS_PER_PAGE as u32;

// Tasks the counter can execute.
#[derive(Clone, Copy, PartialEq)]
pub enum Task {
    Initialize,
    Increment,
}

// The flash page numbers in use by the counter.
#[derive(PartialEq)]
pub enum Page {
    High = 254,
    Low = 255,
}

// Reads the count stored in the given page.
pub fn read_page_count<'f, F: hil::flash::Flash<'f>>(page: Page, flash: &F) -> u32 {
    // Read the count by looking for the last page with 0's. This is slightly
    // more robust against bit flips than scanning from the beginning, as a bit
    // flip away from the current value's location will cause a roll-forward
    // (acceptable) rather than a rollback (unacceptable).

    let page_offset = page as usize * WORDS_PER_PAGE;
    // Locate the "current" word (the last word that has been written since the
    // last erase, or the first word if the page is currently erased) and the
    // count it represents.
    let (current_index, current_count) = (|| {
        for i in (0..WORDS_PER_PAGE).rev() {
            // The read should never fail, as `page`'s value is constrained by
            // the type system and WORDS_PER_PAGE is not enough for i to cause
            // an overflow.
            let value = match flash.read(page_offset + i) {
                ReturnCode::SuccessWithValue { value } => value,
                _ => return (0, 0),
            };
            if value != 0xFFFFFFFF {
                // Decoding is somewhat tolerant of partially-written states,
                // preferring to overestimate the count rather than
                // underestimate it.
                if value & 0x3CFFFFFF == 0x3CFFFFFF { return (i, 1); }
                if value & 0xC3FFFFFF == 0x00FFFFFF { return (i, 2); }
                if value & 0xFF3CFFFF == 0x003CFFFF { return (i, 3); }
                if value & 0xFFC3FFFF == 0x0000FFFF { return (i, 4); }
                if value & 0xFFFF3CFF == 0x00003CFF { return (i, 5); }
                if value & 0xFFFFC3FF == 0x000000FF { return (i, 6); }
                if value & 0xFFFFFF3C == 0x0000003C { return (i, 7); }
                return (i, 8);
            }
        }
        (0, 0)
    })();

    COUNTS_PER_WORD * current_index as u32 + current_count
}

// Begins the write to increment the value stored in the given flash page.
// Requires the current count, and will return ESIZE if the count is maxed out.
pub fn start_increment<'f, F: hil::flash::Flash<'f>>(
    page: Page, current_value: u32, flash: &F, buffer: &'f mut [u32; 1])
    -> (ReturnCode, Option<&'f mut [u32; 1]>)
{
    use core::convert::TryInto;
    const WRITE_PATTERNS: [u32; COUNTS_PER_WORD as usize] =
        [0x3CFFFFFF, 0x00FFFFFF, 0x003CFFFF, 0x0000FFFF,
         0x00003CFF, 0x000000FF, 0x0000003C, 0x00000000];
    if current_value >= COUNTS_PER_PAGE { return (ReturnCode::ESIZE, Some(buffer)); }
    let word_to_write = (current_value / COUNTS_PER_WORD) as usize;
    buffer[0] = WRITE_PATTERNS[(current_value % COUNTS_PER_WORD) as usize];
    let (return_code, buffer) = flash.write(WORDS_PER_PAGE * page as usize + word_to_write, buffer);
    (return_code, buffer.map(|e| e.try_into().unwrap()))
}

// Returns true if the given page was reset.
pub fn page_empty<'f, F: hil::flash::Flash<'f>>(page: Page, flash: &F) -> bool {
    let page_start = page as usize * WORDS_PER_PAGE;
    let page_end = page_start + WORDS_PER_PAGE;  // 1 past the end
    (page_start..page_end).all(|word| {
        flash.read(word) == ReturnCode::SuccessWithValue { value: 0xFFFFFFFF }
    })
}

// Return true if the low page is full (maxed out).
pub fn low_page_full<'f, F: hil::flash::Flash<'f>>(flash: &F) -> bool {
    flash.read(Page::Low as usize * WORDS_PER_PAGE + WORDS_PER_PAGE - 1)
        == ReturnCode::SuccessWithValue { value: 0x00000000 }
}

// Computes the counter value for the given low and high counts.
pub fn counter_value(mut high_count: u32, mut low_count: u32) -> u32 {
    if high_count & 1 != 0 {
        // High count is odd, fast-forward to the state after low_count is
        // erased.
        high_count += 1;
        low_count = 0;
    }
    (COUNTS_PER_PAGE + 1) * (high_count >> 1) + low_count
}
