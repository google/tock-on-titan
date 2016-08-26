#![crate_name = "golf"]
#![no_std]
#![no_main]
#![feature(lang_items)]

#[macro_use(static_init)]
extern crate common;
extern crate drivers;
extern crate hotel;
extern crate hil;
extern crate main;

#[macro_use]
pub mod io;

use main::{Chip, MPU, Platform};

unsafe fn load_processes() -> &'static mut [Option<main::process::Process<'static>>] {
    extern "C" {
        /// Beginning of the ROM region containing app images.
        static _sapps: u8;
    }

    const NUM_PROCS: usize = 2;

    #[link_section = ".app_memory"]
    static mut MEMORIES: [[u8; 8192]; NUM_PROCS] = [[0; 8192]; NUM_PROCS];

    static mut processes: [Option<main::process::Process<'static>>; NUM_PROCS] = [None, None];

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
        *process = Some(main::process::Process::create(addr, total_size, memory));
        // TODO: panic if loading failed?

        addr = addr.offset(total_size as isize);
    }

    if *(addr as *const usize) != 0 {
        panic!("Exceeded maximum NUM_PROCS.");
    }

    &mut processes
}

pub struct Golf {
    gpio: &'static drivers::gpio::GPIO<'static, hotel::gpio::GPIOPin>,
}

#[no_mangle]
pub unsafe fn reset_handler() {
    hotel::init();

    let timer = {
        use hotel::pmu::*;
        use hotel::timeus::Timeus;
        Clock::new(PeripheralClock::Bank1(PeripheralClock1::TimeUs0Timer)).enable();
        let timer = Timeus::new(0);
        timer
    };

    timer.start();
    let start = timer.now();

    {
        use hotel::pmu::*;
        Clock::new(PeripheralClock::Bank0(PeripheralClock0::Gpio0)).enable();
        let pinmux = &mut *hotel::pinmux::PINMUX;
        pinmux.diob0.select.set(hotel::pinmux::Function::Gpio0Gpio0);
    }

    let gpio_pins = static_init!(
        [&'static hotel::gpio::GPIOPin; 1],
        [&hotel::gpio::PORT0.pins[0]],
        4);

    let gpio = static_init!(
        drivers::gpio::GPIO<'static, hotel::gpio::GPIOPin>,
        drivers::gpio::GPIO::new(gpio_pins),
        20);

    let platform = static_init!(Golf, Golf { gpio: gpio }, 4);

    let end = timer.now();

    println!("Hello from Rust! Initialization took {} tics.",
             end.wrapping_sub(start));

    let mut chip = hotel::chip::Hotel::new();
    chip.mpu().enable_mpu();


    main::main(platform, &mut chip, load_processes());
}

impl Platform for Golf {
    fn with_driver<F, R>(&mut self, driver_num: usize, f: F) -> R
        where F: FnOnce(Option<&main::Driver>) -> R
    {
        match driver_num {
            1 => f(Some(self.gpio)),
            _ => f(None),
        }
    }
}
