use core::fmt::*;
use hotel;

pub struct Writer;

impl Write for Writer {
    fn write_str(&mut self, s: &str) -> ::core::fmt::Result {
        unsafe {
            let uart = &hotel::uart::UART0;

            static mut initialized: bool = false;
            if !initialized {
                initialized = true;

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


#[cfg(not(test))]
#[lang="panic_fmt"]
#[no_mangle]
pub unsafe extern "C" fn rust_begin_unwind(args: Arguments, file: &'static str, line: u32) -> ! {

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
