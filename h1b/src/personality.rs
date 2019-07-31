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

#![allow(dead_code)]

//! Driver for device attestation (personality) data.

use core::mem;
use hil::personality::{Client, Personality, PersonalityData};
use kernel::ReturnCode;
use kernel::common::cells::OptionalCell;

pub struct PersonalityDriver<'a> {
    client: OptionalCell<&'a Client>,
}

pub static mut PERSONALITY: PersonalityDriver<'static> = unsafe {PersonalityDriver::new() };

static mut PERSO: PersonalityData = PersonalityData {
    checksum: [0; 8],
    salt: [0; 8],
    pub_x: [0; 8],
    pub_y: [0; 8],
    certificate_hash: [0; 8],
    certificate_len: 0,
    certificate: [0; 2048 - (4 + 5 * 32)],
};


impl<'a> PersonalityDriver<'a> {
    const unsafe fn new() -> PersonalityDriver<'a> {
        PersonalityDriver {
            client: OptionalCell::empty(),
        }
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
        if data.len() < 2048 {
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
        if data.len() < 2048 {
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
