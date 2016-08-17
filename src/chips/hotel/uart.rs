use common::take_cell::TakeCell;
use common::volatile_cell::VolatileCell;
use pmu::{Clock, PeripheralClock, PeripheralClock1};

pub struct Registers {
    pub read_data: VolatileCell<u32>,
    pub write_data: VolatileCell<u32>,
    pub nco: VolatileCell<u32>,
    pub control: VolatileCell<u32>,
    pub interrupt_control: VolatileCell<u32>,
    pub state: VolatileCell<u32>,
    pub clear_state: VolatileCell<u32>
}

const UART0_BASE: *mut Registers = 0x40600000 as *mut Registers;
const UART1_BASE: *mut Registers = 0x40610000 as *mut Registers;
const UART2_BASE: *mut Registers = 0x40620000 as *mut Registers;

pub static mut UART0: UART = unsafe {
    UART::new(UART0_BASE, PeripheralClock1::Uart0Timer)
};

pub static mut UART1: UART = unsafe {
    UART::new(UART1_BASE, PeripheralClock1::Uart1Timer)
};

pub static mut UART2: UART = unsafe {
    UART::new(UART2_BASE, PeripheralClock1::Uart2Timer)
};

enum EitherBytes {
    Immutable(&'static [u8]),
    Mutable(&'static mut [u8])
}

struct Buffer {
    bytes: EitherBytes,
    cursor: usize
}

pub struct UART {
    regs: *mut Registers,
    clock: Clock,
    buffer: TakeCell<Buffer>
}

impl UART {
    pub const unsafe fn new(uart: *mut Registers, clock: PeripheralClock1) -> UART {
        UART {
            regs: uart,
            clock: Clock::new(PeripheralClock::Bank1(clock)),
            buffer: TakeCell::empty()
        }
    }

    fn enable_tx(&self) {
        let regs = unsafe { &*self.regs };

        self.clock.enable();

        let ctrl = regs.control.get() | 0b1;
        regs.control.set(ctrl);
    }

    fn disable_tx(&self) {
        let regs = unsafe { &*self.regs };

        let ctrl = regs.control.get() & !(0b1);
        regs.control.set(ctrl);

        if ctrl & 0b11 == 0 {
            // Neither TX nor RX enabled anymore
            self.clock.disable();
        }
    }

    pub fn set_baudrate(&self, baudrate: u32) {
        let regs = unsafe { &*self.regs };

        // NCO is 2**20 * f_baud / f_pclk
        // f_pclk is 24_000_000 (24Mhz)
        // To avoid overflow, we use 2**14 * f_baud / (24Mhz / 2**6)
        let nco = (1 << 14) * baudrate / 375000;
        regs.nco.set(nco);
    }

    /// Send an array of bytes synchronously over the UART
    ///
    /// This method is marked unsafe because you shouldn't use it, in general.
    /// Sending bytes synchronously over UART takes forever and will result in
    /// missed interrupts. For example, at 115200 baud rate, each byte takes
    /// ~69 micrseconds to send.
    ///
    /// As a result, this method also takes some liberties enabling/disabling
    /// the TX and doesn't check if there is a pending operation.
    pub unsafe fn send_bytes_sync(&self, bytes: &[u8]) {
        let regs = &*self.regs;

        self.enable_tx();

        for b in bytes {
            while regs.state.get() & 1 != 0 {}
            regs.write_data.set(*b as u32);
        }

        while regs.state.get() & (1 << 5 | 1 << 4) != 0b110000 {}
        self.disable_tx();
    }

    fn send_remaining_bytes(&self) -> usize {
        let regs = unsafe { &*self.regs };

        self.enable_tx();

        self.buffer.map(|buffer| {
            let init_cursor = buffer.cursor;
            let bytes: &[u8] = match buffer.bytes {
                EitherBytes::Immutable(ref b) => b,
                EitherBytes::Mutable(ref b) => &**b
            };
            for b in bytes[buffer.cursor..].iter() {
                if regs.state.get() & 1 == 1 {
                    break; // TX Bufer full, wait for event that it's ready
                }
                buffer.cursor += 1;
                regs.write_data.set(*b as u32);
            }
            regs.interrupt_control.set(regs.interrupt_control.get() | 1);
            unsafe {
                ::cortexm3::nvic::enable(177);
            }
            buffer.cursor - init_cursor
        }).unwrap_or(0)

    }

    pub fn send_mut_bytes(&self, bytes: &'static mut [u8]) {
        self.buffer.replace(Buffer{bytes: EitherBytes::Mutable(bytes), cursor: 0});
        self.send_remaining_bytes();
    }

    pub fn send_bytes(&self, bytes: &'static [u8]) {
        self.buffer.replace(Buffer{bytes: EitherBytes::Immutable(bytes), cursor: 0});
        self.send_remaining_bytes();
    }
}

interrupt_handler!(uart0_handler, 177);
interrupt_handler!(uart1_handler, 184);
interrupt_handler!(uart2_handler, 191);

