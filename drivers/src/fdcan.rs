#![allow(dead_code)]

use core::ptr::{
    read_volatile,
    write_volatile,
};
use stm32h7::stm32h757cm4::{
    FDCAN1,
    GPIOA,
    RCC,
};

const TIMEOUT: u32 = 10_000_000;

/*
 * STM32H747/H757 FDCAN message RAM
 *
 * FDCAN1 register base : 0x4000_A000
 * Message RAM base     : 0x4000_AC00
 */
const MESSAGE_RAM_BASE: usize = 0x4000_AC00;

/*
 * Message RAM 배치
 *
 * byte offset 0x00:
 *   RX FIFO0 element 0
 *   - R0
 *   - R1
 *   - DATA0
 *   - DATA1
 *
 * byte offset 0x10:
 *   TX dedicated buffer 0
 *   - T0
 *   - T1
 *   - DATA0
 *   - DATA1
 */
const RX_FIFO0_OFFSET: usize = 0x00;
const TX_BUFFER0_OFFSET: usize = 0x10;

const TEST_CAN_ID: u16 = 0x123;

const TEST_DATA0: u32 = 0x4433_2211;
const TEST_DATA1: u32 = 0x8877_6655;

pub struct FdcanLoopbackResult {
    pub endn: u32,
    pub cccr: u32,
    pub nbtp: u32,
    pub test: u32,
    pub psr: u32,
    pub ecr: u32,

    pub txbto: u32,
    pub rxf0s: u32,

    pub rx_id: u32,
    pub rx_dlc: u32,
    pub rx_data0: u32,
    pub rx_data1: u32,

    pub d2ccip1r: u32,
    pub apb1henr: u32,
}

#[inline(always)]
unsafe fn ram_write(byte_offset: usize, value: u32) {
    let ptr =
        (MESSAGE_RAM_BASE + byte_offset) as *mut u32;

    write_volatile(ptr, value);
}

#[inline(always)]
unsafe fn ram_read(byte_offset: usize) -> u32 {
    let ptr =
        (MESSAGE_RAM_BASE + byte_offset) as *const u32;

    read_volatile(ptr)
}

