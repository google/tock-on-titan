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

    use core::intrinsics::volatile_store;

    // Turn on GPIO clocks
    let pmu_periclockset0 : *mut u32 = 0x40000064 as *mut u32;
    volatile_store(pmu_periclockset0, 1 << 8);


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
