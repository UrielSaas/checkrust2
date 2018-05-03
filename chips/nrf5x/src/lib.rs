#![feature(asm, concat_idents, const_fn, const_cell_new, try_from)]
#![no_std]

#[allow(unused_imports)]
#[macro_use(debug, debug_gpio)]
extern crate kernel;

mod peripheral_registers;

pub mod aes;
pub mod clock;
pub mod gpio;
pub mod peripheral_interrupts;
pub mod pinmux;
pub mod rtc;
pub mod timer;
pub mod temperature;
pub mod trng;
pub mod constants;
