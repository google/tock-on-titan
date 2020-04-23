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

use core::cell::Cell;
use ::kernel::common::cells::{OptionalCell, TakeCell};
use ::kernel::common::{List, ListLink, ListNode};
use ::kernel::ReturnCode;
use super::flash::Flash;
use super::flash::Client;

/// Virtualizes the H1 flash abstraction to support multiple clients.
pub struct MuxFlash<'f> {
    driver: &'f dyn Flash<'f>,
    users: List<'f, FlashUser<'f>>,
    in_flight: OptionalCell<&'f FlashUser<'f>>,
}

#[derive(Copy, Clone, PartialEq)]
enum Operation {
    Idle,
    Write(usize),        // offset in words
    Erase(usize),        // page number
}

pub struct FlashUser<'f> {
    mux: &'f MuxFlash<'f>,
    buffer: TakeCell<'f, [u32]>,
    write_len: Cell<usize>,
    write_pos: Cell<usize>,
    operation: Cell<Operation>,
    next: ListLink<'f, FlashUser<'f>>,
    client: OptionalCell<&'f dyn Client<'f>>,
}

impl<'f> Client<'f> for MuxFlash<'f> {
    fn erase_done(&self, rcode: ReturnCode) {
        self.in_flight.take().map(move |client| {
            client.erase_done(rcode);
        });
        self.do_next_op();
    }

    fn write_done(&self, data: &'f mut [u32], rcode: ReturnCode) {
        self.in_flight.take().map(move |client| {
            client.write_done(data, rcode);
        });
        self.do_next_op();
    }
}


impl<'f> FlashUser<'f> {
    pub const fn new(mux: &'f MuxFlash<'f>) -> FlashUser<'f> {
        FlashUser {
            mux: mux,
            buffer: TakeCell::empty(),
            write_len: Cell::new(0),
            write_pos: Cell::new(0),
            operation: Cell::new(Operation::Idle),
            next: ListLink::empty(),
            client: OptionalCell::empty()
        }
    }
}

impl<'f> Flash<'f> for FlashUser<'f> {
    fn erase(&self, page: usize) -> ReturnCode {
        if self.operation.get() != Operation::Idle {
            return ReturnCode::EBUSY;
        }
        self.operation.set(Operation::Erase(page));
        self.mux.do_next_op();
        ReturnCode::SUCCESS
    }

    fn read(&self, word: usize) -> ReturnCode {
        self.mux.read(word)
    }

    fn write(&self, target: usize, data: &'f mut [u32]) -> (ReturnCode, Option<&'f mut [u32]>) {
        if self.operation.get() != Operation::Idle {
            return (ReturnCode::EBUSY, Some(data));
        }
        self.write_pos.set(target);
        self.write_len.set(data.len());
        self.buffer.replace(data);
        self.operation.set(Operation::Write(target));
        self.mux.do_next_op();
        (ReturnCode::SUCCESS, None)
    }

    fn set_client(&'f self, client: &'f dyn Client<'f>) {
        self.mux.users.push_head(self);
        self.client.set(client);
    }
}


impl<'f> Client<'f> for FlashUser<'f> {
    fn erase_done(&self, rcode: ReturnCode) {
        self.operation.set(Operation::Idle);
        self.client.map(|client| client.erase_done(rcode));
    }

    fn write_done(&self, data: &'f mut [u32], rcode: ReturnCode) {
        self.operation.set(Operation::Idle);
        self.client.map(move |client| client.write_done(data, rcode));
    }
}

impl<'f> MuxFlash<'f> {
    pub fn new(driver: &'static dyn Flash<'f>) -> Self {
        MuxFlash {
            driver: driver,
            users: List::new(),
            in_flight: OptionalCell::empty(),
        }
    }

    fn do_next_op(&self) {
        if self.in_flight.is_some() {
            return;
        } // busy
        let mnode = self
            .users
            .iter()
            .find(|node| node.operation.get() != Operation::Idle);
        // This code is mostly borrowed from virtual_flash in
        // mainline Tock's capsule directory
        mnode.map(|node| {
            node.buffer.take().map_or_else(
                || {
                    // Erase doesn't require a buffer
                    match node.operation.get() {
                        Operation::Erase(page_number) => {
                            self.driver.erase(page_number);
                        }
                        _ => {} // Signal an error on Erase and Write?
                    };
                },
                |buf| {
                    match node.operation.get() {
                        Operation::Write(offset) => {
                            self.driver.write(offset, buf);
                        },
                        Operation::Erase(page_number) => {
                            self.driver.erase(page_number);
                        }
                        Operation::Idle => {} // Can't get here
                    }
                },
            );
            self.in_flight.set(node);
        });
    }

    fn read(&self, word: usize) -> ReturnCode {
        self.driver.read(word)
    }
}


impl<'f> ListNode<'f, FlashUser<'f>> for FlashUser<'f> {
    fn next(&'f self) -> &'f ListLink<'f, FlashUser<'f>> {
        &self.next
    }
}
