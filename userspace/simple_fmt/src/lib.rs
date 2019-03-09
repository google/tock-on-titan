// Copyright 2018 Google LLC
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

#![no_std]

// The tests for this library are run on x86_64, not Thumb, as they are quite
// expensive. They compare against std's number formatting and therefore depend
// on std.
#[cfg(test)]
#[macro_use]
extern crate std;

/// Bases supported by fmt_u32
pub enum Base {
    Decimal = 10,
    Hexadecimal = 16,
}

/// Formats a u32 in base `base`, using the provided buffer for storage.
/// The user must provide a large-enough buffer -- this will not fail gracefully
/// if `buffer` is too small.
pub fn fmt_u32(mut num: u32, base: Base, buffer: &mut [u8]) -> &str {
    let base = base as u32;
    let mut position = buffer.len();
    loop {
        position -= 1;
        buffer[position] = digit_to_ascii((num % base) as u8);
        num /= base;
        if num == 0 {
            // At this point, all entries in `buffer` from `position` through
            // the end of `buffer` have been set to ASCII characters and are
            // therefore valid UTF-8.
            return unsafe { core::str::from_utf8_unchecked(&buffer[position..]) }
        }
    }
}

/// Formats a i32 as a decimal number, using the provided buffer for storage.
pub fn i32_to_decimal(num: i32, buffer: &mut [u8; 11]) -> &str {
    if num >= 0 {
        return fmt_u32(num as u32, Base::Decimal, &mut buffer[1..]);
    }
    if num == i32::min_value() { return "-2147483648" };

    let abs_val_len = fmt_u32((-num) as u32, Base::Decimal, &mut buffer[1..]).len();
    buffer[10 - abs_val_len] = b'-';
    return unsafe { core::str::from_utf8_unchecked(&buffer[10 - abs_val_len..]) };
}

// Given an individual digit to print (e.g. 5), returns the corresponding ASCII
// character. Works for bases 2-36 -- may return invalid ASCII for values of
// digit larger than 35.
fn digit_to_ascii(digit: u8) -> u8 {
    if digit < 10 { digit + b'0' } else { digit - 10 + b'a' }
}

#[cfg(test)]
mod tests {
    use super::{Base,fmt_u32,i32_to_decimal};

    // Runs the given callback in parallel for every value of a u32.
    fn for_each_u32(mut callback: impl Fn(u32) + Copy + Send + Sync) {
        // Rayon only supports parallel iteration for closed-open ranges, so we
        // can't create a parallel iterator that iterates through the entire
        // range. Instead we create an iterator that gets us almost all the way
        // then calls the callback on the last value.
        use rayon::iter::{IntoParallelIterator,ParallelIterator};
        (0..u32::max_value()).into_par_iter().for_each(callback);
        callback(u32::max_value());
    }

    // Takes 190 seconds on jrvanwhy@'s 12-core system.
    #[test]
    fn test_fmt_u32() {
        for_each_u32(|num| {
            let mut buffer = [0; 10];
            assert_eq!(fmt_u32(num, Base::Decimal,     &mut buffer), format!("{}",   num));
            assert_eq!(fmt_u32(num, Base::Hexadecimal, &mut buffer), format!("{:x}", num));
        });
    }

    // Takes 100 seconds on jrvanwhy@'s 12-core system.
    #[test]
    fn test_i32_to_decimal() {
        for_each_u32(|num| {
            let mut buffer = [0; 11];
            assert_eq!(i32_to_decimal(num as i32, &mut buffer), format!("{}", num as i32));
        });
    }
}
