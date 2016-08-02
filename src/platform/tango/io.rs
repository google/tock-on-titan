use hotel;
use core::fmt::*;

use hotel::pmu::*;

pub struct Writer;

impl Write for Writer {
    fn write_str(&mut self, s: &str) -> ::core::fmt::Result {
        let uart = unsafe { &mut *hotel::uart::UART0 };
        let uart_clock =
            unsafe { Clock::new(PeripheralClock::Bank1(PeripheralClock1::Uart0Timer)) };
        uart_clock.enable();

        static mut initialized: bool  = false;
        if unsafe { !initialized } {
            unsafe { initialized = true };

            let pinmux = unsafe { &mut *hotel::pinmux::PINMUX };
            // Drive DIOA0 from TX
            pinmux.dioa0.select.set(hotel::pinmux::Function::Uart0Tx);

            uart.nco.set(5033);
            uart.control.set(1);
        }
        for c in s.bytes() {
            while uart.state.get() & 1 != 0 {}
            uart.write_data.set(c as u32);
        }
        Ok(())
    }
}


#[cfg(not(test))]
#[lang="panic_fmt"]
#[no_mangle]
pub unsafe extern fn rust_begin_unwind(args: Arguments,
    file: &'static str, line: u32) -> ! {

    let mut writer = Writer;
    let _ = writer.write_fmt(format_args!("Kernel panic at {}:{}:\r\n\t\"", file, line));
    let _ = write(&mut writer, args);
    let _ = writer.write_str("\"\r\n");

    loop {}
}

#[macro_export]
macro_rules! print {
        ($($arg:tt)*) => (
            {
                use core::fmt::write;
                let mut writer = $crate::io::Writer;
                let _ = write(&mut writer, format_args!($($arg)*));
            }
        );
}

#[macro_export]
macro_rules! println {
        ($fmt:expr) => (print!(concat!($fmt, "\n")));
            ($fmt:expr, $($arg:tt)*) => (print!(concat!($fmt, "\n"), $($arg)*));
}
