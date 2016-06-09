#![crate_name = "platform"]
#![crate_type = "rlib"]
#![no_std]
#![feature(lang_items)]

extern crate hotel;
extern crate hil;
extern crate support;

pub struct Firestorm;

macro_rules! static_init {
   ($V:ident : $T:ty = $e:expr) => {
        let $V : &mut $T = {
            use core::mem::transmute;
            // Waiting out for size_of to be available at compile-time to avoid
            // hardcoding an abitrary large size...
            static mut BUF : [u8; 1024] = [0; 1024];
            let mut tmp : &mut $T = transmute(&mut BUF);
            *tmp = $e;
            tmp
        };
   }
}

pub unsafe fn init<'a>() -> &'a mut Firestorm {
    use hotel::pmu::*;
    use hotel::gpio;

    let uart_clock =
            Clock::new(PeripheralClock::Bank1(PeripheralClock1::Uart0Timer));
    let gpio_clock =
            Clock::new(PeripheralClock::Bank0(PeripheralClock0::Gpio0));

    let pinmux = &mut *hotel::pinmux::PINMUX;

    // Drive DIOA0 from TX
    pinmux.dioa0.select.set(hotel::pinmux::Function::Uart0Tx);

    // Drive DIOM4 from GPIO0_0
    pinmux.diom4.select.set(hotel::pinmux::Function::Gpio0Gpio0);

    /* ********************
     * UART
     * ********************/

    // Turn on UART0 clock
    uart_clock.enable();

    let uart = &mut *hotel::uart::UART0;

    // Setup baud rate
    uart.nco.set(5300); // 2^20 * 115200 / 24000000

    // Enable TX
    uart.control.set(1);

    for c in "Hello from Rust!\n".chars() {
        while uart.state.get() & 1 != 0 {}
        uart.write_data.set(c as u32);
    }

    /* ********************
     * Blink
     * ********************/

    // Turn on GPIO clocks
    gpio_clock.enable();

    let led = &gpio::GPIOPin::new(gpio::GPIO0_BASE, gpio::Pin::P0);

    led.enable_output();

    loop {
        for _ in 0..3000000 {
            support::nop();
        }
        led.toggle();
    }

    static_init!(firestorm : Firestorm = Firestorm);
    firestorm
}

impl Firestorm {
    pub unsafe fn service_pending_interrupts(&mut self) {
    }

    pub unsafe fn has_pending_interrupts(&mut self) -> bool {
        // FIXME: The wfi call from main() blocks forever if no interrupts are generated. For now,
        // pretend we have interrupts to avoid blocking.
        true
    }

    #[inline(never)]
    pub fn with_driver<F, R>(&mut self, driver_num: usize, f: F) -> R where
            F: FnOnce(Option<&hil::Driver>) -> R {
        match driver_num {
            //1 => f(Some(self.gpio)),
            _ => f(None)
        }
    }
}

use core::fmt::Arguments;
#[cfg(not(test))]
#[lang="panic_fmt"]
#[no_mangle]
pub unsafe extern fn rust_begin_unwind(_args: &Arguments,
    _file: &'static str, _line: usize) -> ! {
    loop {}
}

