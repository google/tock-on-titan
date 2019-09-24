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

use core::cmp;
use core::mem;
use core::cell::Cell;
use crate::hil::personality::{Client, Personality, PersonalityData};
use crate::hil::flash;
use kernel::ReturnCode;
use kernel::common::cells::{OptionalCell, TakeCell};

#[derive(Copy, Clone, Debug, PartialEq)]
enum State {
    Idle,
    ErasingU8,
    WritingU8,
    ErasingStruct,
    WritingStruct,
}

pub struct PersonalityDriver<'a> {
    state: Cell<State>,
    client: OptionalCell<&'a dyn Client<'a>>,
    flash: OptionalCell<&'a dyn flash::Flash<'a>>,
    write_buffer: TakeCell<'a, [u32]>,
}

pub static mut PERSONALITY: PersonalityDriver<'static> = unsafe {PersonalityDriver::new() };

pub static mut BUFFER: [u32; PAGE_SIZE_U32] = [0; PAGE_SIZE_U32];


// Personality data is stored as the third-to-last (N-3) page of flash;
// it is followed by the two pages used as a counter.
const PERSONALITY_ADDRESS: usize = flash::h1b_hw::H1B_FLASH_SIZE - (3 * flash::h1b_hw::H1B_FLASH_PAGE_SIZE) ;
const PERSONALITY_ADDRESS_U32: usize = PERSONALITY_ADDRESS / 4;
const PERSONALITY_SIZE: usize = flash::h1b_hw::H1B_FLASH_PAGE_SIZE;
const PAGE_SIZE_U32: usize    = flash::h1b_hw::H1B_FLASH_PAGE_SIZE / 4;

impl<'a> PersonalityDriver<'a> {
    const unsafe fn new() -> PersonalityDriver<'a> {
        PersonalityDriver {
            state: Cell::new(State::Idle),
            client: OptionalCell::empty(),
            flash: OptionalCell::empty(),
            write_buffer: TakeCell::empty(),
        }
    }

    pub fn set_flash(&self, flash: &'a dyn flash::Flash<'a>) {
        self.flash.set(flash);
    }

    pub fn set_buffer(&self, buf: &'a mut [u32]) {
        self.write_buffer.replace(buf);
    }

    pub fn set_client(&self, client: &'a dyn Client<'a>) {
        self.client.replace(client);
    }

    fn start_write(&self, target: usize) -> bool {
        if self.flash.is_none() || self.write_buffer.is_none() {
            false
        } else {
            let buf = self.write_buffer.take().unwrap();
            self.flash.map(move |flash| {
                let (_rcode, opt) = flash.write(target, buf);
                match opt {
                    None => true,  // Operation successful
                    Some(buf) => { // Not successful
                        self.write_buffer.replace(buf);
                        false
                    }
                }
            }).unwrap()
        }
    }

}

