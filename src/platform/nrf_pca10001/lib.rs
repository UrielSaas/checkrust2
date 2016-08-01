#![crate_name = "platform"]
#![crate_type = "rlib"]
#![no_std]
#![feature(lang_items)]

extern crate drivers;
extern crate hil;
extern crate nrf51822;
extern crate support;

pub mod systick;

pub struct Platform {
    gpio: &'static drivers::gpio::GPIO<'static, nrf51822::gpio::GPIOPin>,
}

pub struct DummyMPU;

impl DummyMPU {
    pub fn set_mpu(&mut self, _: u32, _: u32, _: u32, _: bool, _: u32) {
    }
}

impl Platform {
    pub unsafe fn service_pending_interrupts(&mut self) {
    }

    pub unsafe fn has_pending_interrupts(&mut self) -> bool {
        // FIXME: The wfi call from main() blocks forever if no interrupts are generated. For now,
        // pretend we have interrupts to avoid blocking.
        true
    }

    pub fn mpu(&mut self) -> DummyMPU {
        DummyMPU
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

macro_rules! static_init {
    ($V:ident : $T:ty = $e:expr, $size:expr) => {
        // Ideally we could use mem::size_of<$T> here instead of $size, however
        // that is not currently possible in rust. Instead we write the size as
        // a constant in the code and use compile-time verification to see that
        // we got it right
        let $V : &'static mut $T = {
            use core::{mem, ptr};
            // This is our compile-time assertion. The optimizer should be able
            // to remove it from the generated code.
            let assert_buf: [u8; $size] = mem::uninitialized();
            let assert_val: $T = mem::transmute(assert_buf);
            mem::forget(assert_val);

            // Statically allocate a read-write buffer for the value, write our
            // initial value into it (without dropping the initial zeros) and
            // return a reference to it.
            static mut BUF: [u8; $size] = [0; $size];
            let mut tmp : &mut $T = mem::transmute(&mut BUF);
            ptr::write(tmp as *mut $T, $e);
            tmp
        };
    }
}

pub unsafe fn init() -> &'static mut Platform {
    use nrf51822::gpio::PA;

    //XXX: this should be pared down to only give externally usable pins to the
    //  user gpio driver
    static_init!(gpio_pins: [&'static nrf51822::gpio::GPIOPin; 32] = [
        &nrf51822::gpio::PA[0],
        &nrf51822::gpio::PA[1],
        &nrf51822::gpio::PA[2],
        &nrf51822::gpio::PA[3],
        &nrf51822::gpio::PA[4],
        &nrf51822::gpio::PA[5],
        &nrf51822::gpio::PA[6],
        &nrf51822::gpio::PA[7],
        &nrf51822::gpio::PA[8],
        &nrf51822::gpio::PA[9],
        &nrf51822::gpio::PA[10],
        &nrf51822::gpio::PA[11],
        &nrf51822::gpio::PA[12],
        &nrf51822::gpio::PA[13],
        &nrf51822::gpio::PA[14],
        &nrf51822::gpio::PA[15],
        &nrf51822::gpio::PA[16],
        &nrf51822::gpio::PA[17],
        &nrf51822::gpio::PA[18],
        &nrf51822::gpio::PA[19],
        &nrf51822::gpio::PA[20],
        &nrf51822::gpio::PA[21],
        &nrf51822::gpio::PA[22],
        &nrf51822::gpio::PA[23],
        &nrf51822::gpio::PA[24],
        &nrf51822::gpio::PA[25],
        &nrf51822::gpio::PA[26],
        &nrf51822::gpio::PA[27],
        &nrf51822::gpio::PA[28],
        &nrf51822::gpio::PA[29],
        &nrf51822::gpio::PA[30],
        &nrf51822::gpio::PA[31]
    ], 4 * 32);
    static_init!(gpio: drivers::gpio::GPIO<'static, nrf51822::gpio::GPIOPin> =
                     drivers::gpio::GPIO::new(gpio_pins), 20);
    for pin in gpio_pins.iter() {
        pin.set_client(gpio);
    }

    static_init!(platform: Platform = Platform { gpio: gpio }, 4);

    platform
}

use core::fmt::Arguments;
#[cfg(not(test))]
#[lang="panic_fmt"]
#[no_mangle]
pub unsafe extern fn rust_begin_unwind(_args: &Arguments,
    _file: &'static str, _line: usize) -> ! {
    use support::nop;
    use hil::gpio::GPIOPin;

    let led0 = &nrf51822::gpio::PA[18];
    let led1 = &nrf51822::gpio::PA[19];

    led0.enable_output();
    led1.enable_output();
    loop {
        for _ in 0..100000 {
            led0.set();
            led1.set();
            nop();
        }
        for _ in 0..100000 {
            led0.clear();
            led1.clear();
            nop();
        }
    }
}
