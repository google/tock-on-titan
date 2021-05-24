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
use kernel::hil::time::{self, Alarm, Frequency, Ticks};

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

pub struct Timels {
    registers: *const Registers,
    client: Cell<Option<&'static dyn time::AlarmClient>>,
    now: Cell<u32>,
}

impl Timels {
    const fn new(regs: *const Registers) -> Timels {
        Timels {
            registers: regs,
            client: Cell::new(None),
            now: Cell::new(0),
        }
    }

    pub fn handle_interrupt(&self) {
        let regs = unsafe { &*self.registers };
        regs.interrupt_ack.set(1);
        regs.interrupt_wakeup_ack.set(1);
        regs.control.set(0);
        self.now.set(self.now.get().wrapping_add(regs.reload.get()));
        regs.reload.set(0);
        self.client.get().map(|client| {
            client.alarm();
        });
    }
}

pub struct Freq256Khz;

impl Frequency for Freq256Khz {
    fn frequency() -> u32 {
        256000
    }
}

impl time::Time for Timels {
    type Frequency = Freq256Khz;
    type Ticks = time::Ticks32;

    fn now(&self) -> Self::Ticks {
        let regs = unsafe { &*self.registers };
        let cur = regs.value.get();
        let reload = regs.reload.get();
        let elapsed = reload - cur;
        self.now.get().wrapping_add(elapsed).into()
    }
}
    
impl Alarm<'static> for Timels {
    fn set_alarm(&self, reference: Self::Ticks, dt: Self::Ticks) {
        use crate::kernel::hil::time::Time;
        let regs = unsafe { &*self.registers };

        // Before doing anything else, stop the timer so it
        // doesn't expire on us before we can reset it.
        regs.control.set(0);

        // Re-computing now() here guarantees that we see
        // what the caller saw (or later) instead of the
        // potentially outdated `self.now` value.
        let now = self.now();

        // Compute the target alarm time, but check that it's
        // not before now. If it is, set it to a minimum to
        // ensure that it actually triggers "very soon".
        let target = reference.wrapping_add(dt);

        let distance: Self::Ticks;
        if target <= now {
            distance = 1.into();
        } else {
            distance = target.wrapping_sub(now);
        }

        regs.load.set(distance.into_u32());
        regs.reload.set(distance.into_u32());
        regs.interrupt_enable.set(1);
        regs.control.set(1);
    }

    fn get_alarm(&self) -> Self::Ticks {
        let regs = unsafe { &*self.registers };
        regs.reload.get().into()
    }

    fn set_alarm_client(&'static self, client: &'static dyn time::AlarmClient) {
        self.client.set(Some(client));
    }

    fn is_armed(&self) -> bool {
        let regs = unsafe { &*self.registers };
        regs.control.get() & 1 == 1 && regs.value.get() != 0
    }

    fn disarm(&self) -> kernel::ReturnCode {
        let regs = unsafe { &*self.registers };
        regs.control.set(0);
        kernel::ReturnCode::SUCCESS
    }

    fn minimum_dt(&self) -> Self::Ticks {
        1.into()
    }
}
