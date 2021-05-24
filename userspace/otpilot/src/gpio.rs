// Copyright 2021 lowRISC contributors.
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
//
// SPDX-License-Identifier: Apache-2.0

use core::cell::Cell;
use core::cmp::min;

use libtock::println;
use libtock::result::TockError;
use libtock::result::TockResult;
use libtock::syscalls;

const MAX_GPIOS: usize = 4;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
#[allow(dead_code)]
pub enum GpioValue {
    Low,
    High,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[allow(dead_code)]
pub enum FloatingState {
    PullNone = 0,
    PullUp = 1,
    PullDown = 2,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[allow(dead_code)]
pub enum InterruptEdge {
    EitherEdge = 0,
    RisingEdge = 1,
    FallingEdge = 2,
}

pub trait Gpio {
    /// Get the number of available GPIOs.
    fn get_num_gpios(&self) -> usize;

    /// Enable the specified GPIO for output.
    fn enable_output(&self, gpio_num: usize) -> TockResult<()>;

    /// Enable the specified GPIO for input.
    fn enable_input(&self, gpio_num: usize, floating_state: FloatingState) -> TockResult<()>;

    /// Disable the specified GPIO.
    fn disable(&self, gpio_num: usize) -> TockResult<()>;

    /// Write a value to the specified GPIO.
    fn write(&self, gpio_num: usize, val: GpioValue) -> TockResult<()>;

    /// Toggle the specified GPIO.
    fn toggle(&self, gpio_num: usize) -> TockResult<()>;

    /// Read the current value off the specified GPIO.
    fn read(&self, gpio_num: usize) -> TockResult<GpioValue>;

    /// Enable events to be received on the specified GPIO.
    fn enable_events(&self, gpio_num: usize, edge: InterruptEdge) -> TockResult<()>;

    /// Disable events to be received on the specified GPIO.
    fn disable_events(&self, gpio_num: usize, edge: InterruptEdge) -> TockResult<()>;

    /// Check if there are events to be consumed on the specified GPIO.
    fn has_event(&self, gpio_num: usize) -> bool;

    /// Consume one event on the specified GPIO.
    /// Returns true if there was an event to be consumed.
    fn consume_event(&self, gpio_num: usize) -> bool;

    /// Clear all events on the specified GPIO.
    /// Returns true if there was an event.
    fn clear_event(&self, gpio_num: usize) -> bool;
}

// Get the static Gpio object.
pub fn get() -> &'static dyn Gpio {
    get_impl()
}

const DRIVER_NUMBER: usize = 0x00004;

mod command_nr {
    pub const COUNT: usize = 0;
    pub const ENABLE_OUTPUT: usize = 1;
    pub const SET: usize = 2;
    pub const CLEAR: usize = 3;
    pub const TOGGLE: usize = 4;
    pub const ENABLE_INPUT: usize = 5;
    pub const READ: usize = 6;
    pub const INTERRUPT_ENABLE: usize = 7;
    pub const INTERRUPT_DISABLE: usize = 8;
    pub const DISABLE: usize = 9;
}

mod subscribe_nr {
    pub const SUBSCRIBE_CALLBACK: usize = 0;
}

struct GpioImpl {
    /// The number of GPIOs
    num_gpios: usize,
    events: [Option<Events>; MAX_GPIOS],
}

static mut GPIO: GpioImpl = {
    const NONE: Option<Events> = None;

    GpioImpl {
        num_gpios: 0,
        events: [NONE; MAX_GPIOS],
    }
};

static mut IS_INITIALIZED: bool = false;

fn get_impl() -> &'static GpioImpl {
    unsafe {
        if !IS_INITIALIZED {
            if GPIO.initialize().is_err() {
                panic!("Could not initialize Gpio");
            }
            IS_INITIALIZED = true;
        }
        &GPIO
    }
}

impl GpioImpl {
    fn initialize(&'static mut self) -> TockResult<()> {
        self.num_gpios = syscalls::command(DRIVER_NUMBER, command_nr::COUNT, 0, 0)?;

        if self.num_gpios > self.events.len() {
            println!("WARNING: The kernel reported {} GPIOs but we only support up to {}.",
                self.num_gpios, self.events.len())
        }

        for idx in 0..min(self.num_gpios, self.events.len()) {
            self.events[idx] = Some(Default::default())
        }

        syscalls::subscribe_fn(
            DRIVER_NUMBER,
            subscribe_nr::SUBSCRIBE_CALLBACK,
            GpioImpl::callback_trampoline,
            0)?;

        Ok(())
    }

