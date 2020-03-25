// Copyright 2020 Google LLC
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

// Syscall driver for libtock-rs debugging. Allows applications to print 1, 2,
// or 3 integers to the console using only the command syscall. The integers are
// passed through the minor number and command arguments. Unlike the console
// driver, this does not rely on the allow syscall (useful because relocations
// are not currently working). The syscall is nonblocking and does not produce
// an event. Calling it multiple times in quick succession will not work -- it
// fills up a buffer and stops printing more messages. The driver number is
// 0x80000001.

use kernel::{AppId, Driver, ReturnCode};

// Matched to LowLevelDebug until Tock 1.5 is released.
// TODO: When we update to Tock 1.5, replace UintPrinter with LowLevelDebug.
pub const DRIVER_NUM: usize = 0x00008;
pub struct UintPrinter {}

impl UintPrinter {
    pub fn new() -> UintPrinter {
        UintPrinter {}
    }
}

impl Driver for UintPrinter {
    fn command(&self, minor_num: usize, r2: usize, r3: usize, _caller_id: AppId) -> ReturnCode {
        match (minor_num, r2, r3) {
            (_, 0, 0) => debug!("{}", minor_num),
            (_, _, 0) => debug!("{} {}", minor_num, r2),
            (_, _, _) => debug!("{} {} {}", minor_num, r2, r3),
        }
        ReturnCode::SUCCESS
    }
}
