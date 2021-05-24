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

use self::Pin::*;
use core::cell::Cell;
use core::mem::transmute;
use kernel::common::cells::VolatileCell;
use kernel::hil;

#[repr(C)]
pub struct PortRegisters {
    pub data_in: VolatileCell<u32>,
    pub data_out: VolatileCell<u32>,
    _reserved: [u32; 2],
    pub output_enable: VolatileCell<u32>,
    pub output_disable: VolatileCell<u32>,
    _reserved2: [u32; 2],
    pub interrupt_enable: VolatileCell<u32>,
    pub interrupt_disable: VolatileCell<u32>,
    pub interrupt_type_set: VolatileCell<u32>,
    pub interrupt_type_clear: VolatileCell<u32>,
    pub interrupt_pol_set: VolatileCell<u32>,
    pub interrupt_pol_clear: VolatileCell<u32>,
    pub interrupt_status: VolatileCell<u32>,
}

pub const GPIO0_BASE: *mut PortRegisters = 0x40200000 as *mut PortRegisters;
pub const GPIO1_BASE: *mut PortRegisters = 0x40210000 as *mut PortRegisters;

pub struct Port {
    pub pins: [GPIOPin; 16],
}

pub static mut PORT0: Port = Port {
    pins: [GPIOPin::new(GPIO0_BASE, P0),
           GPIOPin::new(GPIO0_BASE, P1),
           GPIOPin::new(GPIO0_BASE, P2),
           GPIOPin::new(GPIO0_BASE, P3),
           GPIOPin::new(GPIO0_BASE, P4),
           GPIOPin::new(GPIO0_BASE, P5),
           GPIOPin::new(GPIO0_BASE, P6),
           GPIOPin::new(GPIO0_BASE, P7),
           GPIOPin::new(GPIO0_BASE, P8),
           GPIOPin::new(GPIO0_BASE, P9),
           GPIOPin::new(GPIO0_BASE, P10),
           GPIOPin::new(GPIO0_BASE, P11),
           GPIOPin::new(GPIO0_BASE, P12),
           GPIOPin::new(GPIO0_BASE, P13),
           GPIOPin::new(GPIO0_BASE, P14),
           GPIOPin::new(GPIO0_BASE, P15)],
};

pub static mut PORT1: Port = Port {
    pins: [GPIOPin::new(GPIO1_BASE, P0),
           GPIOPin::new(GPIO1_BASE, P1),
           GPIOPin::new(GPIO1_BASE, P2),
           GPIOPin::new(GPIO1_BASE, P3),
           GPIOPin::new(GPIO1_BASE, P4),
           GPIOPin::new(GPIO1_BASE, P5),
           GPIOPin::new(GPIO1_BASE, P6),
           GPIOPin::new(GPIO1_BASE, P7),
           GPIOPin::new(GPIO1_BASE, P8),
           GPIOPin::new(GPIO1_BASE, P9),
           GPIOPin::new(GPIO1_BASE, P10),
           GPIOPin::new(GPIO1_BASE, P11),
           GPIOPin::new(GPIO1_BASE, P12),
           GPIOPin::new(GPIO1_BASE, P13),
           GPIOPin::new(GPIO1_BASE, P14),
           GPIOPin::new(GPIO1_BASE, P15)],
};

#[derive(Clone,Copy,Debug)]
pub enum Pin {
    P0 = 0,
    P1,
    P2,
    P3,
    P4,
    P5,
    P6,
    P7,
    P8,
    P9,
    P10,
    P11,
    P12,
    P13,
    P14,
    P15,
}

pub struct GPIOPin {
    port: *mut PortRegisters,
    pin: Pin,
    change: Cell<bool>,
    client: Cell<Option<&'static dyn hil::gpio::Client>>,
}

impl GPIOPin {
    const fn new(port: *mut PortRegisters, pin: Pin) -> GPIOPin {
        GPIOPin {
            port: port,
            pin: pin,
            change: Cell::new(false),
            client: Cell::new(None),
        }
    }