impl<'a> Personality<'a> for PersonalityDriver<'a> {
    fn set_client(&self, client: &'a dyn Client<'a>) {
        self.client.set(client);
    }

    fn get(&self, data: &mut PersonalityData) -> ReturnCode {
        unsafe {
            self.flash.map_or(ReturnCode::ENOMEM, |flash| {
                let mut personality_ptr = mem::transmute::<*mut PersonalityData, *mut u32>(data);
                let word_count = PAGE_SIZE_U32;
                for i in 0..word_count {
                    let result = flash.read(PERSONALITY_ADDRESS_U32 + i);
                    match result {
                        ReturnCode::SuccessWithValue{value: v} => {
                            *personality_ptr = v as u32;
                            personality_ptr = personality_ptr.offset(1);
                        }
                        _ => {
                            return result;
                        }
                    }
                }
                ReturnCode::SUCCESS
            })
        }
    }

    fn get_u8(&self, data: &mut [u8]) -> ReturnCode {
        if data.len() < PERSONALITY_SIZE {
            ReturnCode::ESIZE
        } else {
            unsafe {
                self.flash.map_or(ReturnCode::ENOMEM, |flash| {
                    let ptr = data.as_mut_ptr();
                    let mut personality_ptr = mem::transmute::<*mut u8, *mut u32>(ptr);
                    let word_count = PAGE_SIZE_U32;
                    for i in 0..word_count {
                        let result = flash.read(PERSONALITY_ADDRESS_U32 + i);
                        match result {
                            ReturnCode::SuccessWithValue{value: v} => {
                                *personality_ptr = v as u32;
                                personality_ptr = personality_ptr.offset(1);
                            }
                            _ => {
                                return result;
                            }
                        }
                    }
                    ReturnCode::SUCCESS
                })
            }
        }
    }

    fn set(&self, data: &mut PersonalityData) -> ReturnCode {
        if self.state.get() != State::Idle {
            return ReturnCode::EBUSY;
        }
        if self.flash.is_some() {
            self.flash.map(move |flash| {
                let offset = PERSONALITY_ADDRESS;
                let page = offset / flash::h1b_hw::H1B_FLASH_PAGE_SIZE;
                let rval = flash.erase(page);
                match rval {
                    ReturnCode::SUCCESS => {
                        self.write_buffer.map(|buffer| {
                            self.state.set(State::ErasingStruct);
                            unsafe {
                                let mut ptr = mem::transmute::<*mut PersonalityData, *mut u32>(data);
                                let word_count = PAGE_SIZE_U32;
                                for i in 0..word_count {
                                    buffer[i] = *ptr;
                                    ptr = ptr.offset(1);
                                }
                            }
                        });
                        ReturnCode::SUCCESS
                    },
                    _ => {
                        rval
                    }
                }
            }).unwrap()
        } else {
            ReturnCode::ENOMEM
        }
    }

    fn set_u8(&self, data: &mut [u8]) -> ReturnCode {
        if data.len() < PERSONALITY_SIZE {
            debug!("personality::set_u8: ESIZE");
            ReturnCode::ESIZE
        }
        else if self.state.get() != State::Idle {
            debug!("personality::set_u8 EBUSY");
            ReturnCode::EBUSY
        } else {
            if self.flash.is_some() {
                self.flash.map(move |flash| {
                    let offset = PERSONALITY_ADDRESS;
                    let page = offset / flash::h1b_hw::H1B_FLASH_PAGE_SIZE;
                    let rval = flash.erase(page);

                    match rval {
                        ReturnCode::SUCCESS => {
                            self.write_buffer.map(|buffer| {
                                self.state.set(State::ErasingU8);
                                let len = cmp::min(data.len(), flash::h1b_hw::H1B_FLASH_PAGE_SIZE);
                                unsafe {
                                    let mut ptr = mem::transmute::<*mut u32, *mut u8>(buffer.as_mut_ptr());
                                    for i in 0..len {
                                        *ptr = data[i];
                                        ptr = ptr.offset(1);
                                    }
                                }
                            });
                            ReturnCode::SUCCESS
                        },
                        _ => {
                            rval
                        }
                    }
                }).unwrap()
            } else {
                ReturnCode::ENOMEM
            }
        }
    }
}

impl<'a> flash::Client<'a> for PersonalityDriver<'a> {
    fn erase_done(&self, _rcode: ReturnCode) {
        let state = self.state.get();
        let target = PERSONALITY_ADDRESS_U32; // Write offset is in words
        match state {
            State::ErasingStruct => {
                if self.start_write(target) {
                    self.state.set(State::WritingStruct);
                } else {
                    self.client.map(|c| c.set_done(ReturnCode::FAIL));
                    self.state.set(State::Idle);
                }
            }

            State::ErasingU8 => {
                if self.start_write(target) {
                    self.state.set(State::WritingU8);
                } else {
                    debug!("personality::write_u8 failed");
                    self.client.map(|c| c.set_u8_done(ReturnCode::FAIL));
                    self.state.set(State::Idle);
                }
            },
            _ => { // Should never happen -pal
                debug!("Erase done called but in state {:?}", state);
            }
        }
    }

    fn write_done(&self, _data: &'a mut [u32], rcode: ReturnCode) {
        let state = self.state.get();
        match state {
            State::WritingStruct => {
                self.state.set(State::Idle);
                self.client.map(|c| c.set_done(rcode));
            },
            State::WritingU8 => {
                self.state.set(State::Idle);
                self.client.map(|c| {
                    c.set_u8_done(rcode);
                });
            },
            _ => { // Should never happen -pal
                debug!(" -- ERROR: personality::write_done in state {:?}", state);
            },
        }
    }
}
