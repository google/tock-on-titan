
use common::volatile_cell::VolatileCell;
use core::mem::transmute;
use hil;

pub struct PortRegisters {
    pub data_in: VolatileCell<u32>,
    pub data_out: VolatileCell<u32>,
    _reserved: [u32; 2],
    pub output_enable: VolatileCell<u32>,
    pub output_disable: VolatileCell<u32>,
}

pub const GPIO0_BASE: *mut PortRegisters = 0x40200000 as *mut PortRegisters;

pub struct Port {
    pub pins: [GPIOPin; 16],
}

pub static mut PORT0: Port = Port {
    pins: [GPIOPin::new(GPIO0_BASE, Pin::P0),
           GPIOPin::new(GPIO0_BASE, Pin::P1),
           GPIOPin::new(GPIO0_BASE, Pin::P2),
           GPIOPin::new(GPIO0_BASE, Pin::P3),
           GPIOPin::new(GPIO0_BASE, Pin::P4),
           GPIOPin::new(GPIO0_BASE, Pin::P5),
           GPIOPin::new(GPIO0_BASE, Pin::P6),
           GPIOPin::new(GPIO0_BASE, Pin::P7),
           GPIOPin::new(GPIO0_BASE, Pin::P8),
           GPIOPin::new(GPIO0_BASE, Pin::P9),
           GPIOPin::new(GPIO0_BASE, Pin::P10),
           GPIOPin::new(GPIO0_BASE, Pin::P11),
           GPIOPin::new(GPIO0_BASE, Pin::P12),
           GPIOPin::new(GPIO0_BASE, Pin::P13),
           GPIOPin::new(GPIO0_BASE, Pin::P14),
           GPIOPin::new(GPIO0_BASE, Pin::P15)],
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

#[derive(Debug)]
pub struct GPIOPin {
    port: *mut PortRegisters,
    pin: Pin,
}

impl GPIOPin {
    const fn new(port: *mut PortRegisters, pin: Pin) -> GPIOPin {
        GPIOPin {
            port: port,
            pin: pin,
        }
    }
}

impl hil::gpio::GPIOPin for GPIOPin {
    fn enable_output(&self) {
        let port: &mut PortRegisters = unsafe { transmute(self.port) };
        port.output_enable.set(1 << (self.pin as u32));
    }

    fn enable_input(&self, _mode: hil::gpio::InputMode) {
        // TODO(alevy): implement
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
        // TODO(alevy): implement
        false
    }

    fn enable_interrupt(&self, _identifier: usize, _mode: hil::gpio::InterruptMode) {
        // TODO(alevy): implement
    }

    fn disable_interrupt(&self) {
        // TODO(alevy): implement
    }
}