    pub fn handle_interrupt(&self) {
        let mask = 1 << (self.pin as u32);

        let port: &mut PortRegisters = unsafe { transmute(self.port) };
        port.interrupt_status.set(mask);

        // If our InterruptMode was `Change`, we need to flip the direction of
        // the interrupt polarity.
        if self.change.get() {
            if port.interrupt_pol_set.get() & mask != 0 {
                port.interrupt_pol_clear.set(mask);
            } else {
                port.interrupt_pol_set.set(mask);
            }
        }

        self.client.get().map(|client| {
            client.fired()
        });
    }

    // Returns the pinmux::Pin corresponding to this GPIO pin.
    fn get_pinmux_pin(&self) -> Option<&'static crate::pinmux::Pin> {
        let pinmux = unsafe { &*crate::pinmux::PINMUX };
        let peripheral = match (self.port, self.pin) {
            (GPIO0_BASE, Pin::P0 ) => &pinmux.gpio0_gpio0,
            (GPIO0_BASE, Pin::P1 ) => &pinmux.gpio0_gpio1,
            (GPIO0_BASE, Pin::P2 ) => &pinmux.gpio0_gpio2,
            (GPIO0_BASE, Pin::P3 ) => &pinmux.gpio0_gpio3,
            (GPIO0_BASE, Pin::P4 ) => &pinmux.gpio0_gpio4,
            (GPIO0_BASE, Pin::P5 ) => &pinmux.gpio0_gpio5,
            (GPIO0_BASE, Pin::P6 ) => &pinmux.gpio0_gpio6,
            (GPIO0_BASE, Pin::P7 ) => &pinmux.gpio0_gpio7,
            (GPIO0_BASE, Pin::P8 ) => &pinmux.gpio0_gpio8,
            (GPIO0_BASE, Pin::P9 ) => &pinmux.gpio0_gpio9,
            (GPIO0_BASE, Pin::P10) => &pinmux.gpio0_gpio10,
            (GPIO0_BASE, Pin::P11) => &pinmux.gpio0_gpio11,
            (GPIO0_BASE, Pin::P12) => &pinmux.gpio0_gpio12,
            (GPIO0_BASE, Pin::P13) => &pinmux.gpio0_gpio13,
            (GPIO0_BASE, Pin::P14) => &pinmux.gpio0_gpio14,
            (GPIO0_BASE, Pin::P15) => &pinmux.gpio0_gpio15,
            (_         , Pin::P0 ) => &pinmux.gpio1_gpio0,
            (_         , Pin::P1 ) => &pinmux.gpio1_gpio1,
            (_         , Pin::P2 ) => &pinmux.gpio1_gpio2,
            (_         , Pin::P3 ) => &pinmux.gpio1_gpio3,
            (_         , Pin::P4 ) => &pinmux.gpio1_gpio4,
            (_         , Pin::P5 ) => &pinmux.gpio1_gpio5,
            (_         , Pin::P6 ) => &pinmux.gpio1_gpio6,
            (_         , Pin::P7 ) => &pinmux.gpio1_gpio7,
            (_         , Pin::P8 ) => &pinmux.gpio1_gpio8,
            (_         , Pin::P9 ) => &pinmux.gpio1_gpio9,
            (_         , Pin::P10) => &pinmux.gpio1_gpio10,
            (_         , Pin::P11) => &pinmux.gpio1_gpio11,
            (_         , Pin::P12) => &pinmux.gpio1_gpio12,
            (_         , Pin::P13) => &pinmux.gpio1_gpio13,
            (_         , Pin::P14) => &pinmux.gpio1_gpio14,
            (_         , Pin::P15) => &pinmux.gpio1_gpio15,
        };
        let pinmux_pin = match peripheral.select.get() {
            crate::pinmux::SelectablePin::Diob7  => &pinmux.diob7,
            crate::pinmux::SelectablePin::Diob6  => &pinmux.diob6,
            crate::pinmux::SelectablePin::Diob5  => &pinmux.diob5,
            crate::pinmux::SelectablePin::Diob4  => &pinmux.diob4,
            crate::pinmux::SelectablePin::Diob3  => &pinmux.diob3,
            crate::pinmux::SelectablePin::Diob2  => &pinmux.diob2,
            crate::pinmux::SelectablePin::Diob1  => &pinmux.diob1,
            crate::pinmux::SelectablePin::Diob0  => &pinmux.diob0,
            crate::pinmux::SelectablePin::Dioa14 => &pinmux.dioa14,
            crate::pinmux::SelectablePin::Dioa13 => &pinmux.dioa13,
            crate::pinmux::SelectablePin::Dioa12 => &pinmux.dioa12,
            crate::pinmux::SelectablePin::Dioa11 => &pinmux.dioa11,
            crate::pinmux::SelectablePin::Dioa10 => &pinmux.dioa10,
            crate::pinmux::SelectablePin::Dioa9  => &pinmux.dioa9,
            crate::pinmux::SelectablePin::Dioa8  => &pinmux.dioa8,
            crate::pinmux::SelectablePin::Dioa7  => &pinmux.dioa7,
            crate::pinmux::SelectablePin::Dioa6  => &pinmux.dioa6,
            crate::pinmux::SelectablePin::Dioa5  => &pinmux.dioa5,
            crate::pinmux::SelectablePin::Dioa4  => &pinmux.dioa4,
            crate::pinmux::SelectablePin::Dioa3  => &pinmux.dioa3,
            crate::pinmux::SelectablePin::Dioa2  => &pinmux.dioa2,
            crate::pinmux::SelectablePin::Dioa1  => &pinmux.dioa1,
            crate::pinmux::SelectablePin::Dioa0  => &pinmux.dioa0,
            crate::pinmux::SelectablePin::Diom4  => &pinmux.diom4,
            crate::pinmux::SelectablePin::Diom3  => &pinmux.diom3,
            crate::pinmux::SelectablePin::Diom2  => &pinmux.diom2,
            crate::pinmux::SelectablePin::Diom1  => &pinmux.diom1,
            crate::pinmux::SelectablePin::Diom0  => &pinmux.diom0,
            _ => return None,
        };
        Some(pinmux_pin)
    }
}

