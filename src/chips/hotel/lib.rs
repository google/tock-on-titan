#![crate_name = "hotel"]
#![crate_type = "rlib"]
#![no_std]
#![feature(const_fn)]

extern crate common;
extern crate support;

pub mod gpio;
pub mod pmu;

extern {
    // _estack is not really a function, but it makes the types work
    // You should never actually invoke it!!
    fn _estack();

    // Defined in src/main/main.rs
    fn main();
}


#[link_section=".vectors"]
pub static ISR_VECTOR: [Option<unsafe extern fn()>; 2] = [
    /* Stack top */     Option::Some(_estack),
    /* Reset */         Option::Some(reset_handler),
];

unsafe extern "C" fn reset_handler() {
    main()
}

