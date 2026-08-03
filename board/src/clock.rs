use stm32h7::stm32h757cm7::{
    FLASH,
    PWR,
    RCC,
    SYSCFG,
};

const TIMEOUT: u32 = 10_000_000;

//
// 디버그 레지스터
//

pub struct ClockDebug {
    pub rcc_cr: u32,
    pub rcc_cfgr: u32,
    pub pllckselr: u32,
    pub pllcfgr: u32,
    pub pll1divr: u32,

    pub d1cfgr: u32,
    pub d2cfgr: u32,
    pub d3cfgr: u32,

    pub pwr_cr3: u32,
    pub pwr_csr1: u32,
    pub pwr_d3cr: u32,

    pub syscfg_pwrcr: u32,
    pub flash_acr: u32,
}

pub fn debug_registers() -> ClockDebug {
    let rcc = unsafe { &*RCC::ptr() };
    let pwr = unsafe { &*PWR::ptr() };
    let syscfg = unsafe { &*SYSCFG::ptr() };
    let flash = unsafe { &*FLASH::ptr() };

    ClockDebug {
        rcc_cr: rcc.cr().read().bits(),
        rcc_cfgr: rcc.cfgr().read().bits(),

        pllckselr: rcc.pllckselr().read().bits(),
        pllcfgr: rcc.pllcfgr().read().bits(),
        pll1divr: rcc.pll1divr().read().bits(),

        d1cfgr: rcc.d1cfgr().read().bits(),
        d2cfgr: rcc.d2cfgr().read().bits(),
        d3cfgr: rcc.d3cfgr().read().bits(),

        pwr_cr3: pwr.cr3().read().bits(),
        pwr_csr1: pwr.csr1().read().bits(),
        pwr_d3cr: pwr.d3cr().read().bits(),

        syscfg_pwrcr: syscfg.pwrcr().read().bits(),
        flash_acr: flash.acr().read().bits(),
    }
}

