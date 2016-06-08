use core::mem::transmute;
use common::volatile_cell::VolatileCell;

pub struct PortRegisters {
    pub data_in: VolatileCell<u32>,
    pub data_out: VolatileCell<u32>,
    _reserved : [u32; 2],
    pub output_enable: VolatileCell<u32>,
    pub output_disable: VolatileCell<u32>,
}

pub const GPIO0_BASE : *mut PortRegisters = 0x40200000 as *mut PortRegisters;

pub struct Port {
    pub pins: [GPIOPin; 16]
}

#[derive(Clone,Copy)]
pub enum Pin {
    P0 = 0, P1, P2, P3, P4, P5, P6, P7, P8, P9, P10, P11, P12, P13, P14, P15
}

pub struct GPIOPin {
    port: *mut PortRegisters,
    pin: Pin
}

impl GPIOPin {
    pub const unsafe fn new(port: *mut PortRegisters, pin: Pin) -> GPIOPin {
        GPIOPin { port: port, pin: pin }
    }

    pub fn enable_output(&self) {
        let port : &mut PortRegisters = unsafe { transmute(self.port) };
        port.output_enable.set(1 << (self.pin as u32));
    }

    pub fn disable_output(&self) {
        let port : &mut PortRegisters = unsafe { transmute(self.port) };
        port.output_disable.set(1 << (self.pin as u32));
    }

    pub fn set(&self) {
        let port : &mut PortRegisters = unsafe { transmute(self.port) };
        let data_out = port.data_out.get();
        port.data_out.set(data_out | 1 << (self.pin as u32));
    }

    pub fn clear(&self) {
        let port : &mut PortRegisters = unsafe { transmute(self.port) };
        let data_out = port.data_out.get();
        port.data_out.set(data_out & !(1 << (self.pin as u32)));
    }

    pub fn toggle(&self) {
        let port : &mut PortRegisters = unsafe { transmute(self.port) };
        let data_out = port.data_out.get();
        port.data_out.set(data_out ^ 1 << (self.pin as u32));
    }
}

