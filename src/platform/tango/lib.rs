#![crate_name = "platform"]
#![crate_type = "rlib"]
#![no_std]
#![feature(core_intrinsics,lang_items)]

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

const LED : u32 = 0;
const LED_GPIO : u16 = 1;

pub unsafe fn init<'a>() -> &'a mut Firestorm {
    use core::intrinsics::{volatile_load,volatile_store};
    use hotel::pmu::*;

    let uart_clock =
            Clock::new(PeripheralClock::Bank1(PeripheralClock1::Uart0Timer));
    let gpio_clock =
            Clock::new(PeripheralClock::Bank0(PeripheralClock0::Gpio0));

    /* ********************
     * UART
     * ********************/

    // Turn on UART0 clock
    uart_clock.enable();

    // Drive DIOA0 from TX
    let pinmux_dioa0_sel : *mut u16 = 0x40060028 as *mut u16;
    volatile_store(pinmux_dioa0_sel, 70);

    // Setup baud rate
    let nco = 5300; // 2^20 * 115200 / 24000000
    let uart_nco : *mut u16 = 0x40600008 as *mut u16;
    volatile_store(uart_nco, nco);

    // Enable TX
    let uart_ctrl : *mut u16 = 0x4060000C as *mut u16;
    volatile_store(uart_ctrl, 1);

    unsafe fn write_char(c : char) {
        let uart_state : *mut u32 = 0x40600014 as *mut u32;
        let uart_wdata : *mut u8 = 0x40600004 as *mut u8;

        while volatile_load(uart_state) & 1 != 0 {}

        volatile_store(uart_wdata, c as u8);
    }

    unsafe fn write_str(s : &str) {
        for c in s.chars() {
            write_char(c);
        }
    }

    write_str("Hello from Rust!\n");

    /* ********************
     * Blink
     * ********************/

    // Turn on GPIO clocks
    gpio_clock.enable();

    // Driver DIOM4 from GPIO0_0
    let pinmux_diom4_sel : *mut u16 = 0x40060020 as *mut u16;
    volatile_store(pinmux_diom4_sel, LED_GPIO);

    // Enable output on GPIO0_0
    let gpio0_outen : *mut u32 = 0x40200010 as *mut u32;
    volatile_store(gpio0_outen, 1 << LED);

    // Set GPIO0_0
    let gpio0_out : *mut u32 = 0x40200004 as *mut u32;
    volatile_store(gpio0_out, 1 << LED);

    loop {
        for _ in 0..3000000 {
            support::nop();
        }
        volatile_store(gpio0_out, 0 << LED);
        for _ in 0..3000000 {
            support::nop();
        }
        volatile_store(gpio0_out, 1 << LED);
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
