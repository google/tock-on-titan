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

#![allow(dead_code)]

//! Driver for device attestation (personality) data.

use hil::personality::{Client, Personality, PersonalityData};
use kernel::ReturnCode;
use kernel::common::cells::OptionalCell;

pub struct Driver<'a> {
    client: OptionalCell<&'a Client>,
}

static mut PERSO: PersonalityData = PersonalityData {
    checksum: [0; 8],
    salt: [0; 8],
    pub_x: [0; 8],
    pub_y: [0; 8],
    certificate_hash: [0; 8],
    certificate_len: 0,
    certificate: [0; 2048 - (4 + 5 * 32)],
};


impl<'a> Driver<'a> {
    const unsafe fn new() -> Driver<'a> {
        Driver {
            client: OptionalCell::empty(),
        }
    }
}

impl<'a> Personality<'a> for Driver<'a> {

    fn set_client(&self, client: &'a Client) {
        self.client.set(client);
    }

    fn get(&self, data: &'a mut PersonalityData) {
        unsafe {
            *data = PERSO;
        }
    }

    fn set(&self, data: &'a PersonalityData) -> ReturnCode {
        unsafe {
            PERSO = *data;
        }
        return ReturnCode::ENOSUPPORT;
    }

}
