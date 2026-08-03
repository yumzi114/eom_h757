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
    use board::clock;
    use rtt_target::{
        rprintln,
        rtt_init_print,
    };
    fn dump_rcc(
        rcc: &stm32h7::stm32h757cm7::rcc::RegisterBlock,
    ) {
        rprintln!(
            "RCC_CR       = {:08X}",
            rcc.cr().read().bits()
        );

        rprintln!(
            "RCC_CFGR     = {:08X}",
            rcc.cfgr().read().bits()
        );

        rprintln!(
            "RCC_D1CFGR   = {:08X}",
            rcc.d1cfgr().read().bits()
        );

        rprintln!(
            "RCC_D2CFGR   = {:08X}",
            rcc.d2cfgr().read().bits()
        );

        rprintln!(
            "RCC_D3CFGR   = {:08X}",
            rcc.d3cfgr().read().bits()
        );

        rprintln!(
            "PLLCKSELR    = {:08X}",
            rcc.pllckselr().read().bits()
        );

        rprintln!(
            "PLLCFGR      = {:08X}",
            rcc.pllcfgr().read().bits()
        );

        rprintln!(
            "PLL1DIVR     = {:08X}",
            rcc.pll1divr().read().bits()
        );
    }
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
        cx: init::Context,
    ) -> (Shared, Local) {
        rtt_init_print!();

        rprintln!("");
        rprintln!("==============================");
        rprintln!(" STM32H757 CM7 BOOT");
        rprintln!("==============================");
        
        match clock::configure_400mhz() {
            Ok(()) => {
                rprintln!("Clock OK");
                // clock::enable_mco2_pc9();
            }

            Err(error) => {
                let d = clock::debug_registers();

                rprintln!("Clock FAIL: {}", error);

                rprintln!("RCC_CR       = {:08X}", d.rcc_cr);
                rprintln!("RCC_CFGR     = {:08X}", d.rcc_cfgr);

                rprintln!("PLLCKSELR    = {:08X}", d.pllckselr);
                rprintln!("PLLCFGR      = {:08X}", d.pllcfgr);
                rprintln!("PLL1DIVR     = {:08X}", d.pll1divr);

                rprintln!("D1CFGR       = {:08X}", d.d1cfgr);
                rprintln!("D2CFGR       = {:08X}", d.d2cfgr);
                rprintln!("D3CFGR       = {:08X}", d.d3cfgr);

                rprintln!("PWR_CR3      = {:08X}", d.pwr_cr3);
                rprintln!("PWR_CSR1     = {:08X}", d.pwr_csr1);
                rprintln!("PWR_D3CR     = {:08X}", d.pwr_d3cr);

                rprintln!("SYSCFG_PWRCR = {:08X}", d.syscfg_pwrcr);
                rprintln!("FLASH_ACR    = {:08X}", d.flash_acr);

                loop {
                    cortex_m::asm::nop();
                }
            }
        }
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
    fn idle(_cx: idle::Context) -> ! {
        let mut count = 0u32;
        let mut rcc_dumped = false;

        loop {
            count = count.wrapping_add(1);

            unsafe {
                write_volatile(CM7_COUNT_ADDR, count);
            }

            // if !rcc_dumped && count >= 5_000_000 {
            //     let rcc = unsafe {
            //         &*stm32h7::stm32h757cm7::RCC::ptr()
            //     };

            //     dump_rcc(rcc);
            //     rcc_dumped = true;
            // }

            if count % 1_000_000 == 0 {
                let cm7_magic =
                    unsafe { read_volatile(CM7_MAGIC_ADDR) };

                let cm4_magic =
                    unsafe { read_volatile(CM4_MAGIC_ADDR) };

                let cm4_count =
                    unsafe { read_volatile(CM4_COUNT_ADDR) };

                rprintln!(
                    "CM7_MAGIC={:08X} CM7={} CM4_MAGIC={:08X} CM4={}",
                    cm7_magic,
                    count,
                    cm4_magic,
                    cm4_count
                );
                let fdcan_result =
                    unsafe { core::ptr::read_volatile(0x3800_0010 as *const u32) };

                let fdcan_endn =
                    unsafe { core::ptr::read_volatile(0x3800_0014 as *const u32) };

                let fdcan_cccr =
                    unsafe { core::ptr::read_volatile(0x3800_0018 as *const u32) };

                let fdcan_nbtp =
                    unsafe { core::ptr::read_volatile(0x3800_001C as *const u32) };

                let fdcan_psr =
                    unsafe { core::ptr::read_volatile(0x3800_0020 as *const u32) };

                let fdcan_rcc =
                    unsafe { core::ptr::read_volatile(0x3800_0024 as *const u32) };
                let fdcan_d2ccip1r =
                    unsafe { core::ptr::read_volatile(0x3800_0028 as *const u32)};
                rprintln!("FDCAN RESULT={:08X}", fdcan_result);
                rprintln!("FDCAN ENDN  ={:08X}", fdcan_endn);
                rprintln!("FDCAN CCCR  ={:08X}", fdcan_cccr);
                rprintln!("FDCAN NBTP  ={:08X}", fdcan_nbtp);
                rprintln!("FDCAN PSR   ={:08X}", fdcan_psr);
                rprintln!("FDCAN RCC   ={:08X}", fdcan_rcc);
                rprintln!("FDCAN D2CCIP1R={:08X}",fdcan_d2ccip1r);
            }

            cortex_m::asm::nop();
        }
    }
}