impl hil::gpio::Configure for GPIOPin {
    fn configuration(&self) -> hil::gpio::Configuration {
        if self.is_output() {
            hil::gpio::Configuration::InputOutput
        } else {
            hil::gpio::Configuration::Input
        }
    }

    fn make_output(&self) -> hil::gpio::Configuration {
        let port: &mut PortRegisters = unsafe { transmute(self.port) };
        port.output_enable.set(1 << (self.pin as u32));
        hil::gpio::Configuration::InputOutput
    }

    fn disable_output(&self) -> hil::gpio::Configuration {
        let port: &mut PortRegisters = unsafe { transmute(self.port) };
        port.output_enable.set(1 << (self.pin as u32));
        hil::gpio::Configuration::Input
    }

    fn make_input(&self) -> hil::gpio::Configuration {
        // Noop, input is always enabled on this chip
        self.configuration()
    }

    fn disable_input(&self) -> hil::gpio::Configuration {
        // Noop, input is always enabled on this chip
        self.configuration()
    }

    fn deactivate_to_low_power(&self) {
        self.disable_output();
        self.set_floating_state(hil::gpio::FloatingState::PullNone);
    }

    fn set_floating_state(&self, state: hil::gpio::FloatingState) {
        use kernel::hil::gpio::FloatingState::{PullUp, PullDown, PullNone};
        if let Some(pin) = self.get_pinmux_pin() {
            // Flip the pulldown (3) and pullup (4) enable bits.
            match state {
                PullUp   => pin.control.set(pin.control.get() & !(1 << 3) | 1 << 4),
                PullDown => pin.control.set(pin.control.get() & !(1 << 4) | 1 << 3),
                PullNone => pin.control.set(pin.control.get() & !(1 << 3 | 1 << 4)),
            }
        }
    }

