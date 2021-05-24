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

#![no_std]

use libtock_core::stack_size;

stack_size!{2048}

// Note: this currently calls into UintPrinter, not LowLevelDebug. When Tock 1.5
// is released, we should replace UintPrinter with LowLevelDebug in golf2, at
// which point this app will work correctly.
fn main() {
    use libtock::timer::Duration;

    // LowLevelDebug: App 0x0 prints 0x123
    libtock::debug::low_level_print1(0x123);

    // LowLevelDebug: App 0x0 prints 0x456 0x789
    libtock::debug::low_level_print2(0x456, 0x789);

    // Print a series of messages quickly to overfill the queue and demonstrate
    // the message drop behavior.
    for _ in 0..10 {
        libtock::debug::low_level_print1(0x1);
        libtock::debug::low_level_print2(0x2, 0x3);
    }

    // Wait for the above to print then output a few more messages.
    let drivers = libtock::retrieve_drivers().ok().expect("retrieve_drivers");
    let mut timer_context = drivers.timer;
    let mut timer_driver = timer_context.create_timer_driver();
    let timer_driver = timer_driver.activate().ok().expect("timer activate");
    unsafe {
        ::libtock::executor::block_on(timer_driver.sleep(Duration::from_ms(100)))
            .ok().expect("sleep");
    }

    // LowLevelDebug: App 0x0 prints 0xA
    libtock::debug::low_level_print1(0xA);

    // LowLevelDebug: App 0x0 prints 0xB 0xC
    libtock::debug::low_level_print2(0xB, 0xC);

    // LowLevelDebug: App 0x0 status code 0x1
    panic!()
}
