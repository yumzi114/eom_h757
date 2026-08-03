#![no_std]
#![no_main]

extern crate alloc;

use core::mem::MaybeUninit;
use embedded_alloc::LlffHeap as Heap;
use panic_halt as _;

// build.rs에서 생성된 AppWindow 등을 현재 crate 루트에 포함한다.
slint::include_modules!();

// ============================================================
// Slint heap
// ============================================================

const HEAP_SIZE: usize = 64 * 1024;

#[global_allocator]
static HEAP: Heap = Heap::empty();

#[unsafe(link_section = ".uninit.heap")]
static mut HEAP_MEM: MaybeUninit<[u8; HEAP_SIZE]> =
    MaybeUninit::uninit();

// ============================================================
// CM7 RTIC application
// ============================================================

#[rtic::app(
    device = stm32h7::stm32h757cm7,
    dispatchers = [SPI1, SPI2, SPI3],
)]
mod app {
    use core::ptr::{
        addr_of_mut,
        read_volatile,
        write_volatile,
    };

    use rtt_target::{
        rprintln,
        rtt_init_print,
    };

    use slint::SharedString;

    // crate 루트에 생성된 AppWindow와 allocator를 가져온다.
    use super::{
        AppWindow,
        HEAP,
        HEAP_MEM,
        HEAP_SIZE,
    };

    // ========================================================
    // STM32H757 shared SRAM4
    // ========================================================
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
    // ========================================================

    const CM7_MAGIC_ADDR: *mut u32 =
        0x3800_0000 as *mut u32;

    const CM4_MAGIC_ADDR: *mut u32 =
        0x3800_0004 as *mut u32;

    const CM7_COUNT_ADDR: *mut u32 =
        0x3800_0008 as *mut u32;

    const CM4_COUNT_ADDR: *mut u32 =
        0x3800_000C as *mut u32;

    const CM7_MAGIC: u32 = 0xC07C_07C7;
    const CM4_MAGIC: u32 = 0xC04C_04C4;

    // ========================================================
    // RCC global control register
    // ========================================================
    //
    // RCC base : 0x5802_4400
    // RCC_GCR  : offset 0xA0
    // address  : 0x5802_44A0
    //
    // bit 3 BOOT_C2:
    //   0 = Cortex-M4 boot hold
    //   1 = Cortex-M4 boot allowed
    // ========================================================

    const RCC_GCR_ADDR: *mut u32 =
        0x5802_44A0 as *mut u32;

    const RCC_GCR_BOOT_C2: u32 =
        1 << 3;

    // ========================================================
    // RTIC resources
    // ========================================================

    #[shared]
    struct Shared {}

    #[local]
    struct Local {}

    // ========================================================
    // CM4 실행 허용
    // ========================================================

    unsafe fn boot_cm4() {
        let current = unsafe {
            read_volatile(RCC_GCR_ADDR)
        };

        unsafe {
            write_volatile(
                RCC_GCR_ADDR,
                current | RCC_GCR_BOOT_C2,
            );
        }

        // 레지스터 쓰기 완료 보장
        cortex_m::asm::dsb();
        cortex_m::asm::isb();

        // WFE로 대기하는 코어가 있다면 이벤트 발생
        cortex_m::asm::sev();
    }

    // ========================================================
    // BOOT_C2 상태 읽기
    // ========================================================

    unsafe fn cm4_boot_enabled() -> bool {
        let gcr = unsafe {
            read_volatile(RCC_GCR_ADDR)
        };

        (gcr & RCC_GCR_BOOT_C2) != 0
    }

    // ========================================================
    // RTIC init
    // ========================================================

