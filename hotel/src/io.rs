use core::fmt::*;
use pinmux;

use uart;

pub struct Writer;

impl Write for Writer {
    fn write_str(&mut self, s: &str) -> ::core::fmt::Result {
        unsafe {
            let uart = &uart::UART0;

            static mut INITIALIZED: bool = false;
            if !INITIALIZED {
                INITIALIZED = true;

                let pinmux = &mut *pinmux::PINMUX;
                // Drive DIOA0 from TX
                pinmux.dioa0.select.set(pinmux::Function::Uart0Tx);

                uart.config(115200);
            }

            uart.send_bytes_sync(s.as_bytes());
 
            Ok(())
        }
    }
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
