#![no_std]
#![no_main]

use ch32v0::ch32v003::Peripherals;
use core::ptr;
use panic_halt as _;

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

// DMA1 Channel 3 is hardwired to SPI1_TX on CH32V003.
// SPI1_DATAR address (from RM, offset 0x0C from SPI1 base 0x4001_3000)
const SPI1_DATAR: u32 = 0x4001_300C;

// Screen resolution
const WIDTH: u16 = 240;
const HEIGHT: u16 = 320;

// ── Tiny busy-wait delay ──────────────────────────────────────────────────────
#[inline(always)]
fn delay_us(us: u32) {
    // CH32V003 runs at 48 MHz after HSI trim; ~48 cycles/µs.
    // Each iteration is roughly 4 cycles → 12 iterations per µs.
    let loops = us * 12;
    for _ in 0..loops {
        unsafe { core::arch::asm!("nop") };
    }
}

#[inline(always)]
fn delay_ms(ms: u32) {
    delay_us(ms * 1000);
}

// ── GPIO helpers ──────────────────────────────────────────────────────────────
#[inline(always)]
fn pin_set(outdr: *mut u32, bit: u8) {
    unsafe { ptr::write_volatile(outdr, ptr::read_volatile(outdr) | (1u32 << bit)) };
}

#[inline(always)]
fn pin_clr(outdr: *mut u32, bit: u8) {
    unsafe { ptr::write_volatile(outdr, ptr::read_volatile(outdr) & !(1u32 << bit)) };
}

// ── DMA helpers ───────────────────────────────────────────────────────────────

/// Send `len` **bytes** from `buf` over SPI1 via DMA1 Ch3 (8-bit, MINC=1).
/// Used for command argument payloads where every byte differs.
/// Caller must assert CS and set DC beforehand.
#[allow(dead_code)]
fn spi_dma_tx(p: &Peripherals, buf: &[u8]) {
    if buf.is_empty() {
        return;
    }
    let dma = &p.DMA1;

    // Disable channel before reconfiguring
    dma.cfgr3().modify(|_, w| w.en().clear_bit());

    // Clear all interrupt flags for ch3 (bits 8..11 in INTFCR)
    dma.intfcr().write(|w| unsafe { w.bits(0x0F00) });

    // Peripheral address → SPI1 data register
    dma.paddr3().write(|w| unsafe { w.bits(SPI1_DATAR) });

    // Memory address → start of buffer
    dma.maddr3()
        .write(|w| unsafe { w.bits(buf.as_ptr() as u32) });

    // Transfer count (bytes)
    dma.cntr3().write(|w| unsafe { w.bits(buf.len() as u32) });

    // Configure channel:
    //   DIR=1   memory→peripheral
    //   MINC=1  auto-increment memory address
    //   PINC=0  peripheral address fixed
    //   PSIZE=00 (8-bit)  MSIZE=00 (8-bit)
    //   PL=10   high priority
    //   CIRC=0  one-shot
    dma.cfgr3().write(|w| unsafe {
        w.dir()
            .set_bit() // mem→periph
            .minc()
            .set_bit() // increment memory ptr
            .pinc()
            .clear_bit()
            .psize()
            .bits(0b00) // 8-bit peripheral
            .msize()
            .bits(0b00) // 8-bit memory
            .pl()
            .bits(0b10) // high priority
            .circ()
            .clear_bit()
            .mem2mem()
            .clear_bit()
            .en()
            .set_bit()
    });

    // Wait for transfer complete (TC flag = bit 9 of INTFR)
    while p.DMA1.intfr().read().bits() & (1 << 9) == 0 {}

    // Wait until SPI is not busy before deasserting CS
    while p.SPI1.statr().read().bsy().bit_is_set() {}
}

