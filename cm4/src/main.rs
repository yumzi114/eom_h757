#![no_std]
#![no_main]

use panic_halt as _;

#[rtic::app(
    device = stm32h7::stm32h757cm4,
)]
mod app {
    use core::ptr::write_volatile;

    const CM4_MAGIC_ADDR: *mut u32 = 0x3800_0004 as *mut u32;
    const CM4_COUNT_ADDR: *mut u32 = 0x3800_000C as *mut u32;

    const CM4_MAGIC: u32 = 0xC04C_04C4;

    #[shared]
    struct Shared {}

    #[local]
    struct Local {}

    #[init]
    fn init(_cx: init::Context) -> (Shared, Local) {
        unsafe {
            write_volatile(CM4_MAGIC_ADDR, CM4_MAGIC);
            write_volatile(CM4_COUNT_ADDR, 0);
        }

        (Shared {}, Local {})
    }

    #[idle]
    fn idle(_cx: idle::Context) -> ! {
        let mut counter = 0u32;

        loop {
            counter = counter.wrapping_add(1);

            unsafe {
                write_volatile(CM4_MAGIC_ADDR, CM4_MAGIC);
                write_volatile(CM4_COUNT_ADDR, counter);
            }

            for _ in 0..100_0 {
                cortex_m::asm::nop();
            }
        }
    }
}