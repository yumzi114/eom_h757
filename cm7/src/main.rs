#![no_std]
#![no_main]

use panic_halt as _;

#[rtic::app(
    device = stm32h7::stm32h757cm7,
    dispatchers = [SPI1, SPI2, SPI3],
)]
mod app {
    use core::ptr::{read_volatile, write_volatile};
    use rtt_target::{rprintln, rtt_init_print};

    // ============================================================
    // STM32H757 shared SRAM4
    // ============================================================
    //
    // SRAM4:
    //   0x3800_0000 ~
    //
    // CM7과 CM4 모두 접근 가능.
    //
    // 0x3800_0000 : CM7 magic
    // 0x3800_0004 : CM4 magic
    // 0x3800_0008 : CM7 counter
    // 0x3800_000C : CM4 counter
    // ============================================================

    const CM7_MAGIC_ADDR: *mut u32 = 0x3800_0000 as *mut u32;
    const CM4_MAGIC_ADDR: *mut u32 = 0x3800_0004 as *mut u32;

    const CM7_COUNT_ADDR: *mut u32 = 0x3800_0008 as *mut u32;
    const CM4_COUNT_ADDR: *mut u32 = 0x3800_000C as *mut u32;

    const CM7_MAGIC: u32 = 0xC07C_07C7;
    const CM4_MAGIC: u32 = 0xC04C_04C4;

    // ============================================================
    // RCC global control register
    // ============================================================
    //
    // RCC base : 0x5802_4400
    // RCC_GCR  : offset 0xA0
    // address  : 0x5802_44A0
    //
    // bit 3 BOOT_C2:
    //   0 = Cortex-M4 boot hold
    //   1 = Cortex-M4 boot allowed
    // ============================================================

    const RCC_GCR_ADDR: *mut u32 = 0x5802_44A0 as *mut u32;
    const RCC_GCR_BOOT_C2: u32 = 1 << 3;

    #[shared]
    struct Shared {}

    #[local]
    struct Local {}

    // ------------------------------------------------------------
    // CM4 실행 허용
    // ------------------------------------------------------------

    unsafe fn boot_cm4() {
        let current = unsafe { read_volatile(RCC_GCR_ADDR) };

        unsafe {
            write_volatile(
                RCC_GCR_ADDR,
                current | RCC_GCR_BOOT_C2,
            );
        }

        // 레지스터 쓰기 완료 보장
        cortex_m::asm::dsb();
        cortex_m::asm::isb();
        cortex_m::asm::sev();
    }

    // ------------------------------------------------------------
    // BOOT_C2 상태 읽기
    // ------------------------------------------------------------

    unsafe fn cm4_boot_enabled() -> bool {
        let gcr = unsafe { read_volatile(RCC_GCR_ADDR) };

        (gcr & RCC_GCR_BOOT_C2) != 0
    }

    #[init]
    fn init(_cx: init::Context) -> (Shared, Local) {
        rtt_init_print!();

        rprintln!("");
        rprintln!("================================");
        rprintln!(" STM32H757 CM7 BOOT");
        rprintln!("================================");

        // 이전 리셋에서 SRAM4에 남은 값을 먼저 제거.
        //
        // SRAM 내용은 리셋 조건에 따라 남아 있을 수 있으므로
        // 초기화하지 않으면 CM4가 실행된 것처럼 보일 수 있다.
        unsafe {
            write_volatile(CM7_MAGIC_ADDR, CM7_MAGIC);

            // write_volatile(CM4_MAGIC_ADDR, 0);

            write_volatile(CM7_COUNT_ADDR, 0);
            // write_volatile(CM4_COUNT_ADDR, 0);
        }

        cortex_m::asm::dsb();

        let gcr_before = unsafe { read_volatile(RCC_GCR_ADDR) };

        rprintln!("RCC_GCR before = 0x{:08X}", gcr_before);

        unsafe {
            boot_cm4();
        }

        let gcr_after = unsafe { read_volatile(RCC_GCR_ADDR) };
        let boot_enabled = unsafe { cm4_boot_enabled() };

        rprintln!("RCC_GCR after  = 0x{:08X}", gcr_after);
        rprintln!("BOOT_C2        = {}", boot_enabled as u8);
        rprintln!("CM4 image      = 0x08100000");
        rprintln!("Waiting for CM4 heartbeat...");

        (
            Shared {},
            Local {},
        )
    }

    #[idle]
    fn idle(_cx: idle::Context) -> ! {
        let mut cm7_counter = 0u32;
        let mut print_divider = 0u32;

        let mut cm4_detected = false;
        let mut previous_cm4_counter = 0u32;
        let mut stale_count = 0u32;

        loop {
            cm7_counter = cm7_counter.wrapping_add(1);

            unsafe {
                write_volatile(CM7_COUNT_ADDR, cm7_counter);
            }

            let cm4_magic = unsafe {
                read_volatile(CM4_MAGIC_ADDR as *const u32)
            };

            let cm4_counter = unsafe {
                read_volatile(CM4_COUNT_ADDR as *const u32)
            };

            // CM4 init()에서 magic을 쓰면 감지.
            if !cm4_detected && cm4_magic == CM4_MAGIC {
                cm4_detected = true;
                previous_cm4_counter = cm4_counter;

                rprintln!("");
                rprintln!("*** CM4 BOOT DETECTED ***");
                rprintln!("CM4 magic   = 0x{:08X}", cm4_magic);
                rprintln!("CM4 counter = {}", cm4_counter);
                rprintln!("");
            }

            // CM4 카운터가 실제로 변하는지도 확인.
            if cm4_detected {
                if cm4_counter == previous_cm4_counter {
                    stale_count = stale_count.wrapping_add(1);
                } else {
                    stale_count = 0;
                    previous_cm4_counter = cm4_counter;
                }
            }

            print_divider = print_divider.wrapping_add(1);

            if print_divider >= 1_000_000 {
                print_divider = 0;

                let gcr = unsafe {
                    read_volatile(RCC_GCR_ADDR as *const u32)
                };

                if cm4_detected {
                    rprintln!(
                        "CM7={} CM4={} magic=0x{:08X} GCR=0x{:08X} stale={}",
                        cm7_counter,
                        cm4_counter,
                        cm4_magic,
                        gcr,
                        stale_count,
                    );
                } else {
                    rprintln!(
                        "CM7={} CM4=NOT_DETECTED magic=0x{:08X} count={} GCR=0x{:08X}",
                        cm7_counter,
                        cm4_magic,
                        cm4_counter,
                        gcr,
                    );
                }
            }

            cortex_m::asm::nop();
        }
    }
}