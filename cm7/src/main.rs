#![no_std]
#![no_main]

use panic_halt as _;

#[rtic::app(
    device = stm32h7::stm32h757cm7,
    dispatchers = [SPI1, SPI2, SPI3],
)]
mod app {
    use core::ptr::{
        read_volatile,
        write_volatile,
    };

    use rtt_target::{
        rprintln,
        rtt_init_print,
    };

    //
    // SRAM4 shared memory
    //
    // STM32H757 SRAM4:
    // 0x3800_0000
    //

    const CM7_MAGIC_ADDR: *mut u32 =
        0x3800_0000 as *mut u32;

    const CM4_MAGIC_ADDR: *mut u32 =
        0x3800_0004 as *mut u32;

    const CM7_COUNT_ADDR: *mut u32 =
        0x3800_0008 as *mut u32;

    const CM4_COUNT_ADDR: *mut u32 =
        0x3800_000C as *mut u32;

    const CM7_MAGIC: u32 =
        0xC07C_07C7;

    //
    // RCC Global Control Register
    //
    // BOOT_C2 bit:
    // Cortex-M4 boot enable
    //

    const RCC_GCR_ADDR: *mut u32 =
        0x5802_44A0 as *mut u32;

    const RCC_GCR_BOOT_C2: u32 =
        1 << 3;

    #[shared]
    struct Shared {}

    #[local]
    struct Local {}

    unsafe fn boot_cm4() {
        let value = unsafe {
            read_volatile(RCC_GCR_ADDR)
        };

        unsafe {
            write_volatile(
                RCC_GCR_ADDR,
                value | RCC_GCR_BOOT_C2,
            );
        }

        cortex_m::asm::dsb();
        cortex_m::asm::isb();
        cortex_m::asm::sev();
    }

    unsafe fn cm4_enabled() -> bool {
        let value = unsafe {
            read_volatile(RCC_GCR_ADDR)
        };

        (value & RCC_GCR_BOOT_C2) != 0
    }

    #[init]
    fn init(
        _cx: init::Context,
    ) -> (Shared, Local) {
        rtt_init_print!();

        rprintln!("");
        rprintln!("==============================");
        rprintln!(" STM32H757 CM7 BOOT");
        rprintln!("==============================");

        //
        // Initialize CM7 shared-memory area
        //

        unsafe {
            write_volatile(
                CM7_MAGIC_ADDR,
                CM7_MAGIC,
            );

            write_volatile(
                CM7_COUNT_ADDR,
                0,
            );
        }

        rprintln!(
            "CM7 SRAM4 magic = {:08X}",
            CM7_MAGIC
        );

        //
        // Read current CM4 shared-memory state
        //

        let cm4_magic_before = unsafe {
            read_volatile(CM4_MAGIC_ADDR)
        };

        let cm4_count_before = unsafe {
            read_volatile(CM4_COUNT_ADDR)
        };

        rprintln!(
            "CM4 before boot: MAGIC={:08X} COUNT={}",
            cm4_magic_before,
            cm4_count_before
        );

        //
        // Start Cortex-M4
        //

        let gcr_before = unsafe {
            read_volatile(RCC_GCR_ADDR)
        };

        rprintln!(
            "GCR before {:08X}",
            gcr_before
        );

        unsafe {
            boot_cm4();
        }

        let gcr_after = unsafe {
            read_volatile(RCC_GCR_ADDR)
        };

        rprintln!(
            "GCR after  {:08X}",
            gcr_after
        );

        rprintln!(
            "CM4 enable {}",
            unsafe {
                cm4_enabled()
            } as u8
        );

        rprintln!("CM7 init done");

        (
            Shared {},
            Local {},
        )
    }

    #[idle]
    fn idle(
        _cx: idle::Context,
    ) -> ! {
        let mut count = 0u32;

        loop {
            count =
                count.wrapping_add(1);

            unsafe {
                write_volatile(
                    CM7_COUNT_ADDR,
                    count,
                );
            }

            if count % 1_000_000 == 0 {
                let cm7_magic = unsafe {
                    read_volatile(CM7_MAGIC_ADDR)
                };

                let cm4_magic = unsafe {
                    read_volatile(CM4_MAGIC_ADDR)
                };

                let cm4_count = unsafe {
                    read_volatile(CM4_COUNT_ADDR)
                };

                rprintln!(
                    "CM7_MAGIC={:08X} CM7={} CM4_MAGIC={:08X} CM4={}",
                    cm7_magic,
                    count,
                    cm4_magic,
                    cm4_count
                );
            }

            cortex_m::asm::nop();
        }
    }
}