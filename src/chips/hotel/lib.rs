#![crate_name = "hotel"]
#![crate_type = "rlib"]
#![no_std]
#![feature(const_fn)]

extern crate common;
extern crate support;

pub mod gpio;
pub mod pinmux;
pub mod pmu;
pub mod uart;

extern {
    // _estack is not really a function, but it makes the types work
    // You should never actually invoke it!!
    fn _estack();

    // Defined in src/main/main.rs
    fn main();

    static mut _ero : u32;
    static mut _sdata : u32;
    static mut _edata : u32;
    static mut _sbss : u32;
    static mut _ebss : u32;
}


#[link_section=".vectors"]
pub static ISR_VECTOR: [Option<unsafe extern fn()>; 2] = [
    /* Stack top */     Option::Some(_estack),
    /* Reset */         Option::Some(reset_handler),
];

unsafe extern "C" fn reset_handler() {
    // Relocate data segment.
    // Assumes data starts right after text segment as specified by the linker
    // file.
    let mut pdest  = &mut _sdata as *mut u32;
    let pend  = &mut _edata as *mut u32;
    let mut psrc = &_ero as *const u32;

    if psrc != pdest {
        while (pdest as *const u32) < pend {
            *pdest = *psrc;
            pdest = pdest.offset(1);
            psrc = psrc.offset(1);
        }
    }

    // Clear the zero segment (BSS)
    let pzero = &_ebss as *const u32;
    pdest = &mut _sbss as *mut u32;

    while (pdest as *const u32) < pzero {
        *pdest = 0;
        pdest = pdest.offset(1);
    }

    main()
}

