#![no_std]
#![no_main]

use panic_halt as _;

#[rtic::app(
    device = stm32h7::stm32h757cm7,
    dispatchers = [SPI1, SPI2, SPI3],
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

        rprintln!("STM32H757 RTIC BOOT");

        (
            Shared {},
            Local {},
        )
    }

    #[idle]
    fn idle(_cx: idle::Context) -> ! {

        let mut cnt: u32 = 0;
        loop {
            cnt = cnt.wrapping_add(1);

            if cnt % 1_000_000 == 0 {
                rprintln!("cnt = {}", cnt);
            }

            cortex_m::asm::nop();
        }
    }
}