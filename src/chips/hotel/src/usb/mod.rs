use pmu::{Clock, PeripheralClock, PeripheralClock1};

mod registers;

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
        self.registers.interrupt_mask.set(0);
    }

    pub fn init(&self) {
        self.core_clock.enable();
        self.timer_clock.enable();

        // Select PHY A
        self.registers.gpio.set(1 << 15 | // WRITE mode
                                0b110 << 4 | // Select PHY A & Set PHY active
                                0); // CUSTOM_CFG Register

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
            1 << 3  | // SOF
            1 << 10 | // Early suspend
            1 << 11 | // USB Suspend
            1 << 12 | // USB Reset
            1 << 13); // Enumeration Done
    }
}

interrupt_handler!(usb_handler, 193);
