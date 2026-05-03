use crate::delay::delay_ms;

use crate::spi::{spi_dma_tx, spi_tx_byte};
use ch32v0::ch32v003::Peripherals;
use core::ptr;

// Screen resolution
pub const WIDTH: u16 = 320;
pub const HEIGHT: u16 = 240;

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

    // Memory access control: row/col order, BGR (Landscape)
    tft_cmd_data(p, 0x36, &[0x28]);

    // Column address set: 0..319
    tft_cmd_data(p, 0x2A, &[0x00, 0x00, 0x01, 0x3F]);

    // Row address set: 0..239
    tft_cmd_data(p, 0x2B, &[0x00, 0x00, 0x00, 0xEF]);

    // Display on
    tft_cmd(p, 0x29);
    delay_ms(10);
}

// ── Fill screen with a solid RGB565 colour using DMA ─────────────────────────
pub fn tft_fill_dma(p: &Peripherals, color: u16) {
    // Set write window to full screen
    tft_cmd_data(p, 0x2A, &[0x00, 0x00, 0x01, 0x3F]); // column 0-319
    tft_cmd_data(p, 0x2B, &[0x00, 0x00, 0x00, 0xEF]); // row 0-239

    // Begin pixel write
    tft_cmd(p, 0x2C);

    // Single pixel source (converted to 2 bytes)
    let color_high = (color >> 8) as u8;
    let color_low = (color & 0xFF) as u8;

    dc_data();
    cs_low();
    // Stream pixels in row-sized chunks
    const CHUNK_COUNT: u32 = WIDTH as u32 * HEIGHT as u32;
    for _ in 0..CHUNK_COUNT {
        crate::spi::spi_tx_byte(p, color_high);
        crate::spi::spi_tx_byte(p, color_low);
    }
    cs_high();
}

pub fn set_window(p: &Peripherals, x: u16, y: u16, w: u16, h: u16) {
    let x2 = x + w - 1;
    let y2 = y + h - 1;
    tft_cmd_data(p, 0x2A, &[(x >> 8) as u8, x as u8, (x2 >> 8) as u8, x2 as u8]);
    tft_cmd_data(p, 0x2B, &[(y >> 8) as u8, y as u8, (y2 >> 8) as u8, y2 as u8]);
    tft_cmd(p, 0x2C);
}

pub fn tft_draw_sprite(p: &Peripherals, x: u16, y: u16, w: u16, h: u16, data: &[u8]) {
    set_window(p, x, y, w, h);
    dc_data();
    cs_low();
    spi_dma_tx(p, data);
    cs_high();
}

pub fn tft_draw_pixel(p: &Peripherals, x: u16, y: u16, color: u16) {
    set_window(p, x, y, 1, 1);
    dc_data();
    cs_low();
    spi_tx_byte(p, (color >> 8) as u8);
    spi_tx_byte(p, color as u8);
    cs_high();
}