/// Repeat a single **16-bit** pixel value `count` times over SPI1 via DMA1 Ch3.
///
/// Key differences from `spi_dma_tx`:
///   - PSIZE = MSIZE = 01  → 16-bit transfers (SPI DFF must be 1)
///   - MINC  = 0           → DMA reads the *same* memory address every time
///
/// This lets us fill the whole screen from a single `u16` without any buffer.
/// The SPI peripheral must already be configured for 16-bit frames (DFF=1)
/// before calling this, and restored to 8-bit afterwards.
/// Caller must assert CS and set DC=data beforehand.
fn spi_dma_fill16(p: &Peripherals, pixel: *const u16, count: u32) {
    if count == 0 {
        return;
    }
    let dma = &p.DMA1;

    // Disable channel before reconfiguring
    dma.cfgr3().modify(|_, w| w.en().clear_bit());

    // Clear all interrupt flags for ch3
    dma.intfcr().write(|w| unsafe { w.bits(0x0F00) });

    // Peripheral address → SPI1 data register
    dma.paddr3().write(|w| unsafe { w.bits(SPI1_DATAR) });

    // Memory address → the single pixel variable
    dma.maddr3().write(|w| unsafe { w.bits(pixel as u32) });

    // Transfer count (number of 16-bit words)
    dma.cntr3().write(|w| unsafe { w.bits(count) });

    // Configure channel:
    //   DIR=1   memory→peripheral
    //   MINC=0  fixed source address (repeat same pixel)
    //   PINC=0  peripheral address fixed
    //   PSIZE=01 (16-bit)  MSIZE=01 (16-bit)
    //   PL=10   high priority
    //   CIRC=0  one-shot
    dma.cfgr3().write(|w| unsafe {
        w.dir()
            .set_bit() // mem→periph
            .minc()
            .clear_bit() // ← fixed: no memory increment
            .pinc()
            .clear_bit()
            .psize()
            .bits(0b01) // 16-bit peripheral
            .msize()
            .bits(0b01) // 16-bit memory
            .pl()
            .bits(0b10) // high priority
            .circ()
            .clear_bit()
            .mem2mem()
            .clear_bit()
            .en()
            .set_bit()
    });

    // Wait for transfer complete (TC flag = bit 9 of INTFR)
    while p.DMA1.intfr().read().bits() & (1 << 9) == 0 {}

    // Wait until SPI is not busy before deasserting CS
    while p.SPI1.statr().read().bsy().bit_is_set() {}
}

// ── Low-level SPI (blocking, no DMA) for short commands ──────────────────────
fn spi_tx_byte(p: &Peripherals, byte: u8) {
    // Wait for TXE
    while p.SPI1.statr().read().txe().bit_is_clear() {}
    p.SPI1.datar().write(|w| unsafe { w.bits(byte as u32) });
    // Wait until not busy
    while p.SPI1.statr().read().bsy().bit_is_set() {}
}

// ── TFT control pin helpers ───────────────────────────────────────────────────
// GPIOD OUTDR is at base+0x0C; we use raw pointers to avoid borrow issues.
const GPIOC_OUTDR: *mut u32 = 0x4001_100C as *mut u32;
const GPIOD_OUTDR: *mut u32 = 0x4001_140C as *mut u32;

// PC1 = CS, PD3 = DC, PD4 = RST
#[inline(always)]
fn cs_low() {
    pin_clr(GPIOC_OUTDR, 1);
}
#[inline(always)]
fn cs_high() {
    pin_set(GPIOC_OUTDR, 1);
}
#[inline(always)]
fn dc_cmd() {
    pin_clr(GPIOD_OUTDR, 3);
} // DC low  = command
#[inline(always)]
fn dc_data() {
    pin_set(GPIOD_OUTDR, 3);
} // DC high = data
#[inline(always)]
fn rst_low() {
    pin_clr(GPIOD_OUTDR, 4);
}
#[inline(always)]
fn rst_high() {
    pin_set(GPIOD_OUTDR, 4);
}

// ── TFT command helpers ───────────────────────────────────────────────────────
fn tft_cmd(p: &Peripherals, cmd: u8) {
    dc_cmd();
    cs_low();
    spi_tx_byte(p, cmd);
    cs_high();
}

