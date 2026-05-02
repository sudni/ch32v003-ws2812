#![no_std]
#![no_main]

use ch32v0::ch32v003::Peripherals;
use panic_halt as _;

mod backlight;
mod delay;
mod ili9341;
mod spi;

use backlight::{init_backlight, set_backlight};
use delay::delay_ms;
//use delay::delay_us;

use ili9341::{cs_high, rst_high, tft_fill_dma, tft_init};

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
            .bits(0b000) // /2 → 24 MHz
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

    // ── 6. Demo: fill screen with alternating colours ─────────────────────────
    let colors: [u16; 224] = [
        0xF800, 0xF820, 0xF860, 0xF8A0, 0xF8C0, 0xF900, 0xF940, 0xF960, 0xF9A0, 0xF9E0, 0xFA00,
        0xFA40, 0xFA80, 0xFAA0, 0xFAE0, 0xFB20, 0xFB60, 0xFB80, 0xFBC0, 0xFC00, 0xFC20, 0xFC60,
        0xFCA0, 0xFCC0, 0xFD00, 0xFD40, 0xFD60, 0xFDA0, 0xFDE0, 0xFE00, 0xFE40, 0xFE80, 0xFEC0,
        0xFEE0, 0xFF20, 0xFF60, 0xFF80, 0xFFC0, 0xF7E0, 0xEFE0, 0xE7E0, 0xDFE0, 0xDFE0, 0xD7E0,
        0xCFE0, 0xC7E0, 0xBFE0, 0xB7E0, 0xB7E0, 0xAFE0, 0xA7E0, 0x9FE0, 0x97E0, 0x8FE0, 0x8FE0,
        0x87E0, 0x7FE0, 0x77E0, 0x6FE0, 0x6FE0, 0x67E0, 0x5FE0, 0x57E0, 0x4FE0, 0x47E0, 0x47E0,
        0x3FE0, 0x37E0, 0x2FE0, 0x27E0, 0x1FE0, 0x1FE0, 0x17E0, 0x0FE0, 0x07E0, 0x07E0, 0x07E1,
        0x07E1, 0x07E2, 0x07E3, 0x07E4, 0x07E5, 0x07E6, 0x07E6, 0x07E7, 0x07E8, 0x07E9, 0x07EA,
        0x07EB, 0x07EB, 0x07EC, 0x07ED, 0x07EE, 0x07EF, 0x07F0, 0x07F0, 0x07F1, 0x07F2, 0x07F3,
        0x07F4, 0x07F5, 0x07F5, 0x07F6, 0x07F7, 0x07F8, 0x07F9, 0x07FA, 0x07FA, 0x07FB, 0x07FC,
        0x07FD, 0x07FE, 0x07FF, 0x07BF, 0x077F, 0x073F, 0x071F, 0x06DF, 0x069F, 0x067F, 0x063F,
        0x05FF, 0x05DF, 0x059F, 0x055F, 0x053F, 0x04FF, 0x04BF, 0x047F, 0x045F, 0x041F, 0x03DF,
        0x03BF, 0x037F, 0x033F, 0x031F, 0x02DF, 0x029F, 0x027F, 0x023F, 0x01FF, 0x01DF, 0x019F,
        0x015F, 0x013F, 0x00FF, 0x00BF, 0x007F, 0x005F, 0x001F, 0x001F, 0x081F, 0x101F, 0x181F,
        0x181F, 0x201F, 0x281F, 0x301F, 0x381F, 0x401F, 0x401F, 0x481F, 0x501F, 0x581F, 0x601F,
        0x681F, 0x681F, 0x701F, 0x781F, 0x801F, 0x881F, 0x881F, 0x901F, 0x981F, 0xA01F, 0xA81F,
        0xB01F, 0xB01F, 0xB81F, 0xC01F, 0xC81F, 0xD01F, 0xD81F, 0xD81F, 0xE01F, 0xE81F, 0xF01F,
        0xF81E, 0xF81D, 0xF81D, 0xF81C, 0xF81B, 0xF81A, 0xF819, 0xF818, 0xF818, 0xF817, 0xF816,
        0xF815, 0xF814, 0xF813, 0xF813, 0xF812, 0xF811, 0xF810, 0xF80F, 0xF80E, 0xF80E, 0xF80D,
        0xF80C, 0xF80B, 0xF80A, 0xF809, 0xF809, 0xF808, 0xF807, 0xF806, 0xF805, 0xF804, 0xF804,
        0xF803, 0xF802, 0xF801, 0xF800,
    ];

    let mut idx = 0usize;
    set_backlight(&p, 50);
    loop {
        // Fade out backlight quickly
        // for level in (0..=20).rev() {
        //     set_backlight(&p, level);
        //     delay_ms(1);
        // }
        //set_backlight(&p, 0);

        // Draw the new frame invisibly
        tft_fill_dma(&p, colors[idx % colors.len()]);
        idx = idx.wrapping_add(1);

        // Fade in backlight quickly
        // for level in 0..=20 {
        //     set_backlight(&p, );
        //     delay_ms(1);
        // }
        //set_backlight(&p, 20);

        // Wait before next transition
        delay_ms(20);
    }
}
