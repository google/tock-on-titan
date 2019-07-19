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

mod hil;

#[test]
fn basic_test() -> bool {
    use core::fmt::Write;
    let _ = writeln!(libtock::console::Console::new(),
                     "Cat video count: {}\nWhat we eat: {:x}", 9001, 3405705229u32);
    libtock::timer::sleep(libtock::timer::Duration::from_ms(100));
    true
}
