use core::mem;
use common::volatile_cell::VolatileCell;
use hil::alarm::{Alarm, Frequency, Freq16Khz};

#[repr(C, packed)]
struct Counter {
    enable: VolatileCell<u32>,
    oneshot_mode: VolatileCell<u32>,
    max_value: VolatileCell<u32>,
    programmed_value: VolatileCell<u32>,
    divider: VolatileCell<u32>,
    current_value: VolatileCell<u32>,
    current_divider_value: VolatileCell<u32>
}

#[repr(C, packed)]
struct Registers {
    _version: VolatileCell<u32>,
    interrupt_enable: VolatileCell<u32>,
    interrupt_clear: VolatileCell<u32>,
    _interrupt_test: VolatileCell<u32>,
    counters: [Counter; 4]
}

const BASE_REGISTERS: *const Registers = 0x40670000 as *const Registers;

pub struct Timeus {
    regs: &'static Registers,
    idx: usize
}

impl Timeus {
    pub unsafe fn new(idx: usize) -> Timeus {
        Timeus {
            regs: mem::transmute(BASE_REGISTERS),
            idx: idx
        }
    }

    fn counter(&self) -> &Counter {
        &&self.regs.counters[self.idx]
    }
}

impl Alarm for Timeus {

    type Frequency = Freq16Khz;

    fn now(&self) -> u32 {
        self.counter().current_value.get()
    }

    fn is_armed(&self) -> bool {
        self.counter().enable.get() != 0
    }

    fn disable_alarm(&self) {
        self.counter().enable.set(0);
    }

    fn get_alarm(&self) -> u32 {
        self.counter().programmed_value.get()
    }

    fn set_alarm(&self, alarm: u32) {
        let counter = self.counter();

        counter.enable.set(0);
        counter.oneshot_mode.set(0);
        counter.max_value.set(!0); // MAX_INT
        counter.programmed_value.set(alarm);
        counter.divider.set(24_000_000 / Self::Frequency::frequency());
        counter.current_value.set(0);
        counter.current_divider_value.set(0);
        counter.enable.set(1);

        self.regs.interrupt_enable.set(1 << (self.idx * 2));
    }
}

