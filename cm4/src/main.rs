#![no_std]
#![no_main]

use panic_halt as _;

#[rtic::app(
    device = stm32h7::stm32h757cm4,
)]
mod app {
    use core::ptr::write_volatile;

    use drivers::fdcan;

    const CM4_MAGIC_ADDR: *mut u32 =
        0x3800_0004 as *mut u32;

    const CM4_COUNT_ADDR: *mut u32 =
        0x3800_000C as *mut u32;

    const FDCAN_RESULT_ADDR: *mut u32 =
        0x3800_0010 as *mut u32;

    const FDCAN_ENDN_ADDR: *mut u32 =
        0x3800_0014 as *mut u32;

    const FDCAN_CCCR_ADDR: *mut u32 =
        0x3800_0018 as *mut u32;

    const FDCAN_NBTP_ADDR: *mut u32 =
        0x3800_001C as *mut u32;

    const FDCAN_PSR_ADDR: *mut u32 =
        0x3800_0020 as *mut u32;

    const FDCAN_RCC_ADDR: *mut u32 =
        0x3800_0024 as *mut u32;

    const CM4_MAGIC: u32 =
        0xC04C_04C4;

    const FDCAN_OK: u32 =
        0xFDC0_0001;

    const FDCAN_FAIL: u32 =
        0xFDC0_DEAD;
    const FDCAN_D2CCIP1R_ADDR: *mut u32 =
        0x3800_0028 as *mut u32;
        const FDCAN_TEST_ADDR: *mut u32 =
        0x3800_002C as *mut u32;

    const FDCAN_ECR_ADDR: *mut u32 =
        0x3800_0030 as *mut u32;

    const FDCAN_TXBTO_ADDR: *mut u32 =
        0x3800_0034 as *mut u32;

    const FDCAN_RXF0S_ADDR: *mut u32 =
        0x3800_0038 as *mut u32;

    const FDCAN_RX_ID_ADDR: *mut u32 =
        0x3800_003C as *mut u32;

    const FDCAN_RX_DLC_ADDR: *mut u32 =
        0x3800_0040 as *mut u32;

    const FDCAN_RX_DATA0_ADDR: *mut u32 =
        0x3800_0044 as *mut u32;

    const FDCAN_RX_DATA1_ADDR: *mut u32 =
        0x3800_0048 as *mut u32;
    #[shared]
    struct Shared {}

    #[local]
    struct Local {}

    #[init]
    fn init(_cx: init::Context) -> (Shared, Local) {
        unsafe {
            write_volatile(CM4_MAGIC_ADDR, CM4_MAGIC);
            write_volatile(CM4_COUNT_ADDR, 0);

            write_volatile(FDCAN_RESULT_ADDR, 0);
            write_volatile(FDCAN_ENDN_ADDR, 0);
            write_volatile(FDCAN_CCCR_ADDR, 0);
            write_volatile(FDCAN_NBTP_ADDR, 0);
            write_volatile(FDCAN_PSR_ADDR, 0);
            write_volatile(FDCAN_RCC_ADDR, 0);
            write_volatile(FDCAN_D2CCIP1R_ADDR, 0);
            write_volatile(FDCAN_TEST_ADDR, 0);
            write_volatile(FDCAN_ECR_ADDR, 0);
            write_volatile(FDCAN_TXBTO_ADDR, 0);
            write_volatile(FDCAN_RXF0S_ADDR, 0);
            write_volatile(FDCAN_RX_ID_ADDR, 0);
            write_volatile(FDCAN_RX_DLC_ADDR, 0);
            write_volatile(FDCAN_RX_DATA0_ADDR, 0);
            write_volatile(FDCAN_RX_DATA1_ADDR, 0);
        }

        match fdcan::init_normal_500k() {
            Ok(status) => unsafe {
                write_volatile(FDCAN_ENDN_ADDR, status.endn);
                write_volatile(FDCAN_CCCR_ADDR, status.cccr);
                write_volatile(FDCAN_NBTP_ADDR, status.nbtp);
                write_volatile(FDCAN_PSR_ADDR, status.psr);
                write_volatile(FDCAN_RCC_ADDR, status.apb1henr);
                write_volatile(FDCAN_D2CCIP1R_ADDR, status.d2ccip1r);

                write_volatile(FDCAN_TEST_ADDR, status.test);
                write_volatile(FDCAN_ECR_ADDR, status.ecr);
                write_volatile(FDCAN_TXBTO_ADDR, status.txbto);
                write_volatile(FDCAN_RXF0S_ADDR, status.rxf0s);
                write_volatile(FDCAN_RX_ID_ADDR, status.rx_id);
                write_volatile(FDCAN_RX_DLC_ADDR, status.rx_dlc);
                write_volatile(FDCAN_RX_DATA0_ADDR, status.rx_data0);
                write_volatile(FDCAN_RX_DATA1_ADDR, status.rx_data1);

                cortex_m::asm::dmb();
                write_volatile(FDCAN_RESULT_ADDR, FDCAN_OK);
            }

            Err(_) => unsafe {
                cortex_m::asm::dmb();
                write_volatile(FDCAN_RESULT_ADDR, FDCAN_FAIL);
            },
        }

        (Shared {}, Local {})
    }

    #[idle]
    fn idle(_cx: idle::Context) -> ! {
        let mut counter = 0u32;

        loop {
            counter = counter.wrapping_add(1);

            unsafe {
                write_volatile(
                    CM4_MAGIC_ADDR,
                    CM4_MAGIC,
                );

                write_volatile(
                    CM4_COUNT_ADDR,
                    counter,
                );
            }

            for _ in 0..100_0 {
                cortex_m::asm::nop();
            }
        }
    }
}