use ch32v0::ch32v003::Peripherals;
use core::ptr;
use crate::spi::{spi_tx_byte, spi_dma_fill16};
use crate::delay::delay_ms;

// Screen resolution
pub const WIDTH: u16 = 240;
pub const HEIGHT: u16 = 320;

// ── GPIO helpers ──────────────────────────────────────────────────────────────
#[inline(always)]
fn pin_set(outdr: *mut u32, bit: u8) {
    unsafe { ptr::write_volatile(outdr, ptr::read_volatile(outdr) | (1u32 << bit)) };
}

#[inline(always)]
fn pin_clr(outdr: *mut u32, bit: u8) {
    unsafe { ptr::write_volatile(outdr, ptr::read_volatile(outdr) & !(1u32 << bit)) };
}

// ── TFT control pin helpers ───────────────────────────────────────────────────
// GPIOD OUTDR is at base+0x0C; we use raw pointers to avoid borrow issues.
const GPIOC_OUTDR: *mut u32 = 0x4001_100C as *mut u32;
const GPIOD_OUTDR: *mut u32 = 0x4001_140C as *mut u32;

// PC1 = CS, PD3 = DC, PD4 = RST
#[inline(always)]
pub fn cs_low() {
    pin_clr(GPIOC_OUTDR, 1);
}
#[inline(always)]
pub fn cs_high() {
    pin_set(GPIOC_OUTDR, 1);
}
#[inline(always)]
pub fn dc_cmd() {
    pin_clr(GPIOD_OUTDR, 3);
} // DC low  = command
#[inline(always)]
pub fn dc_data() {
    pin_set(GPIOD_OUTDR, 3);
} // DC high = data
#[inline(always)]
pub fn rst_low() {
    pin_clr(GPIOD_OUTDR, 4);
}
#[inline(always)]
pub fn rst_high() {
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
pub fn tft_init(p: &Peripherals) {
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
pub fn tft_fill_dma(p: &Peripherals, color: u16) {
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
