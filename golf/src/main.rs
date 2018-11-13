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

#![no_std]
#![no_main]
#![feature(lang_items)]

extern crate capsules;
extern crate hotel;
#[macro_use(static_init)]
extern crate kernel;

pub mod digest;
#[macro_use]
pub mod io;

use kernel::{Chip, MPU, Platform};

unsafe fn load_processes() -> &'static mut [Option<kernel::process::Process<'static>>] {
    extern "C" {
        /// Beginning of the ROM region containing app images.
        static _sapps: u8;
    }

    const NUM_PROCS: usize = 2;

    #[link_section = ".app_memory"]
    static mut MEMORIES: [[u8; 8192]; NUM_PROCS] = [[0; 8192]; NUM_PROCS];

    static mut processes: [Option<kernel::process::Process<'static>>; NUM_PROCS] = [None, None];

    let mut addr = &_sapps as *const u8;
    for i in 0..NUM_PROCS {
        // The first member of the LoadInfo header contains the total size of each process image. A
        // sentinel value of 0 (invalid because it's smaller than the header itself) is used to
        // mark the end of the list of processes.
        let total_size = *(addr as *const usize);
        if total_size == 0 {
            break;
        }

        let process = &mut processes[i];
        let memory = &mut MEMORIES[i];
        *process = Some(kernel::process::Process::create(addr, total_size, memory));
        // TODO: panic if loading failed?

        addr = addr.offset(total_size as isize);
    }

    if *(addr as *const usize) != 0 {
        panic!("Exceeded maximum NUM_PROCS.");
    }

    &mut processes
}

pub struct Golf {
    console: &'static capsules::console::Console<'static, hotel::uart::UART>,
    gpio: &'static capsules::gpio::GPIO<'static, hotel::gpio::Pin>,
    timer: &'static capsules::timer::TimerDriver<'static, hotel::timels::Timels>,
    digest: &'static digest::DigestDriver<'static, hotel::crypto::sha::ShaEngine>,
}

#[no_mangle]
pub unsafe fn reset_handler() {
    hotel::init();

    let timerhs = {
        use hotel::pmu::*;
        use hotel::timeus::Timeus;
        Clock::new(PeripheralClock::Bank1(PeripheralClock1::TimeUs0Timer)).enable();
        Clock::new(PeripheralClock::Bank1(PeripheralClock1::TimeLs0)).enable();
        let timer = Timeus::new(0);
        timer
    };

    timerhs.start();
    let start = timerhs.now();

    {
        use hotel::pmu::*;
        Clock::new(PeripheralClock::Bank0(PeripheralClock0::Gpio0)).enable();
        let pinmux = &mut *hotel::pinmux::PINMUX;
        pinmux.diob0.select.set(hotel::pinmux::Function::Gpio0Gpio0);

        pinmux.gpio0_gpio1.select.set(hotel::pinmux::SelectablePin::Dioa8);
        pinmux.dioa8.select.set(hotel::pinmux::Function::Gpio0Gpio1);
        pinmux.dioa8.control.set(1 << 2 | 1 << 4);

        pinmux.dioa0.select.set(hotel::pinmux::Function::Uart0Tx);
        pinmux.dioa11.control.set(1 << 2 | 1 << 4);
        pinmux.uart0_rx.select.set(hotel::pinmux::SelectablePin::Dioa11);
    }

    let console = static_init!(
        capsules::console::Console<'static, hotel::uart::UART>,
        capsules::console::Console::new(&hotel::uart::UART0,
                                       &mut capsules::console::WRITE_BUF,
                                       kernel::container::Container::create()),
        24);
    hotel::uart::UART0.set_client(console);
    console.initialize();

    let gpio_pins = static_init!(
        [&'static hotel::gpio::Pin; 2],
        [&hotel::gpio::PORT0.pins[0], &hotel::gpio::PORT0.pins[1]],
        8);

    let gpio = static_init!(
        capsules::gpio::GPIO<'static, hotel::gpio::Pin>,
        capsules::gpio::GPIO::new(gpio_pins),
        20);
    for pin in gpio_pins.iter() {
        pin.set_client(gpio)
    }

    let timer = static_init!(
        capsules::timer::TimerDriver<'static, hotel::timels::Timels>,
        capsules::timer::TimerDriver::new(
            &hotel::timels::Timels0, kernel::container::Container::create()),
        12);
    hotel::timels::Timels0.set_client(timer);

    let digest = static_init!(
        digest::DigestDriver<'static, hotel::crypto::sha::ShaEngine>,
        digest::DigestDriver::new(
                &mut hotel::crypto::sha::KEYMGR0_SHA,
                kernel::Container::create()),
        16);

    let platform = static_init!(Golf, Golf {
        console: console,
        gpio: gpio,
        timer: timer,
        digest: digest,
    }, 16);

    hotel::usb::USB0.init(&mut hotel::usb::OUT_DESCRIPTORS,
                          &mut hotel::usb::OUT_BUFFERS,
                          &mut hotel::usb::IN_DESCRIPTORS,
                          &mut hotel::usb::IN_BUFFERS,
                          hotel::usb::PHY::A,
                          None,
                          Some(0x0011),
                          Some(0x7788));

    let end = timerhs.now();

    println!("Hello from Rust! Initialization took {} tics.",
             end.wrapping_sub(start));

    let mut chip = hotel::chip::Hotel::new();
    chip.mpu().enable_mpu();


    kernel::main(platform, &mut chip, load_processes());
}

impl Platform for Golf {
    fn with_driver<F, R>(&mut self, driver_num: usize, f: F) -> R
        where F: FnOnce(Option<&kernel::Driver>) -> R
    {
        match driver_num {
            0 => f(Some(self.console)),
            1 => f(Some(self.gpio)),
            2 => f(Some(self.digest)),
            3 => f(Some(self.timer)),
            _ => f(None),
        }
    }
}