    #[init]
    fn init(_cx: init::Context) -> (Shared, Local) {
        rtt_init_print!();

        rprintln!("");
        rprintln!("================================");
        rprintln!(" STM32H757 CM7 BOOT");
        rprintln!("================================");

        // ----------------------------------------------------
        // Slint allocator 초기화
        // ----------------------------------------------------

        unsafe {
            HEAP.init(
                addr_of_mut!(HEAP_MEM).cast::<u8>() as usize,
                HEAP_SIZE,
            );
        }

        rprintln!(
            "CM7 heap initialized: {} bytes",
            HEAP_SIZE,
        );

        // ----------------------------------------------------
        // Slint 문자열 할당 테스트
        // ----------------------------------------------------

        let test = SharedString::from("Slint loaded");

        rprintln!(
            "Slint SharedString: {}",
            test.as_str(),
        );

        // ----------------------------------------------------
        // Slint 컴포넌트 생성 테스트
        // ----------------------------------------------------
        //
        // 아직 Platform과 Renderer를 등록하지 않았기 때문에
        // 실제 화면 표시나 run()은 호출하지 않는다.
        // 현재 단계는 생성 및 링크 확인용이다.
        // ----------------------------------------------------

        match AppWindow::new() {
            Ok(ui) => {
                rprintln!("Slint AppWindow created");

                // 현재는 UI를 RTIC resource에 보관하지 않으므로
                // 생성 확인 후 의도적으로 해제를 막는다.
                //
                // 실제 LCD 구동 단계에서는 Local resource에 넣는다.
                core::mem::forget(ui);
            }

            Err(error) => {
                rprintln!(
                    "Slint AppWindow creation failed: {:?}",
                    error,
                );
            }
        }

        // ----------------------------------------------------
        // CM7 공유 메모리 초기화
        // ----------------------------------------------------
        //
        // CM4 영역은 CM7이 지우지 않는다.
        // CM4가 먼저 실행되어 값을 기록한 경우 이를 보존한다.
        // ----------------------------------------------------

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

        cortex_m::asm::dsb();

        // ----------------------------------------------------
        // CM4 부팅 허용
        // ----------------------------------------------------

        let gcr_before = unsafe {
            read_volatile(RCC_GCR_ADDR)
        };

        rprintln!(
            "RCC_GCR before = 0x{:08X}",
            gcr_before,
        );

        unsafe {
            boot_cm4();
        }

        let gcr_after = unsafe {
            read_volatile(RCC_GCR_ADDR)
        };

        let boot_enabled = unsafe {
            cm4_boot_enabled()
        };

        rprintln!(
            "RCC_GCR after  = 0x{:08X}",
            gcr_after,
        );

        rprintln!(
            "BOOT_C2        = {}",
            boot_enabled as u8,
        );

        rprintln!(
            "CM4 image      = 0x08100000",
        );

        rprintln!(
            "Waiting for CM4 heartbeat...",
        );

        (
            Shared {},
            Local {},
        )
    }

    // ========================================================
    // RTIC idle
    // ========================================================

    #[idle]
    fn idle(_cx: idle::Context) -> ! {
        let mut cm7_counter = 0u32;
        let mut print_divider = 0u32;

        let mut cm4_detected = false;
        let mut previous_cm4_counter = 0u32;
        let mut stale_count = 0u32;

        loop {
            cm7_counter =
                cm7_counter.wrapping_add(1);

            unsafe {
                write_volatile(
                    CM7_COUNT_ADDR,
                    cm7_counter,
                );
            }

            let cm4_magic = unsafe {
                read_volatile(
                    CM4_MAGIC_ADDR as *const u32,
                )
            };

            let cm4_counter = unsafe {
                read_volatile(
                    CM4_COUNT_ADDR as *const u32,
                )
            };

            // ------------------------------------------------
            // CM4 감지
            // ------------------------------------------------

            if !cm4_detected &&
                cm4_magic == CM4_MAGIC
            {
                cm4_detected = true;
                previous_cm4_counter = cm4_counter;
                stale_count = 0;

                rprintln!("");
                rprintln!("*** CM4 BOOT DETECTED ***");

                rprintln!(
                    "CM4 magic   = 0x{:08X}",
                    cm4_magic,
                );

                rprintln!(
                    "CM4 counter = {}",
                    cm4_counter,
                );

                rprintln!("");
            }

            // ------------------------------------------------
            // CM4 heartbeat 변화 검사
            // ------------------------------------------------

            if cm4_detected {
                if cm4_counter ==
                    previous_cm4_counter
                {
                    stale_count =
                        stale_count.wrapping_add(1);
                } else {
                    stale_count = 0;
                    previous_cm4_counter =
                        cm4_counter;
                }
            }

            print_divider =
                print_divider.wrapping_add(1);

            // ------------------------------------------------
            // 주기적 상태 출력
            // ------------------------------------------------

            if print_divider >= 1_000_000 {
                print_divider = 0;

                let gcr = unsafe {
                    read_volatile(
                        RCC_GCR_ADDR as *const u32,
                    )
                };

                if cm4_detected {
                    rprintln!(
                        concat!(
                            "CM7={} CM4={} ",
                            "magic=0x{:08X} ",
                            "GCR=0x{:08X} ",
                            "stale={}"
                        ),
                        cm7_counter,
                        cm4_counter,
                        cm4_magic,
                        gcr,
                        stale_count,
                    );
                } else {
                    rprintln!(
                        concat!(
                            "CM7={} ",
                            "CM4=NOT_DETECTED ",
                            "magic=0x{:08X} ",
                            "count={} ",
                            "GCR=0x{:08X}"
                        ),
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