    fn floating_state(&self) -> hil::gpio::FloatingState {
        if let Some(pin) = self.get_pinmux_pin() {
            // Read the pulldown (3) and pullup (4) enable bits.
            let pulldown = pin.control.get() & 1 << 3 != 0;
            let pullup   = pin.control.get() & 1 << 4 != 0;
            return match (pullup, pulldown) {
                (true, false) => hil::gpio::FloatingState::PullUp,
                (false, true) => hil::gpio::FloatingState::PullDown,
                _             => hil::gpio::FloatingState::PullNone,
            };
        }
        hil::gpio::FloatingState::PullNone
    }

    fn is_input(&self) -> bool { true }

    fn is_output(&self) -> bool {
        let port: &mut PortRegisters = unsafe { transmute(self.port) };
        // Assumes that reading the output_enable registers indicates which
        // outputs are enabled -- this may or may not be tested.
        port.output_enable.get() & (1 << (self.pin as u32)) != 0
    }
}

impl hil::gpio::Input for GPIOPin {
    fn read(&self) -> bool {
        let port: &mut PortRegisters = unsafe { transmute(self.port) };
        port.data_in.get() & (1 << (self.pin as u32)) != 0
    }
}

impl hil::gpio::Output for GPIOPin {
    fn set(&self) {
        let port: &mut PortRegisters = unsafe { transmute(self.port) };
        let data_out = port.data_out.get();
        port.data_out.set(data_out | 1 << (self.pin as u32));
    }

    fn clear(&self) {
        let port: &mut PortRegisters = unsafe { transmute(self.port) };
        let data_out = port.data_out.get();
        port.data_out.set(data_out & !(1 << (self.pin as u32)));
    }

    fn toggle(&self) -> bool {
        let port: &mut PortRegisters = unsafe { transmute(self.port) };
        let data_out = port.data_out.get();
        let bit_to_flip = 1 << (self.pin as u32);
        let new_value = data_out ^ bit_to_flip;
        port.data_out.set(new_value);
        new_value & bit_to_flip != 0
    }
}

impl hil::gpio::Interrupt<'static> for GPIOPin {
    fn set_client(&self, client: &'static dyn hil::gpio::Client) {
        self.client.set(Some(client));
    }

    // `InterruptMode::Change` is not implemented in hardware, so we simulate it
    // in software. This could lead to missing events if a toggle happens before
    // we install the new events.
    fn enable_interrupts(&self, mode: hil::gpio::InterruptEdge) {
        use kernel::hil::gpio::Input;
        let port: &mut PortRegisters = unsafe { transmute(self.port) };
        let mask = 1 << (self.pin as u32);
        match mode {
            hil::gpio::InterruptEdge::RisingEdge => {
                port.interrupt_pol_set.set(mask);
                self.change.set(false);
            }
            hil::gpio::InterruptEdge::FallingEdge => {
                port.interrupt_pol_clear.set(mask);
                self.change.set(false);
            }
            hil::gpio::InterruptEdge::EitherEdge => {
                self.change.set(true);
                // Set the interrupt polarity based on whatever the current
                // state of the pin is.
                if self.read() {
                    port.interrupt_pol_clear.set(mask);
                } else {
                    port.interrupt_pol_set.set(mask);
                }
            }
        }
        port.interrupt_type_set.set(mask);
        port.interrupt_enable.set(mask);
    }

    fn disable_interrupts(&self) {
        let port: &mut PortRegisters = unsafe { transmute(self.port) };
        let mask = 1 << (self.pin as u32);
        port.interrupt_disable.set(mask);
    }

    fn is_pending(&self) -> bool {
        let port: &mut PortRegisters = unsafe { transmute(self.port) };
        port.interrupt_status.get() & (1 << (self.pin as u32)) != 0
    }
}

impl hil::gpio::Pin for GPIOPin {}
impl hil::gpio::InterruptPin<'static> for GPIOPin {}
