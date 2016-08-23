use pmu::{Clock, PeripheralClock, PeripheralClock1};

mod constants;
mod registers;

use self::constants::*;
use self::registers::Registers;

struct StaticRef<T> {
    ptr: *const T
}

impl<T> StaticRef<T> {
    pub const unsafe fn new(ptr: *const T) -> StaticRef<T> {
        StaticRef { ptr: ptr }
    }
}

pub struct USB {
    registers: StaticRef<Registers>,
    core_clock: Clock,
    timer_clock: Clock
}

const BASE_ADDR: *const Registers = 0x40300000 as *const Registers;

pub static mut USB0: USB = unsafe { USB::new() };

impl<T> ::core::ops::Deref for StaticRef<T> {
    type Target = T;
    fn deref(&self) -> &'static T {
        unsafe { &*self.ptr }
    }

}

impl USB {

    pub const unsafe fn new() -> USB {
        USB {
            registers: StaticRef::new(BASE_ADDR),
            core_clock: Clock::new(PeripheralClock::Bank1(
                    PeripheralClock1::Usb0)),
            timer_clock: Clock::new(PeripheralClock::Bank1(
                    PeripheralClock1::Usb0TimerHs))
        }
    }

    pub fn handle_interrupt(&self) {
        // Save current interrupt status snapshot so can clear only those at the
        // end
        let status = self.registers.interrupt_status.get();

        if status & USB_RESET != 0 {
            println!("USB RESET");
            // 1. Set the nak bit for all OUT endpoitns
            // TODO(alevy): do we actually need to do this for _all_ endpoints?
            //              or just those we intend to enable?
            for ctrl in self.registers.out_endpoints.iter().map(|e|&e.control) {
                ctrl.set(1 << 27);
            }

            // 2. Unmask control 0 IN/OUT
            self.registers.device_all_ep_interrupt_mask.set(
                self.registers.device_all_ep_interrupt_mask.get() |
                1 << 0  | // IN 0
                1 << 16); // OUT 0
            //    Device OUT SETUP & XferCompl
            self.registers.device_out_ep_interrupt_mask.set(
                self.registers.device_out_ep_interrupt_mask.get() |
                1 << 0 | // XferCompl
                1 << 3); // SETUP
            //    Device IN XferCompl & TimeOut
            self.registers.device_in_ep_interrupt_mask.set(
                self.registers.device_in_ep_interrupt_mask.get() |
                1 << 0 | // XferCompl
                1 << 3); // TimeOout

            // 3. Set up data FIFO RAM
            self.registers.receive_fifo_size.set(RX_FIFO_SIZE as u32 & 0xffff);
            self.registers.transmit_fifo_size.set(
                (TX_FIFO_SIZE as u32) << 16 |
                (RX_FIFO_SIZE as u32) & 0xffff);
            for (i,d) in self.registers.device_in_ep_tx_fifo_size.iter().enumerate() {
                let i = i as u16;
                d.set((TX_FIFO_SIZE as u32) << 16 |
                      (RX_FIFO_SIZE + i * TX_FIFO_SIZE) as u32);
            }

            // TODO(alevy): Flush TxFIFOs and RxFIFO

            // 4. Reset device address field
            self.registers.device_config.set(
                self.registers.device_config.get() & 0b11111110000);
        }

        if status & ENUM_DONE != 0 {
            static mut BUF0: [u8; 64] = [0; 64];
            static mut DESC0: registers::DMADescriptor =
                registers::DMADescriptor {
                    flags: 0,
                    addr: 0
                };

            let enum_speed = (self.registers.device_status.get() & 0b110) >> 1;
            println!("ENUM DONE {:#x}", enum_speed);
            // MPS default set to 0 == 64 bytes

            // Setup descriptor for OUT endpoint 0
            unsafe {
                DESC0.flags = 1 << 27 | 1 << 25 | 64;
                DESC0.addr = BUF0.as_ptr() as usize;
                self.registers.out_endpoints[0].dma_address.set(
                    &DESC0 as *const registers::DMADescriptor as u32);
            }
            // Enable OUT endpoint 0 and clear NAK bit
            self.registers.out_endpoints[0].control.set(1 << 31 | 1 << 26);
        }

        if status & EARLY_SUSPEND != 0 {
            println!("EARLY SUSPEND");
        }

        if status & USB_SUSPEND != 0 {
            println!("USB_SUSPEND");
        }

        if self.registers.interrupt_mask.get() & status & SOF != 0 {
            println!("SOF");
            self.registers.interrupt_mask.set(
                self.registers.interrupt_mask.get() & !SOF);
        }

        if status & OEPINT != 0 {
            println!("OEPINT");
            self.registers.interrupt_mask.set(
                self.registers.interrupt_mask.get() & !OEPINT);
        }

        self.registers.interrupt_status.set(status);
    }

