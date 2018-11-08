use core::fmt::Write;
use core::panic::PanicInfo;
use cortexm3;
use kernel::debug;
use kernel::hil::led;
use hotel;

use PROCESSES;

pub struct Writer;

static mut WRITER: Writer = Writer {};

impl Write for Writer {
    fn write_str(&mut self, s: &str) -> ::core::fmt::Result {
        unsafe {
            let uart = &hotel::uart::UART0;

            static mut INITIALIZED: bool = false;
            if !INITIALIZED {
                INITIALIZED = true;

                let pinmux = &mut *hotel::pinmux::PINMUX;
                // Drive DIOA0 from TX
                pinmux.diob1.select.set(hotel::pinmux::Function::Uart0Tx);

                uart.config(115200);
            }

            uart.send_bytes_sync(s.as_bytes());
            Ok(())
        }
    }
}

/// Panic handler.
#[cfg(not(test))]
#[no_mangle]
#[panic_implementation]
pub unsafe extern "C" fn panic_fmt(pi: &PanicInfo) -> ! {
    let led = &mut led::LedLow::new(&mut hotel::gpio::PORT0.pins[0]);
    let writer = &mut WRITER;
    debug::panic(&mut [led], writer, pi, &cortexm3::support::nop, &PROCESSES)
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
