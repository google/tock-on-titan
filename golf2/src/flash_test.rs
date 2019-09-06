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

use ::kernel::ReturnCode;
use ::h1b::hil::flash::Flash;

/// Test tool for the flash driver -- runs test cases using the provided driver.
/// Used for integration testing against the real hardware. Will set itself as
/// the flash driver's client.
pub struct FlashTest<F: Flash<'static> + 'static> {
    driver: &'static F,
    state: ::core::cell::Cell<Option<Tests>>,
}

static mut BUF: [u32; 1] = [0; 1];

impl<F: Flash<'static> + 'static> ::h1b::hil::flash::Client<'static> for FlashTest<F> {
    fn erase_done(&self, code: ReturnCode) {
        match self.state.take() {
            None => println!("FlashTest FAIL: erase_done() w/ state == None"),
            Some(Tests::Erase1) => self.erase1_done(code),
            Some(Tests::Write1) => println!("FlashTest FAIL: erase_done() during Write"),
            Some(Tests::Write2) => println!("FlashTest FAIL: erase_done() during Write"),
            Some(Tests::Erase2) => self.erase2_done(code),
        }
    }

    fn write_done(&self, _data: &'static mut [u32], code: ReturnCode) {
        match self.state.take() {
            None => println!("FlashTest FAIL: write_done() w/ state == None"),
            Some(Tests::Erase1) => println!("FlashTest FAIL: write_done() during Erase"),
            Some(Tests::Write1) => self.write1_done(code),
            Some(Tests::Write2) => self.write2_done(code),
            Some(Tests::Erase2) => println!("FlashTest FAIL: write_done() during Erase"),
        }
    }
}

impl<F: Flash<'static> + 'static> FlashTest<F> {
    const TEST_PAGE: usize = 255;
    const TEST_WORD: usize = 512 * Self::TEST_PAGE;


    pub fn new(driver: &'static F) -> Self {
        FlashTest { driver, state: ::core::cell::Cell::new(None) }
    }

    #[allow(unused)]
    pub fn run(&'static self) {
        self.driver.set_client(self);
        self.erase1_start();
    }

    // -------------------------------------------------------------------------
    // Test cases
    // -------------------------------------------------------------------------
    fn erase1_start(&self) {
        println!("FlashTest: Beginning Erase1. code: {:?}", self.driver.erase(Self::TEST_PAGE));
        self.state.set(Some(Tests::Erase1));
    }

    fn erase1_done(&self, code: ReturnCode) {
        println!("FlashTest: Erase1 done. code: {:?}", code);
        let read_value = self.driver.read(Self::TEST_WORD);
        if read_value != 0xFFFFFFFF {
            println!("FlashTest: Erase1 failed, value: {}", read_value);
        }
        self.write1_start();
    }

    fn write1_start(&self) {
        unsafe {
            BUF[0] = 0x0000FFFF;
            println!("FlashTest: Beginning Write1. code: {:?}",
                     self.driver.write(Self::TEST_WORD, &mut BUF));
        }
        self.state.set(Some(Tests::Write1));
    }

    fn write1_done(&self, code: ReturnCode) {
        println!("FlashTest: Write1 done. code: {:?}", code);
        let read_value = self.driver.read(Self::TEST_WORD);
        if read_value != 0x0000FFFF {
            println!("FlashTest: Write1 failed, value: {}", read_value);
        }
        self.write2_start();
    }

    fn write2_start(&self) {
        unsafe {
            BUF[0] = 0x00000000;
            println!("FlashTest: Beginning Write2. code: {:?}",
                     self.driver.write(Self::TEST_WORD, &mut BUF));
        }
        self.state.set(Some(Tests::Write2));
    }

    fn write2_done(&self, code: ReturnCode) {
        println!("FlashTest: Write2 done. code: {:?}", code);
        let read_value = self.driver.read(Self::TEST_WORD);
        if read_value != 0x00000000 {
            println!("FlashTest: Write2 failed, value: {}", read_value);
        }
        self.erase2_start();
    }

    fn erase2_start(&self) {
        println!("FlashTest: Beginning Erase2. code: {:?}", self.driver.erase(Self::TEST_PAGE));
        self.state.set(Some(Tests::Erase2));
    }

    fn erase2_done(&self, code: ReturnCode) {
        println!("FlashTest: Erase2 done. code: {:?}", code);
        let read_value = self.driver.read(Self::TEST_WORD);
        if read_value != 0xFFFFFFFF {
            println!("FlashTest: Erase2 failed, value: {}", read_value);
        }
    }
}

enum Tests {
    Erase1,  // First erase -- puts the flash page in a known state.
    Write1,  // First write. Converts 0xFFFFFFFF to 0x0000FFFF
    Write2,  // Second write. Converts 0x0000FFFF to 0x00000000
    Erase2,  // Second erase, should reset back to 0xFFFFFFFF
}