    pub fn init(&self) {

        // ** GLOBALSEC **
        // TODO(alevy): refactor out
        unsafe {
            use core::intrinsics::volatile_store as vs;
            // GLOBALSEC_DDMA0-DDMA3
            vs(0x40090080 as *mut u32, !0);
            vs(0x40090084 as *mut u32, !0);
            vs(0x40090088 as *mut u32, !0);
            vs(0x4009008c as *mut u32, !0);

            // GLOBALSEC_DUSB_REGION0-DUSB_REGION3
            vs(0x400900c0 as *mut u32, !0);
            vs(0x400900c4 as *mut u32, !0);
            vs(0x400900c8 as *mut u32, !0);
            vs(0x400900cc as *mut u32, !0);
        }

        self.core_clock.enable();
        self.timer_clock.enable();

        // Select PHY A
        self.registers.gpio.set((1 << 15 | // WRITE mode
                                0b100 << 4 | // Select PHY A & Set PHY active
                                0) << 16); // CUSTOM_CFG Register

        self.registers.reset.set(1);
        while self.registers.reset.get() & 1 == 1 {}

        //=== Begin Core Initialization ==//


        // We should be reading `user_hw_config` registers to find out about the
        // hardware configuration (which endpoints are in/out, OTG capable,
        // etc). Skip that for now and just make whatever assumption CR50 is
        // making.

        // Set the following parameters:
        //   * Enable DMA Mode
        //   * Global unmask interrupts
        //   * Interrupt on Non-Periodic TxFIFO completely empty
        // _Don't_ set:
        //   * Periodic TxFIFO interrupt on empty (only valid in slave mode)
        //   * AHB Burst length (defaults to 1 word)
        self.registers.ahb_config.set(1 |      // Global Interrupt unmask
                                      1 << 5 | // DMA Enable
                                      1 << 7); // Non_periodic TxFIFO

        // Mask all specific interrupts for now
        self.registers.interrupt_mask.set(0);
        self.registers.device_in_ep_interrupt_mask.set(0);
        self.registers.device_out_ep_interrupt_mask.set(0);
        self.registers.device_all_ep_interrupt_mask.set(0);

        // Configure the chip
        self.registers.configuration.set(
            1 << 6 | // USB 1.1 Full Speed
            0 << 5 | // 6-pin unidirectional
            14 << 10 | // USB Turnaround time to 14 -- what does this mean though??
            7); // Timeout calibration to 7 -- what does this mean though??

        // The datasheet says to unmask OTG and Mode Mismatch interrupts, but
        // we don't support anything but device mode for now, so let's skip
        // handling that
        // 
        // If we're right, then
        // `self.registers.interrupt_status.get() & 1 == 0`
        //

        //=== Done with core initialization ==//

        //===  Begin Device Initialization  ==//

        self.registers.device_config.set(
            0b11       | // Device Speed: USB 1.1 Full speed (48Mhz)
            0 << 2     | // Non-zero-length Status: semd packet to application
            0b00 << 11 | // Periodic frame interval: 80%
            1 << 23 );   // Enable Scatter/gather

        // We would set the device threshold control register here, but I don't
        // think we enable thresholding.

        // Clear the Soft Disconnect bit to allow the core to issue a connect.
        self.registers.device_control.set(
            self.registers.device_control.get() & !(1 << 1));

        // To set ourselves for processing the state machine through interrupts,
        // unmask:
        //
        //   * USB Reset
        //   * Enumeration Done
        //   * Early Suspend
        //   * USB Suspend
        //   * SOF
        //
        self.registers.interrupt_mask.set(
            SOF | EARLY_SUSPEND | USB_SUSPEND | USB_RESET | ENUM_DONE | OEPINT);
    }
}

interrupt_handler!(usb_handler, 193);
