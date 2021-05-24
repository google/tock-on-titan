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

use crate::gpio;
use crate::gpio::FloatingState;
use crate::gpio::GpioValue;
use crate::gpio::InterruptEdge;

use core::convert::TryFrom;

use libtock::result::TockResult;

/// GPIO pins and mapping to kernel number.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
#[allow(non_camel_case_types)]
pub enum GpioPin {
    BMC_SRST_N = 0,
    BMC_CPU_RST_N = 1,
    SYS_RSTMON_N = 2,
    BMC_RSTMON_N = 3,
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
}

static mut GPIO_CTRL: GpioControlImpl = GpioControlImpl {
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


impl GpioControlImpl {
    fn initialize(&'static mut self) -> TockResult<()> {
        gpio::get().enable_output(GpioPin::BMC_SRST_N as usize)?;
        gpio::get().enable_output(GpioPin::BMC_CPU_RST_N as usize)?;
        gpio::get().enable_input(GpioPin::SYS_RSTMON_N as usize, FloatingState::PullNone)?;
        gpio::get().enable_events(GpioPin::SYS_RSTMON_N as usize, InterruptEdge::RisingEdge)?;
        gpio::get().enable_input(GpioPin::BMC_RSTMON_N as usize, FloatingState::PullNone)?;
        gpio::get().enable_events(GpioPin::BMC_RSTMON_N as usize, InterruptEdge::RisingEdge)?;
        Ok(())
    }
}

impl GpioControl for GpioControlImpl {

    fn have_events(&self) -> bool {
        gpio::get().has_event(GpioPin::SYS_RSTMON_N as usize) ||
            gpio::get().has_event(GpioPin::BMC_RSTMON_N as usize)
    }

    fn consume_event(&self, pin: GpioPin) -> bool {
        gpio::get().consume_event(pin as usize)
    }

    fn clear_event(&self, pin: GpioPin) -> bool {
        gpio::get().clear_event(pin as usize)
    }

    fn set(&self, pin: GpioPin, val: GpioValue) -> TockResult<()> {
        gpio::get().write(pin as usize, val)
    }
}


