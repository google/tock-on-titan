use self::Pin::*;
use core::cell::Cell;
use core::mem::transmute;
use kernel::common::cells::VolatileCell;
use kernel::hil;

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
    client_data: Cell<usize>,
    change: Cell<bool>,
    client: Cell<Option<&'static hil::gpio::Client>>,
}

impl GPIOPin {
    const fn new(port: *mut PortRegisters, pin: Pin) -> GPIOPin {
        GPIOPin {
            port: port,
            pin: pin,
            change: Cell::new(false),
            client_data: Cell::new(0),
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
            client.fired(self.client_data.get());
        });
    }

    pub fn set_client(&self, client: &'static hil::gpio::Client) {
        self.client.set(Some(client));
    }
}

impl hil::gpio::Pin for GPIOPin {
    fn make_output(&self) {
        let port: &mut PortRegisters = unsafe { transmute(self.port) };
        port.output_enable.set(1 << (self.pin as u32));
    }

    fn make_input(&self) {
        // Noop, input is always enabled on this chip
    }

    fn disable(&self) {
        let port: &mut PortRegisters = unsafe { transmute(self.port) };
        port.output_disable.set(1 << (self.pin as u32));
    }

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

    fn toggle(&self) {
        let port: &mut PortRegisters = unsafe { transmute(self.port) };
        let data_out = port.data_out.get();
        port.data_out.set(data_out ^ 1 << (self.pin as u32));
    }

    fn read(&self) -> bool {
        let port: &mut PortRegisters = unsafe { transmute(self.port) };
        port.data_in.get() & (1 << (self.pin as u32)) != 0
    }

    // `InterruptMode::Change` is not implemented in hardware, so we simulate it
    // in software. This could lead to missing events if a toggle happens before
    // we install the new events.
    fn enable_interrupt(&self, identifier: usize, mode: hil::gpio::InterruptMode) {
        self.client_data.set(identifier);

        let port: &mut PortRegisters = unsafe { transmute(self.port) };
        let mask = 1 << (self.pin as u32);
        match mode {
            hil::gpio::InterruptMode::RisingEdge => {
                port.interrupt_pol_set.set(mask);
                self.change.set(false);
            }
            hil::gpio::InterruptMode::FallingEdge => {
                port.interrupt_pol_clear.set(mask);
                self.change.set(false);
            }
            hil::gpio::InterruptMode::EitherEdge => {
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

    fn disable_interrupt(&self) {
        let port: &mut PortRegisters = unsafe { transmute(self.port) };
        let mask = 1 << (self.pin as u32);
        port.interrupt_disable.set(mask);
    }
}

impl hil::gpio::PinCtl for GPIOPin {
    // InputMode equivilent is set in the Pinmux.
    fn set_input_mode(&self, mode: hil::gpio::InputMode) {
        match mode {
            hil::gpio::InputMode::PullUp => {
            }
            hil::gpio::InputMode::PullDown => {
            }
            hil::gpio::InputMode::PullNone => {
            }
        }
    }
}
