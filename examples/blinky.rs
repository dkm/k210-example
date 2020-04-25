#![no_std]
#![no_main]

use panic_halt as _;
use k210_hal::{prelude::*, fpioa, pac, gpio::Gpio};

#[riscv_rt::entry]
fn main() -> ! {
    let p = pac::Peripherals::take().unwrap();
    
    let fpioa = p.FPIOA.split();
    let io14 = fpioa.io14.into_function(fpioa::GPIO6);

    let gpio = p.GPIO.split();
    let mut gpio6 = Gpio::new(gpio.gpio6, io14).into_push_pull_output();

    gpio6.set_low().ok();

    let mut last_update = riscv::register::mcycle::read();;
    loop {
        let cur = riscv::register::mcycle::read();;
        if cur - last_update >= 40_000_000 {
            last_update = cur;
            gpio6.toggle().ok();
        }
    }
}
