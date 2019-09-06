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

//! Peripheral driver for device attestation (personality) data.  This
//! is per-device data that will be stored durably on the device; this
//! implementations currently stores it in RAM.

use core::mem;
use hil::personality::{Client, Personality, PersonalityData};
use hil::flash;
use kernel::ReturnCode;
use kernel::common::cells::OptionalCell;


pub struct PersonalityDriver<'a> {
    client: OptionalCell<&'a Client>,
    flash: OptionalCell<&'a flash::Flash<'a>>,
}

pub static mut PERSONALITY: PersonalityDriver<'static> = unsafe {PersonalityDriver::new() };

const PERSONALITY_SIZE: usize = 2048;

// Personality data is stored as the third-to-last (N-3) page of flash;
// it is followed by the two pages used as a counter.
const PERSONALITY_ADDDRESS: usize = 0;

static mut PERSO: PersonalityData = PersonalityData {
    checksum: [0; 8],
    salt: [0; 8],
    pub_x: [0; 8],
    pub_y: [0; 8],
    certificate_hash: [0; 8],
    certificate_len: 0,
    certificate: [0; PERSONALITY_SIZE - (4 + 5 * 32)],
};


impl<'a> PersonalityDriver<'a> {
    const unsafe fn new() -> PersonalityDriver<'a> {
        PersonalityDriver {
            client: OptionalCell::empty(),
            flash: OptionalCell::empty(),
        }
    }

    pub fn set_flash(&self, flash: &'a flash::Flash<'a>) {
        self.flash.set(flash);
    }

}

impl<'a> Personality<'a> for PersonalityDriver<'a> {

    fn set_client(&self, client: &'a Client) {
        self.client.set(client);
    }

    fn get(&self, data: &mut PersonalityData) {
        unsafe {
            *data = PERSO;
        }
    }

    fn get_u8(&self, data: &mut [u8]) -> ReturnCode {
        if data.len() < PERSONALITY_SIZE {
            ReturnCode::ESIZE
        } else {
            unsafe {
                let ptr = data.as_mut_ptr();
                let personality_ptr = mem::transmute::<*mut u8, *mut PersonalityData>(ptr);
                *personality_ptr = PERSO;
            }
            ReturnCode::SUCCESS
        }
    }

    fn set(&self, data: &PersonalityData) -> ReturnCode {
        unsafe {
            PERSO = *data;
        }
        return ReturnCode::SUCCESS;
    }

    fn set_u8(&self, data: &[u8]) -> ReturnCode {
        if data.len() < PERSONALITY_SIZE {
            ReturnCode::ESIZE
        } else {
            unsafe {
                let ptr = data.as_ptr();
                let personality_ptr = mem::transmute::<*const u8, *const PersonalityData>(ptr);
                PERSO = *personality_ptr;
            }
            ReturnCode::SUCCESS
        }
    }
}

impl<'a> flash::Client<'a> for PersonalityDriver<'a> {
    fn erase_done(&self, rcode: ReturnCode) {

    }
    fn write_done(&self, data: &'a mut [u32], rcode: ReturnCode) {

    }
}
