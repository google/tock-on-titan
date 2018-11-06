//! Driver for the UART controllers capable of sending and receiving data
//! asynchronously.
//!
//! The UART has a configurable baud rate and can run with or without hardware
//! flow-control. There is no DMA for the UART, but it has a 32-character deep
//! FIFO transmit and receive buffer.
//!
//! # Examples
//!
//! Before using the UART you must configure the TX and/or RX pins and set the
//! baud rate:w
//!
//! ```
//! let uart = &hotel::uart::UART0;
//! let pinmux = unsafe { &mut *hotel::pinmux::PINMUX };
//! pinmux.dioa0.select.set(hotel::pinmux::Function::Uart0Tx);
//! uart.config(115200);
//! uart.enable_tx();
//! ```
//! Then, you can (unsafely) send bytes synchronously (e.g. for debugging)
//!
//! ```
//! uart.send_bytes_sync("Debug string".as_bytes());
//! ```
//!
//! Or asynchornously:
//!
//! ```
//! uart.send_bytes("Pretend I'm a very long string").as_bytes());
//! ```
//! you'll be notified of completion through a callback
//!
//! TODO
//!

use core::cell::Cell;
use kernel::common::cells::TakeCell;
use kernel::common::cells::VolatileCell;
use kernel::hil;
use kernel::ReturnCode;
use pmu::{Clock, PeripheralClock, PeripheralClock1};

/// Registers for the UART controller
#[allow(dead_code)]
struct Registers {
    read_data: VolatileCell<u32>,
    write_data: VolatileCell<u32>,
    nco: VolatileCell<u32>,
    control: VolatileCell<u32>,
    interrupt_control: VolatileCell<u32>,
    state: VolatileCell<u32>,
    clear_state: VolatileCell<u32>,
    interrupt_state: VolatileCell<u32>,
    clear_interrupt_state: VolatileCell<u32>,
}

const UART0_BASE: *mut Registers = 0x40600000 as *mut Registers;
const UART1_BASE: *mut Registers = 0x40610000 as *mut Registers;
const UART2_BASE: *mut Registers = 0x40620000 as *mut Registers;

pub static mut UART0: UART = unsafe { UART::new(UART0_BASE, PeripheralClock1::Uart0Timer) };

pub static mut UART1: UART = unsafe { UART::new(UART1_BASE, PeripheralClock1::Uart1Timer) };

pub static mut UART2: UART = unsafe { UART::new(UART2_BASE, PeripheralClock1::Uart2Timer) };

// A resumable buffer that tracks the last written index
//struct Buffer {
//    bytes: &'static mut [u8],
//    cursor: usize,
//    limit: usize,
//}

/// A UART channel
///
/// Each UART manages its own clock and NVIC interrupt internally.
pub struct UART {
    regs: *mut Registers,
    clock: Clock,
    tx_buffer: TakeCell<'static, [u8]>,
    tx_limit: Cell<usize>,
    tx_cursor: Cell<usize>,
    client: Cell<Option<&'static hil::uart::Client>>,
}

impl UART {
    const unsafe fn new(uart: *mut Registers, clock: PeripheralClock1) -> UART {
        UART {
            regs: uart,
            clock: Clock::new(PeripheralClock::Bank1(clock)),
            tx_buffer: TakeCell::empty(),
            tx_limit: Cell::new(0),
            tx_cursor: Cell::new(0),
            client: Cell::new(None),
        }
    }

    fn init(&self, params: hil::uart::UARTParameters) {
        self.config(params.baud_rate);
        // TODO(alevy) can we handle other parameters?
    }
    
    /// Enables transmission on the UART
    ///
    /// Side-effect: ensures the clock is on.
    pub fn enable_tx(&self) {
        let regs = unsafe { &*self.regs };

        self.clock.enable();

        let ctrl = regs.control.get() | 0b1;
        regs.control.set(ctrl);
    }

    /// Disable transmission on the UART
    ///
    /// Side-effect: turns the clock off if RX is also disabled.
    pub fn disable_tx(&self) {
        let regs = unsafe { &*self.regs };

        let ctrl = regs.control.get() & !(0b1);
        regs.control.set(ctrl);

        if ctrl & 0b11 == 0 {
            // Neither TX nor RX enabled anymore
            self.clock.disable();
        }
    }

    /// Enables reception on the UART
    ///
    /// Side-effect: ensures the clock is on.
    pub fn enable_rx(&self) {
        let regs = unsafe { &*self.regs };

        self.clock.enable();

        let ctrl = regs.control.get() | 0b10;
        regs.control.set(ctrl);
        regs.interrupt_control.set(regs.interrupt_control.get() | 2);
    }

    /// Disable reception on the UART
    ///
    /// Side-effect: turns the clock off if TX is also disabled.
    pub fn disable_rx(&self) {
        let regs = unsafe { &*self.regs };

        let ctrl = regs.control.get() & !(0b10);
        regs.control.set(ctrl);

        if ctrl & 0b11 == 0 {
            // Neither TX nor RX enabled anymore
            self.clock.disable();
        }
        regs.interrupt_control.set(regs.interrupt_control.get() & !2);
    }


