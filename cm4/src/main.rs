#![no_std]
#![no_main]

use panic_halt as _;

#[rtic::app(
    device = stm32h7::stm32h757cm4,
)]
mod app {

    use rtt_target::{rtt_init_print, rprintln};

    #[shared]
    struct Shared {}

    #[local]
    struct Local {}

    #[init]
    fn init(_cx: init::Context) -> (Shared, Local) {

        rtt_init_print!();

        rprintln!("CM4 BOOT OK");

        (
            Shared {},
            Local {},
        )
    }

    #[idle]
    fn idle(_cx: idle::Context) -> ! {
        loop {
            rprintln!("CM4 RUN");

            for _ in 0..10_000_000 {
                cortex_m::asm::nop();
            }
        }
    }
}