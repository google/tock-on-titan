#![crate_name = "hil"]
#![crate_type = "rlib"]
#![feature(asm,lang_items,const_fn)]
#![no_std]

extern crate common;
extern crate main;

pub mod adc;
pub mod alarm;
pub mod digest;
pub mod gpio;
pub mod i2c;
pub mod led;
pub mod spi_master;
pub mod timer;
pub mod uart;

pub trait Controller {
    type Config;

    fn configure(&self, Self::Config);
}
