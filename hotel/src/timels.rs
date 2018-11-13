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

use core::cell::Cell;
use kernel::common::cells::VolatileCell;
use kernel::hil::time::{self, Alarm, Frequency};

const TIMELS0_BASE: *const Registers = 0x40540000 as *const Registers;
const TIMELS1_BASE: *const Registers = 0x40540040 as *const Registers;

pub static mut TIMELS0: Timels = Timels::new(TIMELS0_BASE);
pub static mut TIMELS1: Timels = Timels::new(TIMELS1_BASE);

struct Registers {
    pub control: VolatileCell<u32>,
    pub status: VolatileCell<u32>,
    pub load: VolatileCell<u32>,
    pub reload: VolatileCell<u32>,
    pub value: VolatileCell<u32>,
    pub step: VolatileCell<u32>,
    pub interrupt_enable: VolatileCell<u32>,
    pub interrupt_status: VolatileCell<u32>,
    pub interrupt_pending: VolatileCell<u32>,
    pub interrupt_ack: VolatileCell<u32>,
    pub interrupt_wakeup_ack: VolatileCell<u32>,
}

pub struct Timels<'a> {
    registers: *const Registers,
    client: Cell<Option<&'a time::Client>>,
    now: Cell<u32>,
}

impl<'a> Timels<'a> {
    const fn new(regs: *const Registers) -> Timels<'a> {
        Timels {
            registers: regs,
            client: Cell::new(None),
            now: Cell::new(0),
        }
    }

    pub fn set_client(&'static self, client: &'static time::Client) {
        self.client.set(Some(client));
    }

    pub fn handle_interrupt(&self) {
        let regs = unsafe { &*self.registers };
        regs.interrupt_ack.set(1);
        regs.interrupt_wakeup_ack.set(1);
        regs.control.set(0);
        self.now.set(self.now.get().wrapping_add(regs.reload.get()));
        regs.reload.set(0);
        self.client.get().map(|client| {
            client.fired();
        });
    }

    fn disable_alarm(&self) {
        let regs = unsafe { &*self.registers };
        regs.control.set(0);
    }

    fn is_enabled(&self) -> bool {
        let regs = unsafe { &*self.registers };
        regs.control.get() & 1 == 1 && regs.value.get() != 0
    }

    
}

pub struct Freq256Khz;

impl Frequency for Freq256Khz {
    fn frequency() -> u32 {
        256000
    }
}

impl<'a> time::Time for Timels<'a> {
    type Frequency = Freq256Khz;

    fn disable(&self) {
        self.disable_alarm();
    }

    fn is_armed(&self) -> bool {
        self.is_enabled()
    }
}
    
impl<'a> Alarm for Timels<'a> {

    fn now(&self) -> u32 {
        let regs = unsafe { &*self.registers };
        let cur = regs.value.get();
        let reload = regs.reload.get();
        let elapsed = reload - cur;
        self.now.get().wrapping_add(elapsed)
    }

    fn set_alarm(&self, tics: u32) {
        let distance = tics.wrapping_sub(self.now.get());
        let regs = unsafe { &*self.registers };
        regs.load.set(distance);
        regs.reload.set(distance);
        regs.interrupt_enable.set(1);
        regs.control.set(1);
    }

    fn get_alarm(&self) -> u32 {
        let regs = unsafe { &*self.registers };
        regs.reload.get()
    }
}