    /// Prepare the UART for operation
    ///
    /// `baudrate` is specified in Hz (e.g. 9600, 115200).
    // TODO: Allow specification of other parameters like hardware flow control,
    // parity, etc.
    pub fn config(&self, baudrate: u32) {
        let regs = unsafe { &*self.regs };

        // NCO is 2**20 * f_baud / f_pclk
        // f_pclk is 24_000_000 (24Mhz)
        // To avoid overflow, we use 2**14 * f_baud / (24Mhz / 2**6)
        let nco = (1 << 14) * baudrate / 375000;
        regs.nco.set(nco);

        regs.clear_interrupt_state.set(!0);
        regs.state.set(!0);
    }

    /// Send an array of bytes synchronously over the UART
    ///
    /// # Safety
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
    }

    // Call this function when there might be bytes left in the `buffer` to
    // send. Writes bytes out to the TX FIFO until there are no bytes left, or
    // the FIFO is full. If any bytes _were_ written, it will enable the TX
    // interrupt, which will fire when number of bytes in the FIFO drops to a
    // certain threshold (determined by the `fifo` register and defaults to 1
    // byte).
    //
    // Returns the number of bytes written.
    fn send_remaining_bytes(&self) -> usize {
        let regs = unsafe { &*self.regs };

        // If there is no current buffer, just return zero. Probably shouldn't
        // happen though.
        let nwritten = self.tx_buffer.map(|bytes| {
            let init_cursor = self.tx_cursor.get();
            let limit = self.tx_limit.get();
            
            for b in bytes[init_cursor..limit].iter() {
                if regs.state.get() & 1 == 1 {
                    break; // TX Buffer full, we'll continue later
                }
                self.tx_cursor.set(self.tx_cursor.get() + 1);
                regs.write_data.set(*b as u32);
            }
            self.tx_cursor.get() - init_cursor
        })
            .unwrap_or(0);
        
        if nwritten > 0 {
            // if we wrote anything, we're gonna want to get notified when the FIFO has room again.
            // Technically we could be done here if there is nothing left to send, but we want to
            // get an interrupt anyway so we can return the buffer to the client from
            // `handle_tx_interrupt`.
            regs.interrupt_control.set(regs.interrupt_control.get() | 1);
        }

        nwritten

    }

    /// Called by the chip following a TX interrupt.
    ///
    /// If there are bytes left in the buffer to send, write another batch to the TX FIFO.
    /// Otherwise, return the buffer back to the client.
    ///
    /// # Invariants
    ///
    ///   * NVIC is disabled
    ///
    ///   * NVIC pending bit is high
    ///
    pub fn handle_tx_interrupt(&self) {
        let regs = unsafe { &*self.regs };

        regs.clear_interrupt_state.set(1);
        if self.send_remaining_bytes() == 0 {
            self.client.get().map(|client| {
                if self.tx_buffer.is_some() {
                    client.transmit_complete(self.tx_buffer.take().unwrap(), hil::uart::Error::CommandComplete);
                }
//                }
//                self.tx_buffer.map(|buffer| {
//                    client.transmit_complete(buffer, hil::uart::Error::CommandComplete);
//                });
            });
        }
    }

    /// Called by the chip following a RX interrupt.
    ///
    /// This will clear the NVIC pending bit to mark that we've handled the
    /// interrupt. Then, if there are bytes left in the buffer to send, write
    /// another batch to the TX FIFO. Otherwise, return the buffer back to the
    /// client (TODO: no client yet, so not yet implemented).
    ///
    /// # Invariants
    ///
    ///   * NVIC is disabled
    ///
    ///   * NVIC pending bit is high
    ///
    pub fn handle_rx_interrupt(&self) {
        let regs = unsafe { &*self.regs };
        // Currently discards bytes: need to read into buffer. -pal 4/11/18
        regs.clear_interrupt_state.set(2);
        self.client.get().map(|_client| {
            while regs.state.get() & 1 << 7 == 0 {
                // While RX FIFO not empty
                let _b = regs.read_data.get() as u8;
//                client.receive_complete(b, hil::uart::Error::CommandComplete);
            }
        });
    }
}

impl hil::uart::UART for UART {
    fn set_client(&self, client: &'static hil::uart::Client) {
        self.client.set(Some(client));
    }
    
    fn transmit(&self, tx_buffer: &'static mut [u8], tx_len: usize) {
        self.tx_buffer.replace(tx_buffer);
        self.tx_cursor.set(0);
        self.tx_limit.set(tx_len);
        self.send_remaining_bytes();
    }

    fn receive(&self, _rx_buffer: &'static mut[u8], _rx_len: usize) {
        unimplemented!();
    }

    fn abort_receive(&self) {
        unimplemented!();
    }

    fn configure(&self, params: hil::uart::UARTParameters) -> ReturnCode {
        self.config(params.baud_rate);
        ReturnCode::SUCCESS
    }
}
