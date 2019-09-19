// Copyright 2018 Google LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use core::fmt::Write;
use core::panic::PanicInfo;
use cortexm3;
use kernel::debug;
use kernel::hil::led;
use h1b;

use PROCESSES;

pub struct Writer;

static mut WRITER: Writer = Writer {};

impl Write for Writer {
    fn write_str(&mut self, s: &str) -> ::core::fmt::Result {
        unsafe {
            let uart = &h1b::uart::UART0;

            static mut INITIALIZED: bool = false;
            if !INITIALIZED {
                INITIALIZED = true;

                let pinmux = &mut *h1b::pinmux::PINMUX;
                // Drive DIOA0 from TX
                pinmux.diob1.select.set(h1b::pinmux::Function::Uart0Tx);

                uart.config(115200);
            }

            uart.send_bytes_sync(s.as_bytes());
            Ok(())
        }
    }
}

/// Panic handler.
#[cfg(not(test))]
#[panic_implementation]
#[no_mangle]
pub unsafe extern "C" fn panic_fmt(pi: &PanicInfo) -> ! {
    let led = &mut led::LedLow::new(&mut h1b::gpio::PORT0.pins[0]);
    let writer = &mut WRITER;
    debug::panic(&mut [led], writer, pi, &cortexm3::support::nop, &PROCESSES)

}

//#[cfg(not(test))]
//#[panic_handler]
//fn panic_fmt(pi: &PanicInfo) -> ! {
//    unsafe {
//        let led = &mut led::LedLow::new(&mut h1b::gpio::PORT0.pins[0]);
//        let writer = &mut WRITER;
//        debug::panic(&mut [led], writer, pi, &cortexm3::support::nop, &PROCESSES)
//    }
//}


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
