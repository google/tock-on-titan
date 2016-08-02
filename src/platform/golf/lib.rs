#![crate_name = "platform"]
#![crate_type = "rlib"]
#![no_std]
#![feature(core_intrinsics,lang_items)]

extern crate cortexm3;
extern crate drivers;
extern crate hotel;
extern crate hil;
extern crate support;

#[macro_use]
pub mod io;

pub mod systick;

pub struct Firestorm {
    chip: hotel::chip::Hotel,
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

pub unsafe fn init<'a>() -> &'a mut Firestorm {
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
        use hil::gpio::GPIOPin;
        Clock::new(PeripheralClock::Bank0(PeripheralClock0::Gpio0)).enable();
        let pinmux = &mut *hotel::pinmux::PINMUX;
        pinmux.diob0.select.set(hotel::pinmux::Function::Gpio0Gpio0);
    }

    static_init!(gpio_pins : [&'static hotel::gpio::GPIOPin; 1] =
        [ &hotel::gpio::PORT0.pins[0] ]
    );

    static_init!(gpio : drivers::gpio::GPIO<'static, hotel::gpio::GPIOPin> =
                 drivers::gpio::GPIO::new(gpio_pins));

    static_init!(firestorm : Firestorm = Firestorm {
        chip: hotel::chip::Hotel::new(),
        gpio: gpio
    });

    let end = timer.now();

    println!("Hello from Rust! Initialization took {} tics.", end.wrapping_sub(start));

    firestorm
}

impl Firestorm {
    pub unsafe fn service_pending_interrupts(&mut self) {
    }

    pub unsafe fn has_pending_interrupts(&mut self) -> bool {
        // FIXME: Obviously this won't work when we have interrupts.
        false
    }

    pub fn mpu(&mut self) -> &mut cortexm3::mpu::MPU {
        &mut self.chip.mpu
    }

    #[inline(never)]
    pub fn with_driver<F, R>(&mut self, driver_num: usize, f: F) -> R where
            F: FnOnce(Option<&hil::Driver>) -> R {
        match driver_num {
            1 => f(Some(self.gpio)),
            _ => f(None)
        }
    }
}