    extern "C"
    fn callback_trampoline(arg1: usize, arg2: usize, _arg3: usize, _data: usize) {
        get_impl().callback(arg1, arg2);
    }

    fn callback(&self, gpio_num: usize, _pin_state: usize) {
        self.add_event(gpio_num)
    }

    fn add_event(&self, gpio_num: usize) {
        if gpio_num >= self.events.len() { return; }

        if let Some(events) = &self.events[gpio_num] {
            events.add()
        }
    }
}

impl Gpio for GpioImpl {
    fn get_num_gpios(&self) -> usize {
        self.num_gpios
    }

    fn enable_output(&self, gpio_num: usize) -> TockResult<()> {
        syscalls::command(DRIVER_NUMBER, command_nr::ENABLE_OUTPUT, gpio_num, 0)?;

        Ok(())
    }
    fn enable_input(&self, gpio_num: usize, floating_state: FloatingState) -> TockResult<()> {
        syscalls::command(DRIVER_NUMBER, command_nr::ENABLE_INPUT, gpio_num, floating_state as usize)?;

        Ok(())
    }
    fn disable(&self, gpio_num: usize) -> TockResult<()> {
        syscalls::command(DRIVER_NUMBER, command_nr::DISABLE, gpio_num, 0)?;

        Ok(())
    }
    fn write(&self, gpio_num: usize, val: GpioValue) -> TockResult<()> {
        match val {
            GpioValue::High => syscalls::command(DRIVER_NUMBER, command_nr::SET, gpio_num, 0)?,
            GpioValue::Low => syscalls::command(DRIVER_NUMBER, command_nr::CLEAR, gpio_num, 0)?,
        };

        Ok(())
    }
    fn toggle(&self, gpio_num: usize) -> TockResult<()> {
        syscalls::command(DRIVER_NUMBER, command_nr::TOGGLE, gpio_num, 0)?;

        Ok(())
    }
    fn read(&self, gpio_num: usize) -> TockResult<GpioValue> {
        match syscalls::command(DRIVER_NUMBER, command_nr::READ, gpio_num, 0)? {
            0 => Ok(GpioValue::Low),
            1 => Ok(GpioValue::High),
            _ => Err(TockError::Format),
        }
    }
    fn enable_events(&self, gpio_num: usize, edge: InterruptEdge) -> TockResult<()> {
        syscalls::command(DRIVER_NUMBER, command_nr::INTERRUPT_ENABLE, gpio_num, edge as usize)?;

        Ok(())
    }
    fn disable_events(&self, gpio_num: usize, edge: InterruptEdge) -> TockResult<()> {
        syscalls::command(DRIVER_NUMBER, command_nr::INTERRUPT_DISABLE, gpio_num, edge as usize)?;

        Ok(())
    }

    fn has_event(&self, gpio_num: usize) -> bool {
        if gpio_num >= self.events.len() { return false; }

        if let Some(events) = &self.events[gpio_num] {
            events.has_any()
        } else {
            false
        }
    }

    fn consume_event(&self, gpio_num: usize) -> bool {
        if gpio_num >= self.events.len() { return false; }

        if let Some(events) = &self.events[gpio_num] {
            events.consume() > 0
        } else {
            false
        }
    }

    fn clear_event(&self, gpio_num: usize) -> bool {
        if gpio_num >= self.events.len() { return false; }

        if let Some(events) = &self.events[gpio_num] {
            events.clear() > 0
        } else {
            false
        }
    }
}

#[derive(Default)]
struct Events {
    count: Cell<usize>,
}

impl Events {
    /// Adds one event.
    /// If the maximum number of events would be exceeded, do nothing.
    fn add(&self) {
        let val = self.count.get();
        if val < core::usize::MAX - 1 {
            self.count.set(val + 1);
        }
    }

    /// Consumes one event.
    /// Returns the number of events to consume before consuming one.
    fn consume(&self) -> usize {
        let val = self.count.get();
        if val > core::usize::MIN {
            self.count.set(val - 1);
        }
        val
    }

    /// Clears all events.
    /// Returns the number of events to consume before clearing them.
    fn clear(&self) -> usize {
        let val = self.count.get();
        self.count.set(core::usize::MIN);
        val
    }

    /// Checks if we have an event.
    /// Returns true if there's at least one event to be consumed.
    fn has_any(&self) -> bool {
        self.count.get() > core::usize::MIN
    }
}