pub fn configure_400mhz() -> Result<(), &'static str> {
    let rcc = unsafe { &*RCC::ptr() };
    let pwr = unsafe { &*PWR::ptr() };
    let syscfg = unsafe { &*SYSCFG::ptr() };
    let flash = unsafe { &*FLASH::ptr() };

    let mut timeout;

    //
    // STM32H757I-EVAL = Direct SMPS
    //
    // CR3 lower byte is write-once after POR.
    // SCUEN=1 -> supply configuration write enable
    // SCUEN=0, LDOEN=0 -> Direct SMPS
    //

    pwr.cr3().write(|w| unsafe {
        w.bits(1 << 2)
    });

    pwr.cr3().write(|w| unsafe {
        w.bits(0)
    });

    timeout = TIMEOUT;

    while (pwr.csr1().read().bits() & (1 << 13)) == 0 {
        timeout -= 1;

        if timeout == 0 {
            return Err("ACTVOSRDY timeout");
        }
    }

    //
    // VOS1
    //

    pwr.d3cr().modify(|r, w| unsafe {
        let value =
            (r.bits() & !(0b11 << 14))
            | (0b11 << 14);

        w.bits(value)
    });

    timeout = TIMEOUT;

    while (pwr.d3cr().read().bits() & (1 << 13)) == 0 {
        timeout -= 1;

        if timeout == 0 {
            return Err("VOS1 timeout");
        }
    }

    //
    // Flash: HCLK 200 MHz
    //

    flash.acr().modify(|r, w| unsafe {
        let mut value = r.bits();

        value &= !0x3F;
        value |= 4;
        value |= 0b10 << 4;

        w.bits(value)
    });

    //
    // CM7 400 MHz
    // HCLK 200 MHz
    // APB 100 MHz
    //

    rcc.d1cfgr().modify(|r, w| unsafe {
        let mut value = r.bits();

        value &= !(
            (0xF << 8)
                | (0x7 << 4)
                | 0xF
        );

        value |= 0b100 << 4; // APB3 /2
        value |= 0b1000;     // HCLK /2

        w.bits(value)
    });

    rcc.d2cfgr().modify(|r, w| unsafe {
        let mut value = r.bits();

        value &= !(
            (0x7 << 8)
                | (0x7 << 4)
        );

        value |= 0b100 << 8;
        value |= 0b100 << 4;

        w.bits(value)
    });

    rcc.d3cfgr().modify(|r, w| unsafe {
        let mut value = r.bits();

        value &= !(0x7 << 4);
        value |= 0b100 << 4;

        w.bits(value)
    });

    //
    // PLL1 off
    //

    rcc.cr().modify(|r, w| unsafe {
        w.bits(r.bits() & !(1 << 24))
    });

    timeout = TIMEOUT;

    while (rcc.cr().read().bits() & (1 << 25)) != 0 {
        timeout -= 1;

        if timeout == 0 {
            return Err("PLL1 disable timeout");
        }
    }

    //
    // HSI 64 / 4 = 16 MHz
    //

    rcc.pllckselr().modify(|r, w| unsafe {
        let mut value = r.bits();

        value &= !(0b11 | (0x3F << 4));
        value |= 4 << 4;

        w.bits(value)
    });

    //
    // Wide VCO, DIVP enabled
    //

    rcc.pllcfgr().modify(|r, w| unsafe {
        let mut value = r.bits();

        value &= !(
            (0b11 << 2)
                | (1 << 1)
                | (1 << 0)
                | (1 << 16)
                | (1 << 17)
                | (1 << 18)
        );

        value |= 0b11 << 2;
        value |= (1 << 16) | (1 << 17);

        w.bits(value)
    });
    rcc.pll1divr().write(|w| unsafe {
        w.bits(
            49              // DIVN1 = 50
                | (1 << 9)  // DIVP1 = 2
                | (9 << 16) // DIVQ1 = 10
                | (1 << 24) // DIVR1 = 2
        )
    });
 

    rcc.d2ccip1r().modify(|_, w| {
        w.fdcansel().pll1_q()
    });

    rcc.pll1fracr().write(|w| unsafe {
        w.bits(0)
    });

    rcc.cr().modify(|r, w| unsafe {
        w.bits(r.bits() | (1 << 24))
    });

    timeout = TIMEOUT;

    while (rcc.cr().read().bits() & (1 << 25)) == 0 {
        timeout -= 1;

        if timeout == 0 {
            return Err("PLL1 lock timeout");
        }
    }

    rcc.cfgr().modify(|r, w| unsafe {
        let mut value = r.bits();

        value &= !0b111;
        value |= 0b011;

        w.bits(value)
    });

    timeout = TIMEOUT;

    while ((rcc.cfgr().read().bits() >> 3) & 0b111) != 0b011 {
        timeout -= 1;

        if timeout == 0 {
            return Err("SYSCLK switch timeout");
        }
    }

    cortex_m::asm::dsb();
    cortex_m::asm::isb();

    Ok(())
}

pub fn enable_mco2_pc9() {
    use stm32h7::stm32h757cm7::{GPIOC, RCC};

    let rcc = unsafe { &*RCC::ptr() };
    let gpioc = unsafe { &*GPIOC::ptr() };

    rcc.ahb4enr().modify(|r, w| unsafe {
        w.bits(r.bits() | (1 << 2))
    });

    let _ = rcc.ahb4enr().read().bits();

    gpioc.moder().modify(|r, w| unsafe {
        let mut v = r.bits();
        v &= !(0b11 << 18);
        v |= 0b10 << 18;
        w.bits(v)
    });

    gpioc.otyper().modify(|r, w| unsafe {
        w.bits(r.bits() & !(1 << 9))
    });

    gpioc.ospeedr().modify(|r, w| unsafe {
        let mut v = r.bits();
        v &= !(0b11 << 18);
        v |= 0b11 << 18;
        w.bits(v)
    });

    gpioc.pupdr().modify(|r, w| unsafe {
        w.bits(r.bits() & !(0b11 << 18))
    });

    gpioc.afrh().modify(|r, w| unsafe {
        let mut v = r.bits();
        v &= !(0xF << 4);
        w.bits(v)
    });

    rcc.cfgr().modify(|r, w| unsafe {
        let mut v = r.bits();

        v &= !((0b111 << 29) | (0b1111 << 25));

        v |= 0b000 << 29; // SYSCLK
        v |= 0b0101 << 25; // /5

        w.bits(v)
    });
}