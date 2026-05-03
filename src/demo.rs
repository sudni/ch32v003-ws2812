use ch32v0::ch32v003::Peripherals;
use crate::ili9341::{tft_draw_pixel, tft_draw_sprite, tft_fill_dma, WIDTH, HEIGHT};
use crate::assets::SPRITE_32X32;

// A simple sine LUT generated from -127 to 127
const SINE_LUT: [i8; 256] = [
    0, 3, 6, 9, 12, 15, 18, 21, 24, 28, 31, 34, 37, 40, 43, 46, 48, 51, 54, 57, 60, 63, 65, 68,
    71, 73, 76, 78, 81, 83, 85, 88, 90, 92, 94, 96, 98, 100, 102, 104, 106, 107, 109, 111, 112, 113,
    115, 116, 117, 118, 120, 121, 122, 122, 123, 124, 125, 125, 126, 126, 126, 127, 127, 127, 127, 127,
    127, 127, 126, 126, 126, 125, 125, 124, 123, 122, 122, 121, 120, 118, 117, 116, 115, 113, 112, 111,
    109, 107, 106, 104, 102, 100, 98, 96, 94, 92, 90, 88, 85, 83, 81, 78, 76, 73, 71, 68, 65, 63, 60,
    57, 54, 51, 48, 46, 43, 40, 37, 34, 31, 28, 24, 21, 18, 15, 12, 9, 6, 3, 0, -3, -6, -9, -12, -15,
    -18, -21, -24, -28, -31, -34, -37, -40, -43, -46, -48, -51, -54, -57, -60, -63, -65, -68, -71,
    -73, -76, -78, -81, -83, -85, -88, -90, -92, -94, -96, -98, -100, -102, -104, -106, -107, -109,
    -111, -112, -113, -115, -116, -117, -118, -120, -121, -122, -122, -123, -124, -125, -125, -126,
    -126, -126, -127, -127, -127, -127, -127, -127, -127, -126, -126, -126, -125, -125, -124, -123,
    -122, -122, -121, -120, -118, -117, -116, -115, -113, -112, -111, -109, -107, -106, -104, -102,
    -100, -98, -96, -94, -92, -90, -88, -85, -83, -81, -78, -76, -73, -71, -68, -65, -63, -60, -57,
    -54, -51, -48, -46, -43, -40, -37, -34, -31, -28, -24, -21, -18, -15, -12, -9, -6, -3
];

fn sin(angle: u8) -> i8 {
    SINE_LUT[angle as usize]
}

fn cos(angle: u8) -> i8 {
    SINE_LUT[angle.wrapping_add(64) as usize]
}

const NUM_STARS: usize = 40;

struct Star {
    x: i8,
    y: i8,
    z: u8,
}

pub fn run_demo(p: &Peripherals) -> ! {
    // Fill screen black
    tft_fill_dma(p, 0x0000);

    let mut stars: [Star; NUM_STARS] = core::array::from_fn(|i| Star {
        x: ((i * 37) % 255) as i8,
        y: ((i * 59) % 255) as i8,
        z: ((i * 17) % 255) as u8,
    });

    let mut frame: u8 = 0;
    
    // Previous sprite coordinates to erase
    let mut prev_sx: u16 = 0;
    let mut prev_sy: u16 = 0;

    let text = b"SuDni like Rust & AI      ";
    let mut scroll_x: i32 = WIDTH as i32;

    loop {
        // --- 1. Starfield ---
        for star in stars.iter_mut() {
            // Erase old star
            if star.z > 0 {
                let sx = (WIDTH / 2) as i32 + ((star.x as i32 * 128) / star.z as i32);
                let sy = (HEIGHT / 2) as i32 + ((star.y as i32 * 128) / star.z as i32);
                if sx >= 0 && sx < WIDTH as i32 && sy >= 0 && sy < HEIGHT as i32 {
                    tft_draw_pixel(p, sx as u16, sy as u16, 0x0000); // Erase
                }
            }

            // Move star
            star.z = star.z.wrapping_sub(4);

            // Respawn star if it reached the camera or goes out of bounds
            if star.z == 0 {
                star.z = 255;
            }
            
            // Draw new star
            let sx = (WIDTH / 2) as i32 + ((star.x as i32 * 128) / star.z as i32);
            let sy = (HEIGHT / 2) as i32 + ((star.y as i32 * 128) / star.z as i32);
            if sx >= 0 && sx < WIDTH as i32 && sy >= 0 && sy < HEIGHT as i32 {
                // Dim further stars
                let color = if star.z > 200 {
                    0x4208
                } else if star.z > 100 {
                    0x8410
                } else {
                    0xFFFF
                };
                tft_draw_pixel(p, sx as u16, sy as u16, color);
            } else {
                star.z = 255; // Respawn if off-screen
            }
        }

        // --- 2. Bouncing Sprite ---
        // Calculate new position using sine LUT
        // screen is 320x240. Sprite is 32x32.
        // x goes from 0 to 320 - 32 = 288. Center is 144. Amplitude is 144.
        // y goes from 0 to 240 - 32 = 208. Center is 104. Amplitude is 104.
        
        let new_sx = (144 + (sin(frame.wrapping_mul(2)) as i32 * 144 / 127)) as u16;
        let new_sy = (104 + (cos(frame.wrapping_mul(3)) as i32 * 104 / 127)) as u16;

        // Erase old sprite using fill rect (dma) if we had a primitive, or just redraw black
        // Actually erasing 32x32 pixel by pixel or by block:
        // We can draw a 32x32 black square if it moved significantly.
        // A better way is to do a filled black rect over the old position.
        // To save code size, we can just send 1024 black pixels.
        // BUT, tft_fill_dma fills the WHOLE screen, not a window. Let's make a fill_window
        // Or we can just re-draw the background (which is black) using the same tft_draw_sprite, but we don't have a black array.
        // Actually, for a C64 demo, leaving trails or doing a block erase is common.
        
        // Let's implement a quick erase block function using spi_dma_fill16 windowed
        erase_block(p, prev_sx, prev_sy, 32, 32);

        // Draw new sprite
        tft_draw_sprite(p, new_sx, new_sy, 32, 32, &SPRITE_32X32);

        prev_sx = new_sx;
        prev_sy = new_sy;

        // --- 3. Scrolling Text ---
        let text_y = 220;

        for (i, &ch) in text.iter().enumerate() {
            let char_x = scroll_x + (i as i32 * 16);
            if char_x > -16 && char_x < WIDTH as i32 {
                // Color gradient effect based on character index and frame
                let color = if (i as u8).wrapping_add(frame / 4) % 2 == 0 {
                    0x07E0 // Green
                } else {
                    0xFFE0 // Yellow
                };
                draw_char_block(p, ch, char_x, text_y as i32, color);
            }
        }

        // Clean up the trailing edge left by the moving string (2 pixels wide)
        let trail_x = scroll_x + (text.len() as i32 * 16);
        if trail_x >= 0 && trail_x < WIDTH as i32 {
            let draw_w = core::cmp::min(2, WIDTH as i32 - trail_x) as u16;
            erase_block(p, trail_x as u16, text_y, draw_w, 16);
        }

        scroll_x -= 2;
        if scroll_x < -(text.len() as i32 * 16) {
            scroll_x = WIDTH as i32;
        }

        frame = frame.wrapping_add(1);

        // Small delay to keep the framerate reasonable and avoid tearing
        //delay_ms(10);
    }
}

