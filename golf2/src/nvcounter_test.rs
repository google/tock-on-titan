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

use h1::nvcounter::NvCounter;
use kernel::ReturnCode;
use ReturnCode::SuccessWithValue;

/// Integration test for the NvCounter. Will set itself as the NvCounter's
/// client.
pub struct NvCounterTest<'t, C: NvCounter<'t>> {
    current_value: core::cell::Cell<usize>,
    failed: core::cell::Cell<bool>,
    nvcounter: &'t C,
}

impl<'t, C: NvCounter<'t>> NvCounterTest<'t, C> {
    #[allow(unused)]
    pub fn new(nvcounter: &'t C) -> NvCounterTest<'t, C> {
        NvCounterTest {
            current_value: Default::default(),
            failed: Default::default(),
            nvcounter
        }
    }

    /// Start the integration test. The test will run asynchronously in the
    /// background (and will print to the console).
    #[allow(unused)]
    pub fn run(&'t self) {
        self.nvcounter.set_client(self);
        println!("NvCounterTest: Beginning initialize. code: {:?}",
                 self.nvcounter.initialize());
    }
}

impl<'t, C: NvCounter<'t>> h1::nvcounter::Client for NvCounterTest<'t, C> {
    fn initialize_done(&self, status: ReturnCode) {
        println!("NvCounterTest: Initialize done, status: {:?}", status);
        if status != ReturnCode::SUCCESS {
            println!("NvCounterTest: FAILED");
            self.failed.set(true);
            return;
        }
        // self.current_value is already 0, as we don't increment and then
        // initialize.
        let increment_result = self.nvcounter.read_and_increment();
        println!("NvCounterTest: Beginning increment. Status: {:?}",
                 increment_result);
        if increment_result != (SuccessWithValue { value: 0 }) {
            println!("NvCounterTest: FAILED");
            self.failed.set(true);
        }
    }

    fn increment_done(&self, status: ReturnCode) {
        println!("NvCounterTest: increment_done({:?})", status);
        if status != ReturnCode::SUCCESS {
            println!("NvCounterTest: FAILED");
            self.failed.set(true);
            return;
        }
        self.current_value.set(self.current_value.get() + 1);
        if self.current_value.get() > 5000 {
            println!("NvCounterTest: Completed successfully!");
            return;
        }
        let increment_result = self.nvcounter.read_and_increment();
        println!("NvCounterTest: Beginning increment. Status: {:?}",
                 increment_result);
        let expected = SuccessWithValue { value: self.current_value.get() };
        if increment_result != expected {
            println!("NvCounterTest: FAILED");
            self.failed.set(true);
        }
    }
}