pub fn init_normal_500k()
    -> Result<FdcanLoopbackResult, &'static str>
{   
    let rcc = unsafe { &*RCC::ptr() };
    let gpioa = unsafe { &*GPIOA::ptr() };
    let fdcan = unsafe { &*FDCAN1::ptr() };
    

    /*
     * FDCAN kernel clock = PLL1Q.
     *
     * CM7에서 PLL1Q = 80 MHz로 구성한 상태.
     */
    rcc.d2ccip1r().modify(|_, w| {
        w.fdcansel().pll1_q()
    });

    if !rcc
        .d2ccip1r()
        .read()
        .fdcansel()
        .is_pll1_q()
    {
        return Err("FDCANSEL write failed");
    }

    /*
     * FDCAN peripheral clock enable.
     */
    rcc.apb1henr().modify(|_, w| {
        w.fdcanen().set_bit()
    });

    let _ = rcc.apb1henr().read().bits();
    /*
    * GPIOA clock enable.
    */
    rcc.ahb4enr().modify(|r, w| unsafe {
        w.bits(r.bits() | (1 << 0))
    });

    let _ = rcc.ahb4enr().read().bits();

    /*
    * PA11 = FDCAN1_RX
    * PA12 = FDCAN1_TX
    * Alternate function mode
    */
    gpioa.moder().modify(|r, w| unsafe {
        let mut value = r.bits();

        value &= !(
            (0b11 << 22) |
            (0b11 << 24)
        );

        value |=
            (0b10 << 22) |
            (0b10 << 24);

        w.bits(value)
    });

    /*
    * Push-pull
    */
    gpioa.otyper().modify(|r, w| unsafe {
        let mut value = r.bits();

        value &= !(
            (1 << 11) |
            (1 << 12)
        );

        w.bits(value)
    });

    /*
    * Very high speed
    */
    gpioa.ospeedr().modify(|r, w| unsafe {
        let mut value = r.bits();

        value &= !(
            (0b11 << 22) |
            (0b11 << 24)
        );

        value |=
            (0b11 << 22) |
            (0b11 << 24);

        w.bits(value)
    });

    /*
    * No pull-up / pull-down
    */
    gpioa.pupdr().modify(|r, w| unsafe {
        let mut value = r.bits();

        value &= !(
            (0b11 << 22) |
            (0b11 << 24)
        );

        w.bits(value)
    });

    /*
    * PA11 / PA12 = AF9
    */
    gpioa.afrh().modify(|r, w| unsafe {
        let mut value = r.bits();

        value &= !(
            (0xF << 12) |
            (0xF << 16)
        );

        value |=
            (9 << 12) |
            (9 << 16);

        w.bits(value)
    });
    /*
     * Peripheral reset.
     */
    rcc.apb1hrstr().modify(|_, w| {
        w.fdcanrst().set_bit()
    });

    cortex_m::asm::nop();
    cortex_m::asm::nop();

    rcc.apb1hrstr().modify(|_, w| {
        w.fdcanrst().clear_bit()
    });

    let _ = rcc.apb1hrstr().read().bits();

    /*
     * INIT 모드 진입.
     */
    fdcan.cccr().modify(|_, w| {
        w.init().set_bit()
    });

    let mut timeout = TIMEOUT;

    while fdcan.cccr().read().init().bit_is_clear() {
        timeout -= 1;

        if timeout == 0 {
            return Err("FDCAN INIT timeout");
        }
    }

    /*
     * Configuration Change Enable.
     */
    fdcan.cccr().modify(|_, w| {
        w.cce().set_bit()
    });

    timeout = TIMEOUT;

    while fdcan.cccr().read().cce().bit_is_clear() {
        timeout -= 1;

        if timeout == 0 {
            return Err("FDCAN CCE timeout");
        }
    }

    /*
     * Classic CAN only.
     *
     * FDOE = 0
     * BRSE = 0
     * MON  = 0
     */
    fdcan.cccr().modify(|r, w| unsafe {
        let mut value = r.bits();

        value &= !(
            (1 << 5)  // MON
                | (1 << 8)  // FDOE
                | (1 << 9)  // BRSE
        );

        w.bits(value)
    });

    /*
     * 80 MHz → 500 kbit/s
     *
     * Prescaler = 10
     * TSEG1     = 13
     * TSEG2     = 2
     * SJW       = 2
     *
     * 80 MHz / 10 / (1 + 13 + 2)
     * = 500 kbit/s
     *
     * 레지스터에는 각 값 - 1 기록.
     */
    fdcan.nbtp().write(|w| unsafe {
        w.bits(
            (1 << 25)    // NSJW   = 1 → 2 TQ
                | (9 << 16) // NBRP   = 9 → /10
                | (12 << 8) // NTSEG1 = 12 → 13 TQ
                | 1          // NTSEG2 = 1 → 2 TQ
        )
    });

    /*
     * Internal loopback.
     *
     * CCCR.TEST = 1
     * TEST.LBCK = 1
     */
    // fdcan.cccr().modify(|r, w| unsafe {
    //     w.bits(r.bits() | (1 << 7))
    // });

    // fdcan.test().modify(|r, w| unsafe {
    //     w.bits(r.bits() | (1 << 4))
    // });
    fdcan.cccr().modify(|r, w| unsafe {
        w.bits(r.bits() & !(1 << 7)) // CCCR.TEST = 0
    });

    fdcan.test().modify(|r, w| unsafe {
        w.bits(r.bits() & !(1 << 4)) // TEST.LBCK = 0
    });
    /*
     * 모든 non-matching standard frame을
     * RX FIFO0으로 받는다.
     *
     * GFC = 0:
     * ANFS = 00 → FIFO0
     * ANFE = 00 → FIFO0
     */
    fdcan.gfc().write(|w| unsafe {
        w.bits(0)
    });

    /*
     * Standard/extended filter 없음.
     */
    fdcan.sidfc().write(|w| unsafe {
        w.bits(0)
    });

    fdcan.xidfc().write(|w| unsafe {
        w.bits(0)
    });

    /*
     * RX FIFO0:
     *
     * 시작 byte offset = 0x00
     * element 개수     = 1
     *
     * RXF0C:
     * F0SA = 0x00
     * F0S  = 1
     */
    fdcan.rxf0c().write(|w| unsafe {
        w.bits(
            (1 << 16)
                | RX_FIFO0_OFFSET as u32
        )
    });

    /*
     * RX element data size = 8 bytes.
     *
     * F0DS = 000
     * F1DS = 000
     * RBDS = 000
     */
    fdcan.rxesc().write(|w| unsafe {
        w.bits(0)
    });

    /*
     * TX dedicated buffer:
     *
     * TBSA = byte offset 0x10
     * NDTB = 1
     * TFQS = 0
     */
    fdcan.txbc().write(|w| unsafe {
        w.bits(
            TX_BUFFER0_OFFSET as u32
                | (1 << 16)
        )
    });

    /*
     * TX element data size = 8 bytes.
     */
    fdcan.txesc().write(|w| unsafe {
        w.bits(0)
    });

    /*
     * Message RAM 초기화.
     */
    unsafe {
        for offset in (0..0x20).step_by(4) {
            ram_write(offset, 0);
        }
    }

    cortex_m::asm::dmb();

    /*
     * TX buffer element 0 작성.
     *
     * T0:
     * Standard ID bits 28:18
     * RTR = 0
     * XTD = 0
     * ESI = 0
     */
    let tx_t0 =
        (TEST_CAN_ID as u32) << 18;

    /*
     * T1:
     * DLC = 8
     * BRS = 0
     * FDF = 0
     * EFC = 0
     */
    let tx_t1 =
        8u32 << 16;

    unsafe {
        ram_write(TX_BUFFER0_OFFSET + 0x00, tx_t0);
        ram_write(TX_BUFFER0_OFFSET + 0x04, tx_t1);
        ram_write(TX_BUFFER0_OFFSET + 0x08, TEST_DATA0);
        ram_write(TX_BUFFER0_OFFSET + 0x0C, TEST_DATA1);
    }

    cortex_m::asm::dmb();

    /*
     * 이전 interrupt flags 제거.
     * IR은 1을 쓰면 clear.
     */
    fdcan.ir().write(|w| unsafe {
        w.bits(0xFFFF_FFFF)
    });

    /*
     * INIT 해제 → 통신 시작.
     */
    fdcan.cccr().modify(|_, w| {
        w.init().clear_bit()
    });

    timeout = TIMEOUT;

    while fdcan.cccr().read().init().bit_is_set() {
        timeout -= 1;

        if timeout == 0 {
            return Err("FDCAN start timeout");
        }
    }

    /*
     * TX buffer 0 transmission request.
     */
    fdcan.txbar().write(|w| unsafe {
        w.bits(1)
    });

    /*
     * 내부 루프백으로 RX FIFO0에
     * 프레임이 들어올 때까지 대기.
     *
     * RXF0S.F0FL bits 6:0
     */
    let result = FdcanLoopbackResult {
        endn: fdcan.endn().read().bits(),
        cccr: fdcan.cccr().read().bits(),
        nbtp: fdcan.nbtp().read().bits(),
        test: fdcan.test().read().bits(),
        psr: fdcan.psr().read().bits(),
        ecr: fdcan.ecr().read().bits(),

        txbto: fdcan.txbto().read().bits(),
        rxf0s: fdcan.rxf0s().read().bits(),

        rx_id: 0,
        rx_dlc: 0,
        rx_data0: 0,
        rx_data1: 0,

        d2ccip1r: rcc.d2ccip1r().read().bits(),
        apb1henr: rcc.apb1henr().read().bits(),
    };
    Ok(result)
}