// Ref: https://github.com/laanwj/k210-sdk-stuff/blob/master/rust/interrupt/src/main.rs
#![feature(llvm_asm)]
#![no_std]
#![no_main]

use k210_hal::{prelude::*, pac, clint::{msip}, stdout::Stdout};
use panic_halt as _;
use riscv::register::{mie,mstatus,mhartid,mvendorid,marchid,mimpid,mcause};
use core::sync::atomic::{AtomicBool, Ordering};
// use core::ptr;

// fn peek<T>(addr: u64) -> T {
//     unsafe { ptr::read_volatile(addr as *const T) }
// }

static INTR: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Copy, Clone)]
struct IntrInfo {
    hart_id: usize,
    cause: usize,
}

static mut INTR_INFO: Option<IntrInfo> = None;

#[export_name = "DefaultHandler"]
fn my_trap_handler() {
    let hart_id = mhartid::read();
    let cause = mcause::read().bits();

    unsafe { INTR_INFO = Some(IntrInfo { hart_id, cause }); }

    INTR.store(true, Ordering::SeqCst);

    msip::set_value(hart_id, false);
}

#[riscv_rt::entry]
fn main() -> ! {
    let hart_id = mhartid::read();

    static mut SHARED_TX: Option<k210_hal::serial::Tx<
        k210_hal::pac::UARTHS
    >> = None;

    if hart_id == 0 {
        let p = pac::Peripherals::take().unwrap();

        //configure_fpioa(p.FPIOA);

        // Configure clocks (TODO)
        let clocks = k210_hal::clock::Clocks::new();

        // Configure UART
        let serial = p.UARTHS.configure(115_200.bps(), &clocks);
        let (tx, _) = serial.split();

        unsafe {
            SHARED_TX.replace(tx);
        }
    }

    // Super-unsafe UART sharing!
    let tx = unsafe {
        SHARED_TX.as_mut().unwrap()
    };
    let mut stdout = Stdout(tx);

    if hart_id == 1 {
        // Add delay for hart 1
        for _ in 0..100000 {
            let _ = mhartid::read();
        }
    }

    // writeln!(stdout, "Hello! Some CPU information!").unwrap();
    // writeln!(stdout, "  mvendorid {:?}", mvendorid::read()).unwrap();
    // writeln!(stdout, "  marchid {:?}", marchid::read()).unwrap();
    // writeln!(stdout, "  mimpid {:?}", mimpid::read()).unwrap();
    writeln!(stdout, "This code is running on hart {}", mhartid::read()).unwrap();

    writeln!(stdout, "Enabling interrupts").unwrap();

    unsafe {
        // Enable interrupts in general
        mstatus::set_mie();
        // Set the Machine-Software bit in MIE
        mie::set_msoft();
        // Set the Machine-External bit in MIE
        mie::set_mext();
    }

    writeln!(stdout, "Generate IPI for core {} !", hart_id).unwrap();
    msip::set_value(hart_id, true);

    writeln!(stdout, "Waiting for interrupt").unwrap();
    while !INTR.load(Ordering::SeqCst) {
    }
    INTR.store(false, Ordering::SeqCst);
    writeln!(stdout, 
        "Interrupt was triggered! hart_id: {}, cause: {}", 
        unsafe { INTR_INFO }.unwrap().hart_id,
        unsafe { INTR_INFO }.unwrap().cause,
    ).unwrap();


    if hart_id == 0 {
        writeln!(stdout, "Waking other harts...").unwrap();
        // wake hart 1
        msip::set_value(1, true);
    }

    loop { unsafe { riscv::asm::wfi(); } }
}

#[export_name = "_mp_hook"]
pub extern "Rust" fn user_mp_hook() -> bool {
    use riscv::register::/*{mie, */mip/*}*/;
    use riscv::asm::wfi;

    let hart_id = mhartid::read();
    if hart_id == 0 {
        true
    } else {

        unsafe {
            msip::set_value(hart_id, false);

            // Start listening for software interrupts
            mie::set_msoft();

            loop {
                wfi();
                if mip::read().msoft() {
                    break;
                }
            }

            // Stop listening for software interrupts
            mie::clear_msoft();

            // Clear IPI
            msip::set_value(hart_id, false);
        }
        false
    }
}
