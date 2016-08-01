use core::mem;
use common::volatile_cell::VolatileCell;
use hil::alarm::{Alarm, Frequency};

#[repr(u32)]
#[derive(PartialEq, Eq)]
pub enum Enable {
    Disabled = 0,
    Enabled = 1
}

#[repr(u32)]
#[derive(PartialEq, Eq)]
pub enum Mode {
    Wrapping = 0,
    OneShot = 1
}

#[repr(C, packed)]
/// Registers for a counter in the Timeus controller
///
/// The Timeus controller has four counters that can be programmed
/// independently, each with its own set of registers. Each counter tics at
/// 24Mhz. On each tic the `current_divider_value` is updated. When
/// `current_divider_value` reaches the value of `divider`, `current_value` is
/// incremented. Thus, `current_value` can be set to tick at 24Mhz by setting
/// `divider` to 1 or at a slower frequency by setting divider to a non-zero
/// value. For example, setting divider to 24 would result in a frequency of
/// 1Mhz (or one tic per microsecond). The counter continues until
/// `current_value` reaches `max_value`.
///
/// A counter can operate in one-shot or wrapping mode. In one-shot mode, the
/// counter stops when it reaches `max_value`, while in wrapping mode it
/// resets and starts counting again.
///
/// In addition to the `max_value`, each counter has a `programmed_value`. When
/// the counter reaches the `programmed_value` it generates an interrupt but
/// continues counting up to `max_value`.
pub struct Counter {
    /// Enables/disables the counter. 0 for disable, 1 for enable.
    enable: VolatileCell<Enable>,

    /// Chooses between oneshot and wrapping mode
    mode: VolatileCell<Mode>,

    /// Sets the maximum value of the counter. In one-shot mode, the coutner
    /// stops when it reaches this value. In wrapping mode, it resets.
    max_value: VolatileCell<u32>,

    /// Sets the intermediate programmed value. If the counter reaches this
    /// value before reaching `max_value` and interrupt will be issued.
    programmed_value: VolatileCell<u32>,

    /// The counter divider
    divider: VolatileCell<u32>,

    /// The current value of the counter.
    current_value: VolatileCell<u32>,

    /// The current value of the divider. When this register reaches `divider`,
    /// `current_value` is incremented.
    current_divider_value: VolatileCell<u32>
}

#[repr(C, packed)]
pub struct Registers {
    /// Marks the version of the controller. Always reads as `0x800ea91`.
    _version: VolatileCell<u32>,

    /// Enable interrupts
    ///
    /// Each bit marks a different interrupt in groups of two, where each group is for a
    /// different counter (i.e. bits 0-1 are for counter 0, 2-3 for counter 2, etc.)
    ///
    /// The first bit (e.g. bit 0) enabled interrupts for the counter's programmed value.
    /// The second bit (e.g. bit 1) enabled interrupts for the counter's max value.
    pub interrupt_enable: VolatileCell<u32>,

    /// Clear interrupts
    ///
    /// Same mapping as `interrupt_enable`
    pub interrupt_clear: VolatileCell<u32>,

    _interrupt_test: VolatileCell<u32>,
    _reserved: [u8; 240],

    /// Registers for each of the four counters
    pub counters: [Counter; 4]
}

const BASE_REGISTERS: *const Registers = 0x40670000 as *const Registers;

pub struct Timeus {
    regs: &'static Registers,
    idx: usize
}

impl Timeus {
    /// Creates a new Timeus for a particular counter.
    ///
    /// It is unsafe to create multiple Timeus with the same `idx`.
    ///
    /// `idx` must betwee in the range [0, 3].
    pub unsafe fn new(idx: usize) -> Timeus {
        Timeus {
            regs: mem::transmute(BASE_REGISTERS),
            idx: idx
        }
    }

    fn counter(&self) -> &Counter {
        &self.regs.counters[self.idx]
    }
}

pub struct Freq24Mhz;

impl Frequency for Freq24Mhz {
    fn frequency() -> u32 {
        24_000_000
    }
}

impl Alarm for Timeus {

    type Frequency = Freq24Mhz;

    fn now(&self) -> u32 {
        self.counter().current_value.get()
    }

    fn is_armed(&self) -> bool {
        self.counter().enable.get() == Enable::Enabled
    }

    fn disable_alarm(&self) {
        self.counter().enable.set(Enable::Disabled);
    }

    fn get_alarm(&self) -> u32 {
        self.counter().programmed_value.get()
    }

    fn set_alarm(&self, alarm: u32) {
        let counter = self.counter();

        counter.enable.set(Enable::Disabled);
        counter.max_value.set(!0); // MAX_INT
        counter.divider.set(24_000_000 / Self::Frequency::frequency());
        counter.programmed_value.set(alarm);
        counter.mode.set(Mode::OneShot);

        counter.enable.set(Enable::Enabled);

        self.regs.interrupt_enable.set(1 << (self.idx * 2));
    }
}

