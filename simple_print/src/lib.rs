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
#![feature(alloc)]

/// Tock userspace library for simple console printing/debugging. This avoids
/// dynamic polymorphism and as a result is much lighter weight than core::fmt.
/// Its interface is inspired by absl::StrCat.

// Although currently it is very malloc-happy, it is designed to be adapted to
// minimize allocations. It should be possible to print numeric values using
// only fixed-size stack-allocated buffers with no change to the public API.
// Similarly, once we have some form of allow_const (i.e. an allow() syscall
// that can point into RAM or flash), we can remove the allocation from the path
// that prints a &str.
//
// The downside of this design is it produces a sequence of console writes
// rather than allocating a buffer and doing the write in a single syscall,
// which may hurt performance.

extern crate alloc;

use tock::console::Console;

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

pub trait Printable {
    fn print(self);
}

impl Printable for &str {
    fn print(self) {
        use core::fmt::Write;
        // Tock's Console cannot fail.
        let _ = Console.write_str(self);
    }
}

impl Printable for alloc::string::String {
    fn print(self) {
        Console.write(self);
    }
}

impl Printable for i32 {
    fn print(self) {
        Console.write(tock::fmt::i32_as_decimal(self));
    }
}

impl Printable for u32 {
    fn print(self) {
        Console.write(tock::fmt::u32_as_decimal(self));
    }
}

// Types that may be printed in hex. Currently libtock-rs only supports
// formatting u32's as hex; for simplicity, we simply convert anything we'd like
// to print to u32's.
pub trait HexPrintable {
    fn to_u32(self) -> u32;
}

pub struct Hex { value: u32 }

impl Printable for Hex {
    fn print(self) {
        Console.write(tock::fmt::u32_as_hex(self.value));
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
