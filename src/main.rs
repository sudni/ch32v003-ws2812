#![no_std]
#![no_main]

use ch32v0::ch32v003::Peripherals;
use panic_halt as _;

mod assets;
mod backlight;
mod delay;
mod demo;
mod font;
mod ili9341;
mod spi;

use backlight::{init_backlight, set_backlight};
//use delay::delay_ms;
//use delay::delay_us;

use ili9341::{cs_high, rst_high, tft_init};

// =============================================================================
// Pin mapping (CH32V003 TSSOP20 / SOP16)
// =============================================================================
// SPI1_SCK   → PC5   (AF push-pull)
// SPI1_MOSI  → PC6   (AF push-pull)
// SPI1_NSS   → PC1   (software CS, GPIO output)
// TFT_DC     → PD3   (GPIO output, high=data / low=command)
// TFT_RST    → PD4   (GPIO output, active-low reset)
//
// Wiring to QVGA 2.2" TFT SPI 240×320:
//   VCC   → 3.3 V
//   GND   → GND
//   CS    → PC1
//   RESET → PD4
//   DC/RS → PD3
//   SDI/MOSI → PC6
//   SCK   → PC5
//   LED   → 3.3 V (or PWM if dimming needed)
//   SDO/MISO → NC  (not connected – write-only driver)
// =============================================================================

// =============================================================================
// Entry point
// =============================================================================
#[qingke_rt::entry]
fn main() -> ! {
    let p = Peripherals::take().unwrap();

    // ── 0. Configure System Clock to 48MHz using 24MHz HSE ────────────────────
    // 1. Enable HSE (External High-Speed Oscillator) and CSS (Clock Security System)
    p.RCC
        .ctlr()
        .modify(|_, w| w.hseon().set_bit().csson().set_bit());
    while p.RCC.ctlr().read().hserdy().bit_is_clear() {}

    // 2. Set Flash Latency (1 wait state required for frequencies > 24MHz)
    p.FLASH.actlr().modify(|_, w| w.latency().set_bit());

    // 3. Configure PLL: Select HSE as source and enable PLL
    // Note: CH32V003 PLL multiplier is fixed at x2. 24MHz HSE * 2 = 48MHz.
    p.RCC
        .cfgr0()
        .modify(|_, w| unsafe { w.pllsrc().bits(0b01) });
    p.RCC.ctlr().modify(|_, w| w.pllon().set_bit());
    while p.RCC.ctlr().read().pllrdy().bit_is_clear() {}

    // 4. Switch System Clock (SYSCLK) to PLL output
    p.RCC.cfgr0().modify(|_, w| unsafe { w.sw().bits(0b10) });
    while p.RCC.cfgr0().read().sws().bits() != 0b10 {}

    // ── 1. Enable clocks ──────────────────────────────────────────────────────
    // DMA1 on AHB bus
    p.RCC.ahbpcenr().modify(|_, w| w.dma1en().set_bit());

    // SPI1, GPIOC, GPIOD, AFIO on APB2 bus
    p.RCC.apb2pcenr().modify(|_, w| {
        w.spi1en()
            .set_bit()
            .iopcen()
            .set_bit() // GPIOC: PC1(CS), PC5(SCK), PC6(MOSI)
            .iopden()
            .set_bit() // GPIOD: PD3(DC), PD4(RST)
            .afioen()
            .set_bit() // AFIO for SPI alternate function
    });

    // ── 2. Configure GPIO pins ────────────────────────────────────────────────
    // GPIOC CFGLR:
    //   PC1 (CS)   → Output push-pull 10 MHz  : MODE=01, CNF=00
    //   PC5 (SCK)  → AF push-pull 50 MHz      : MODE=11, CNF=10
    //   PC6 (MOSI) → AF push-pull 50 MHz      : MODE=11, CNF=10
    p.GPIOC.cfglr().modify(|_, w| unsafe {
        w.mode1()
            .bits(0b01)
            .cnf1()
            .bits(0b00) // PC1 = CS (GP output)
            .mode5()
            .bits(0b11)
            .cnf5()
            .bits(0b10) // PC5 = SCK (AF PP 50 MHz)
            .mode6()
            .bits(0b11)
            .cnf6()
            .bits(0b10) // PC6 = MOSI (AF PP 50 MHz)
    });

    // GPIOD CFGLR:
    //   PD3 (DC)  → Output push-pull 10 MHz : MODE=01, CNF=00
    //   PD4 (RST) → Output push-pull 10 MHz : MODE=01, CNF=00
    p.GPIOD.cfglr().modify(|_, w| unsafe {
        w.mode3()
            .bits(0b01)
            .cnf3()
            .bits(0b00) // PD3 = DC
            .mode4()
            .bits(0b01)
            .cnf4()
            .bits(0b00) // PD4 = RST
    });

    // Deassert CS and RST to known states
    cs_high();
    rst_high();

    // ── 3. Configure SPI1 ─────────────────────────────────────────────────────
    // Reset SPI1
    p.RCC.apb2prstr().modify(|_, w| w.spi1rst().set_bit());
    p.RCC.apb2prstr().modify(|_, w| w.spi1rst().clear_bit());

    // CTLR1:
    //   MSTR=1   master mode
    //   SSM=1    software NSS management
    //   SSI=1    internal NSS high (we drive CS manually)
    //   CPOL=0   clock idle low
    //   CPHA=0   data captured on first edge  (mode 0)
    //   DFF=0    8-bit frame
    //   BR=000   fPCLK/2  → 48MHz/2 = 24 MHz (max ILI9341 write = 25 MHz)
    //   LSBFIRST=0 MSB first
    //   SPE=1    enable
    p.SPI1.ctlr1().write(|w| unsafe {
        w.mstr()
            .set_bit()
            .ssm()
            .set_bit()
            .ssi()
            .set_bit()
            .cpol()
            .clear_bit()
            .cpha()
            .clear_bit()
            .dff()
            .clear_bit()
            .br()
            .bits(0b001) // /4 → 12 MHz (more stable than 24MHz)
            .lsbfirst()
            .clear_bit()
            .spe()
            .set_bit()
    });

    // CTLR2: enable TX DMA request
    p.SPI1.ctlr2().modify(|_, w| w.txdmaen().set_bit());

    // ── 4. Initialise TFT display ─────────────────────────────────────────────
    tft_init(&p);

    // ── 5. Initialize PWM Backlight on PD2 ────────────────────────────────────
    init_backlight(&p);

    // ── 6. Run C64-like Demo ──────────────────────────────────────────────────
    set_backlight(&p, 50);
    crate::demo::run_demo(&p);
}
