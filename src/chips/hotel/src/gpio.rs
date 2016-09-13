use core::cell::Cell;
use common::volatile_cell::VolatileCell;
use common::take_cell::TakeCell;
use core::mem::transmute;
use hil::gpio::{Client, GPIOPin, InputMode, InterruptMode};

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
    pub interrupt_status: VolatileCell<u32>
}

pub const GPIO0_BASE: *mut PortRegisters = 0x40200000 as *mut PortRegisters;

pub struct Port {
    pub pins: [Pin; 16],
}

pub static mut PORT0: Port = Port {
    pins: [Pin::new(GPIO0_BASE, PinNum::P0),
           Pin::new(GPIO0_BASE, PinNum::P1),
           Pin::new(GPIO0_BASE, PinNum::P2),
           Pin::new(GPIO0_BASE, PinNum::P3),
           Pin::new(GPIO0_BASE, PinNum::P4),
           Pin::new(GPIO0_BASE, PinNum::P5),
           Pin::new(GPIO0_BASE, PinNum::P6),
           Pin::new(GPIO0_BASE, PinNum::P7),
           Pin::new(GPIO0_BASE, PinNum::P8),
           Pin::new(GPIO0_BASE, PinNum::P9),
           Pin::new(GPIO0_BASE, PinNum::P10),
           Pin::new(GPIO0_BASE, PinNum::P11),
           Pin::new(GPIO0_BASE, PinNum::P12),
           Pin::new(GPIO0_BASE, PinNum::P13),
           Pin::new(GPIO0_BASE, PinNum::P14),
           Pin::new(GPIO0_BASE, PinNum::P15)],
};

#[derive(Clone,Copy,Debug)]
pub enum PinNum {
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

pub struct Pin {
    port: *mut PortRegisters,
    pin: PinNum,
    client_data: Cell<usize>, 
    change: Cell<bool>, 
    client: TakeCell<&'static Client>,
}

impl Pin {
    const fn new(port: *mut PortRegisters, pin: PinNum) -> Pin {
        Pin {
            port: port,
            pin: pin,
            change: Cell::new(false),
            client_data: Cell::new(0),
            client: TakeCell::empty(),
        }
    }

    pub fn handle_interrupt(&self) {
        let mask = 1 << (self.pin as u32);

        let port: &mut PortRegisters = unsafe { transmute(self.port) };
        port.interrupt_status.set(mask);

        if self.change.get() {
            if port.interrupt_pol_set.get() & mask != 0 {
                port.interrupt_pol_clear.set(mask);
            } else {
                port.interrupt_pol_set.set(mask);
            }
        }

        self.client.map(|client| {
            client.fired(self.client_data.get());
        });
    }

    pub fn set_client(&self, client: &'static Client) {
        self.client.put(Some(client));
    }
}

impl GPIOPin for Pin {
    fn enable_output(&self) {
        let port: &mut PortRegisters = unsafe { transmute(self.port) };
        port.output_enable.set(1 << (self.pin as u32));
    }

    fn enable_input(&self, _mode: InputMode) {
        // Noop, input is always enabled on this chip

        // InputMode equivilant is actually set in the Pinmux. It actually kind of
        // makes sense to have this be something that's setup by the platform
        // initialization, rather than chosen by the client of a particular GPIO
        // pin.
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

    fn enable_interrupt(&self, identifier: usize, mode: InterruptMode) {
        self.client_data.set(identifier);

        let port: &mut PortRegisters = unsafe { transmute(self.port) };
        let mask = 1 << (self.pin as u32);
        match mode {
            InterruptMode::RisingEdge => {
                port.interrupt_pol_set.set(mask);
                self.change.set(false);
            },
            InterruptMode::FallingEdge => {
                port.interrupt_pol_clear.set(mask);
                self.change.set(false);
            },
            InterruptMode::Change => {
                self.change.set(true);
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

interrupt_handler!(gpio0_combined_handler, 81);

interrupt_handler!(gpio0_0_handler, 65);
interrupt_handler!(gpio0_1_handler, 66);
