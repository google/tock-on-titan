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

#![feature(alloc)]
#![no_std]

extern crate alloc;
extern crate tock;

fn main() {
    use tock::console::Console;

    let mut console = Console::new();
    loop {
        use core::fmt::Write;
        use tock::timer;
        console
            .write_str("Hello, World!\n")
            .expect("Failed console write");
        timer::sleep(timer::Duration::from_ms(1000));
    }
}