// Helper to erase a block using DMA
fn erase_block(p: &Peripherals, x: u16, y: u16, w: u16, h: u16) {
    crate::ili9341::set_window(p, x, y, w, h);
    crate::ili9341::dc_data();
    crate::ili9341::cs_low();
    
    // Use the 16-bit DMA fill for black
    p.SPI1.ctlr1().modify(|_, w| w.dff().set_bit());
    static mut BLACK: u16 = 0x0000;
    crate::spi::spi_dma_fill16(p, &raw const BLACK, w as u32 * h as u32);
    p.SPI1.ctlr1().modify(|_, w| w.dff().clear_bit());
    crate::ili9341::cs_high();
}

fn draw_char_block(p: &Peripherals, ch: u8, x: i32, y: i32, color: u16) {
    static mut BUF: [u16; 256] = [0; 256];
    
    // Determine visible area
    let mut start_x = 0;
    let mut end_x = 16;
    if x < 0 { start_x = -x; }
    if x + 16 > WIDTH as i32 { end_x = WIDTH as i32 - x; }
    
    if start_x >= end_x { return; }
    
    let draw_w = (end_x - start_x) as u16;
    let draw_x = (x + start_x) as u16;
    
    // Convert color to big-endian for SPI
    let color_be = color.to_be();
    let bg_be = 0x0000;
    
    unsafe {
        // Clear buffer
        for i in 0..256 {
            BUF[i] = bg_be;
        }
        
        let glyph = &crate::font::FONT8X8[(ch & 0x7F) as usize];
        for (row_idx, &row_val) in glyph.iter().enumerate() {
            for col_idx in 0..8 {
                if (row_val & (1 << col_idx)) != 0 {
                    let base_idx = row_idx * 2 * 16 + col_idx * 2;
                    BUF[base_idx] = color_be;
                    BUF[base_idx + 1] = color_be;
                    BUF[base_idx + 16] = color_be;
                    BUF[base_idx + 17] = color_be;
                }
            }
        }
        
        crate::ili9341::set_window(p, draw_x, y as u16, draw_w, 16);
        crate::ili9341::dc_data();
        crate::ili9341::cs_low();
        
        if draw_w == 16 {
            // Full character visible, single DMA transfer
            let ptr = core::ptr::addr_of!(BUF) as *const u8;
            let buf_u8 = core::slice::from_raw_parts(ptr, 512);
            crate::spi::spi_dma_tx(p, buf_u8);
        } else {
            // Clipped character, row by row
            for row in 0..16 {
                let row_start = row * 16 + start_x as usize;
                let ptr = (core::ptr::addr_of!(BUF) as *const u16).add(row_start) as *const u8;
                let slice = core::slice::from_raw_parts(ptr, (draw_w * 2) as usize);
                crate::spi::spi_dma_tx(p, slice);
            }
        }
        crate::ili9341::cs_high();
    }
}
