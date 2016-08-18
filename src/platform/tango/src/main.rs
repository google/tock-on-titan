#![no_std]
#![no_main]
#![feature(lang_items)]

extern crate drivers;
extern crate hotel;
extern crate hil;
extern crate main;

#[macro_use]
pub mod io;

use main::{Chip, MPU, Platform};

unsafe fn load_processes() -> &'static mut [Option<main::process::Process<'static>>] {
    extern {
        /// Beginning of the ROM region containing app images.
        static _sapps : u8;
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

pub struct Tango {
    gpio: &'static drivers::gpio::GPIO<'static, hotel::gpio::GPIOPin>
}

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

    static_init!(gpio_pins : [&'static hotel::gpio::GPIOPin; 1] =
        [ &hotel::gpio::PORT0.pins[0] ]
    );

    static_init!(gpio : drivers::gpio::GPIO<'static, hotel::gpio::GPIOPin> =
                 drivers::gpio::GPIO::new(gpio_pins));

    static_init!(platform : Tango = Tango {
        gpio: gpio
    });

    let end = timer.now();

    println!("Hello from Rust! Initialization took {} tics.", end.wrapping_sub(start));

    let mut chip = hotel::chip::Hotel::new();
    chip.mpu().enable_mpu();


    main::main(platform, &mut chip, load_processes());
}

impl Platform for Tango {
    fn with_driver<F, R>(&mut self, driver_num: usize, f: F) -> R where
            F: FnOnce(Option<&main::Driver>) -> R {
        match driver_num {
            1 => f(Some(self.gpio)),
            _ => f(None)
        }
    }
}

