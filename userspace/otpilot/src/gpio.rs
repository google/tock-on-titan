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
use core::convert::TryFrom;

use libtock::gpio::GpioPinUnitialized;
use libtock::gpio::GpioPinRead;
use libtock::gpio::GpioPinWrite;
use libtock::gpio::{IrqMode, InputMode};
use libtock::result::TockError;
use libtock::result::TockResult;

/// GPIO pins.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
#[allow(non_camel_case_types)]
pub enum GpioPin {
    BMC_SRST_N = 0,
    BMC_CPU_RST_N = 1,
    SYS_RSTMON_N = 2,
    BMC_RSTMON_N = 3,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
#[allow(dead_code)]
pub enum GpioValue {
    Low,
    High,
}

pub trait GpioControl {
    /// Check if there are any events to be consumed.
    fn have_events(&self) -> bool;

    /// Consume one event on the specified pin.
    /// Returns true if there was an event to be consumed.
    fn consume_event(&self, pin: GpioPin) -> bool;

    /// Clear all events on the specified pin.
    /// Returns true if there was an event.
    fn clear_event(&self, pin: GpioPin) -> bool;

    /// Set GpioPin value.
    fn set(&self, pin: GpioPin, val: GpioValue) -> TockResult<()>;
}

// Get the static GpioControl object.
pub fn get() -> &'static dyn GpioControl {
    get_impl()
}

/// Error for invalid GpioPin conversion.
pub struct InvalidGpioPin;

impl TryFrom<usize> for GpioPin {
    type Error = InvalidGpioPin;

    fn try_from(item: usize) -> Result<GpioPin, Self::Error> {
        match item {
            0 => Ok(GpioPin::BMC_SRST_N),
            1 => Ok(GpioPin::BMC_CPU_RST_N),
            2 => Ok(GpioPin::SYS_RSTMON_N),
            3 => Ok(GpioPin::BMC_RSTMON_N),
            _ => Err(InvalidGpioPin),
        }
    }
}

impl From<GpioPin> for usize {
    fn from(item: GpioPin) -> usize {
        item as usize
    }
}


struct GpioControlImpl {
    bmc_srst_n: Option<GpioPinWrite>,
    bmc_cpu_rst_n: Option<GpioPinWrite>,
    sys_rstmon_n: Option<GpioPinRead>,
    bmc_rstmon_n: Option<GpioPinRead>,

    sys_rstmon_n_events: Cell<usize>,
    bmc_rstmon_n_events: Cell<usize>,
}

static mut GPIO_CTRL: GpioControlImpl = GpioControlImpl {
    bmc_srst_n: None,
    bmc_cpu_rst_n: None,
    sys_rstmon_n: None,
    bmc_rstmon_n: None,
    sys_rstmon_n_events: Cell::new(0),
    bmc_rstmon_n_events: Cell::new(0),
};

static mut IS_INITIALIZED: bool = false;

fn get_impl() -> &'static GpioControlImpl {
    unsafe {
        if !IS_INITIALIZED {
            if GPIO_CTRL.initialize().is_err() {
                panic!("Could not initialize GPIO Control");
            }
            IS_INITIALIZED = true;
        }
        &GPIO_CTRL
    }
}


fn add_event(events: &Cell<usize>) {
    let val = events.get();
    events.set(val + 1);
}

fn delete_event(events: &Cell<usize>) -> usize {
    let val = events.get();
    if val > 0 {
        events.set(val - 1);
    }
    val
}

fn clear_event(events: &Cell<usize>) -> usize {
    let val = events.get();
    if val > 0 {
        events.set(0);
    }
    val
}

impl GpioControlImpl {
    fn initialize(&'static mut self) -> TockResult<()> {
        self.bmc_srst_n = Some(GpioPinUnitialized::new(GpioPin::BMC_SRST_N as usize).open_for_write()?);
        self.bmc_cpu_rst_n = Some(GpioPinUnitialized::new(GpioPin::BMC_CPU_RST_N as usize).open_for_write()?);
        self.sys_rstmon_n = Some(GpioPinUnitialized::new(GpioPin::SYS_RSTMON_N as usize).open_for_read(
            Some((GpioControlImpl::event_trampoline, IrqMode::RisingEdge)),
            InputMode::PullNone)?);
        self.bmc_rstmon_n = Some(GpioPinUnitialized::new(GpioPin::BMC_RSTMON_N as usize).open_for_read(
            Some((GpioControlImpl::event_trampoline, IrqMode::RisingEdge)),
            InputMode::PullNone)?);
        Ok(())
    }

    extern "C" fn event_trampoline(pin_num: usize, pin_state: usize, _: usize, _: usize) {
        get_impl().event(pin_num, pin_state);
    }

    fn event(&self, pin_num: usize, _pin_state: usize) {
        let pin = match GpioPin::try_from(pin_num) {
            Ok(val) => val,
            Err(_) => return,
        };

        match pin {
            GpioPin::SYS_RSTMON_N => add_event(&self.sys_rstmon_n_events),
            GpioPin::BMC_RSTMON_N => add_event(&self.bmc_rstmon_n_events),
            _ => (),
        };
    }

    fn set_value(&self, maybe_pin: &Option<GpioPinWrite>, val: GpioValue) -> TockResult<()> {
        if let Some(pin) = maybe_pin {
            match val {
                GpioValue::Low => pin.set_low(),
                GpioValue::High => pin.set_high(),
            }
        } else {
            Err(TockError::Format)
        }
    }
}

impl GpioControl for GpioControlImpl {

    fn have_events(&self) -> bool {
        self.sys_rstmon_n_events.get() != 0 || self.bmc_rstmon_n_events.get() != 0
    }

    fn consume_event(&self, pin: GpioPin) -> bool {
        let val = match pin {
            GpioPin::SYS_RSTMON_N => delete_event(&self.sys_rstmon_n_events),
            GpioPin::BMC_RSTMON_N => delete_event(&self.bmc_rstmon_n_events),
            _ => 0,
        };
        val != 0
    }

    fn clear_event(&self, pin: GpioPin) -> bool {
        let val = match pin {
            GpioPin::SYS_RSTMON_N => clear_event(&self.sys_rstmon_n_events),
            GpioPin::BMC_RSTMON_N => clear_event(&self.bmc_rstmon_n_events),
            _ => 0,
        };
        val != 0
    }

    fn set(&self, pin: GpioPin, val: GpioValue) -> TockResult<()> {
        match pin {
            GpioPin::BMC_SRST_N => self.set_value(&self.bmc_srst_n, val),
            GpioPin::BMC_CPU_RST_N => self.set_value(&self.bmc_cpu_rst_n, val),
            _ => Ok(()),
        }
    }
}
