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

//! Test RNG hardware

use crate::hil::rng::{Client, Continue, RNG};

pub struct TestRng<'a> {
    rng: &'a dyn RNG<'a>,
}

impl<'a> TestRng<'a> {
    pub fn new(rng: &'a dyn RNG<'a>) -> Self {
        TestRng { rng: rng }
    }

    pub fn run(&self) {
        self.rng.get();
    }
}

impl<'a> Client for TestRng<'a> {
    fn randomness_available(&self, randomness: &mut dyn Iterator<Item = u32>) -> Continue {
        print!("Randomness: \r");
        randomness.take(5).for_each(|r| print!("  [{:x}]\r", r));
        Continue::Done
    }
}
