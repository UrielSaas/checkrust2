extern crate gcc;


use std::path::Path;

fn main() {
    gcc::Config::new()
        .compiler(&Path::new("arm-none-eabi-gcc"))
        .flag("-mcpu=cortex-m0")
        .flag("-mthumb")
        .file("src/cortex-m0/mul.s")
        .file("src/cortex-m0/sqr.s")
        .file("src/cortex_m0_reduce25519.s")
        .file("src/cortex_m0_mpy121666.s")
        .compile("libcrt1.a");
}