fn tft_data(p: &Peripherals, data: u8) {
    dc_data();
    cs_low();
    spi_tx_byte(p, data);
    cs_high();
}

fn tft_cmd_data(p: &Peripherals, cmd: u8, args: &[u8]) {
    tft_cmd(p, cmd);
    for &b in args {
        tft_data(p, b);
    }
}

// ── ILI9341 / ST7789 initialisation sequence ─────────────────────────────────
fn tft_init(p: &Peripherals) {
    // Hardware reset
    rst_high();
    delay_ms(10);
    rst_low();
    delay_ms(20);
    rst_high();
    delay_ms(150);

    // Software reset
    tft_cmd(p, 0x01);
    delay_ms(150);

    // Sleep out
    tft_cmd(p, 0x11);
    delay_ms(255);

    // Pixel format: 16-bit (RGB565)
    tft_cmd_data(p, 0x3A, &[0x55]);

    // Memory access control: row/col order, BGR
    tft_cmd_data(p, 0x36, &[0x48]);

    // Column address set: 0..239
    tft_cmd_data(p, 0x2A, &[0x00, 0x00, 0x00, 0xEF]);

    // Row address set: 0..319
    tft_cmd_data(p, 0x2B, &[0x00, 0x00, 0x01, 0x3F]);

    // Display on
    tft_cmd(p, 0x29);
    delay_ms(10);
}

// ── Fill screen with a solid RGB565 colour using DMA ─────────────────────────
//
// Optimised for minimum RAM usage:
//   • A single `static mut PIXEL: u16` holds the colour (2 bytes, not 480).
//   • DMA is configured in 16-bit mode with MINC=0 so it reads the same
//     address 76 800 times (WIDTH × HEIGHT), sending every pixel in one shot.
//   • SPI is temporarily switched to 16-bit frame mode (DFF=1) for the fill,
//     then restored to 8-bit afterwards.
//
// RAM saved vs. the old ROW_BUF approach: 480 − 2 = 478 bytes.
fn tft_fill_dma(p: &Peripherals, color: u16) {
    // Turn display OFF to hide the drawing process
    tft_cmd(p, 0x28);

    // Set write window to full screen
    tft_cmd_data(p, 0x2A, &[0x00, 0x00, 0x00, 0xEF]); // column 0-239
    tft_cmd_data(p, 0x2B, &[0x00, 0x00, 0x01, 0x3F]); // row 0-319

    // Begin pixel write
    tft_cmd(p, 0x2C);

    // Single pixel source – only 2 bytes of RAM needed.
    // `static mut` keeps it out of the stack (critical in 2 KB RAM).
    static mut PIXEL: u16 = 0;

    // Switch SPI to 16-bit frame mode for the fill transfer
    p.SPI1.ctlr1().modify(|_, w| w.dff().set_bit());

    // Stream pixels in row-sized chunks to avoid the 16-bit DMA counter limit
    dc_data();
    cs_low();
    unsafe {
        PIXEL = color;
        for _ in 0..HEIGHT {
            spi_dma_fill16(p, &raw const PIXEL, WIDTH as u32);
        }
    }
    cs_high();

    // Restore SPI to 8-bit frame mode for subsequent commands
    p.SPI1.ctlr1().modify(|_, w| w.dff().clear_bit());

    // Turn display ON to reveal the updated screen instantly
    tft_cmd(p, 0x29);
}

// =============================================================================
// Entry point
// =============================================================================
#[qingke_rt::entry]
fn main() -> ! {
    let p = Peripherals::take().unwrap();

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

    // ── 5. Demo: fill screen with alternating colours ─────────────────────────
    let colors: [u16; 4] = [
        0xF800, // Red
        0x07E0, // Green
        0x001F, // Blue
        0xFFFF, // White
    ];

    let mut idx = 0usize;
    loop {
        tft_fill_dma(&p, colors[idx % colors.len()]);
        idx = idx.wrapping_add(1);
        delay_ms(50);
    }
}
