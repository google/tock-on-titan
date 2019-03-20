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

/// Tock userspace library for simple console printing/debugging. This avoids
/// dynamic polymorphism and as a result is much lighter weight than core::fmt.
/// Its interface is inspired by absl::StrCat. It does not require dynamic
/// memory allocation.

use libtock::console::Console;

/// Prints a sequence of values to the console.
///
/// # Example
/// ```
/// console!("Cat video count: ", 9001, "\nWhere we eat: ", hex(51966), "\n");
/// ```
#[macro_export]
macro_rules! console {
    ($($v:expr),*) => {
        { $(simple_print::Printable::print($v);)* }
    };
}

/// Marks that a value should be printed in hexadecimal rather than in decimal.
///
/// # Example
/// ```
/// console!("Address of 8: ", hex(&8));
/// ```
pub fn hex<T: HexPrintable>(value: T) -> Hex {
    Hex { value: value.to_u32() }
}

// -----------------------------------------------------------------------------
// Implementation details below.
// -----------------------------------------------------------------------------

use simple_fmt::Base;

pub trait Printable {
    fn print(self);
}

impl Printable for &str {
    fn print(self) {
        use core::fmt::Write;
        // Tock's Console cannot fail.
        let _ = Console::new().write_str(self);
    }
}

impl Printable for i32 {
    fn print(self) {
        let mut buffer = [0; 11];
        Console::new().write(simple_fmt::i32_to_decimal(self, &mut buffer));
    }
}

impl Printable for u32 {
    fn print(self) {
        let mut buffer = [0; 10];
        Console::new().write(simple_fmt::fmt_u32(self, Base::Decimal, &mut buffer));
    }
}

// Types that may be printed in hex. Currently, all the types we'd like to print
// as hex are equivalent to u32, so for simplicity we convert everything to u32.
pub trait HexPrintable {
    fn to_u32(self) -> u32;
}

pub struct Hex { value: u32 }

impl Printable for Hex {
    fn print(self) {
        let mut buffer = [0; 8];
        Console::new().write(simple_fmt::fmt_u32(self.value, Base::Hexadecimal, &mut buffer));
    }
}

impl HexPrintable for u32 {
    fn to_u32(self) -> u32 { self }
}

impl HexPrintable for usize {
    fn to_u32(self) -> u32 { self as u32 }
}

impl<T> HexPrintable for *const T {
    fn to_u32(self) -> u32 { self as u32 }
}

impl<T> HexPrintable for &T {
    fn to_u32(self) -> u32 { self as *const T as u32 }
}
