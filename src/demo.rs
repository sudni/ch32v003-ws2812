use crate::assets::SPRITE_32X32;
use crate::ili9341::{tft_draw_pixel, tft_draw_sprite, tft_fill_dma, HEIGHT, WIDTH};
use ch32v0::ch32v003::Peripherals;

// A simple sine LUT generated from -127 to 127
const SINE_LUT: [i8; 256] = [
    0, 3, 6, 9, 12, 15, 18, 21, 24, 28, 31, 34, 37, 40, 43, 46, 48, 51, 54, 57, 60, 63, 65, 68, 71,
    73, 76, 78, 81, 83, 85, 88, 90, 92, 94, 96, 98, 100, 102, 104, 106, 107, 109, 111, 112, 113,
    115, 116, 117, 118, 120, 121, 122, 122, 123, 124, 125, 125, 126, 126, 126, 127, 127, 127, 127,
    127, 127, 127, 126, 126, 126, 125, 125, 124, 123, 122, 122, 121, 120, 118, 117, 116, 115, 113,
    112, 111, 109, 107, 106, 104, 102, 100, 98, 96, 94, 92, 90, 88, 85, 83, 81, 78, 76, 73, 71, 68,
    65, 63, 60, 57, 54, 51, 48, 46, 43, 40, 37, 34, 31, 28, 24, 21, 18, 15, 12, 9, 6, 3, 0, -3, -6,
    -9, -12, -15, -18, -21, -24, -28, -31, -34, -37, -40, -43, -46, -48, -51, -54, -57, -60, -63,
    -65, -68, -71, -73, -76, -78, -81, -83, -85, -88, -90, -92, -94, -96, -98, -100, -102, -104,
    -106, -107, -109, -111, -112, -113, -115, -116, -117, -118, -120, -121, -122, -122, -123, -124,
    -125, -125, -126, -126, -126, -127, -127, -127, -127, -127, -127, -127, -126, -126, -126, -125,
    -125, -124, -123, -122, -122, -121, -120, -118, -117, -116, -115, -113, -112, -111, -109, -107,
    -106, -104, -102, -100, -98, -96, -94, -92, -90, -88, -85, -83, -81, -78, -76, -73, -71, -68,
    -65, -63, -60, -57, -54, -51, -48, -46, -43, -40, -37, -34, -31, -28, -24, -21, -18, -15, -12,
    -9, -6, -3,
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

// Common buffer to share SRAM between different effects (1152 bytes total)
// Use u32 to ensure 4-byte alignment
static mut COMMON_BUF: [u32; 288] = [0; 288];

// Helpers to access the common buffer
unsafe fn get_buf_u16() -> &'static mut [u16; 576] {
    core::mem::transmute(&raw mut COMMON_BUF)
}

unsafe fn get_buf_u8() -> &'static mut [u8; 1152] {
    core::mem::transmute(&raw mut COMMON_BUF)
}

pub fn run_demo(p: &Peripherals) -> ! {
    loop {
        run_c64_demo(p, 1000);
        run_plasma_demo(p, 200);
        run_cube_demo(p, 500);
        run_cone_demo(p, 500);
        run_torus_demo(p, 500);
        run_word_demo(p, 500);
    }
}

pub fn run_c64_demo(p: &Peripherals, frames: u32) {
    // Fill screen black
    tft_fill_dma(p, 0x0000);

    let mut stars: [Star; NUM_STARS] = core::array::from_fn(|i| Star {
        x: ((i * 37) % 255) as i8,
        y: ((i * 59) % 255) as i8,
        z: ((i * 17) % 255) as u8,
    });

    let mut frame: u16 = 0;

    // Previous sprite coordinates to erase
    let mut prev_sx: u16 = 0;
    let mut prev_sy: u16 = 0;

    let text = b"SuDni like Rust & AI      ";
    let mut scroll_x: i32 = WIDTH as i32;

    for _ in 0..frames {
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
        let f8 = frame as u8;
        let new_sx = (144 + (sin(f8.wrapping_mul(2)) as i32 * 144 / 127)) as u16;
        let new_sy = (104 + (cos(f8.wrapping_mul(3)) as i32 * 104 / 127)) as u16;

        erase_block(p, prev_sx, prev_sy, 34, 34); // Increased size to ensure full clear
        tft_draw_sprite(p, new_sx, new_sy, 32, 32, &SPRITE_32X32);

        prev_sx = new_sx;
        prev_sy = new_sy;

        // --- 3. Scrolling Text ---
        let text_y = 220;
        for (i, &ch) in text.iter().enumerate() {
            let char_x = scroll_x + (i as i32 * 16);
            if char_x > -16 && char_x < WIDTH as i32 {
                let color = if (i as u8).wrapping_add((frame / 4) as u8) % 2 == 0 {
                    0x07E0 // Green
                } else {
                    0xFFE0 // Yellow
                };
                draw_char_block(p, ch, char_x, text_y as i32, color);
            }
        }

        let trail_x = scroll_x + (text.len() as i32 * 16);
        if trail_x >= 0 && trail_x < WIDTH as i32 {
            let draw_w = core::cmp::min(4, WIDTH as i32 - trail_x) as u16;
            erase_block(p, trail_x as u16, text_y, draw_w, 16);
        }

        scroll_x -= 3; // Slightly faster scroll
        if scroll_x < -(text.len() as i32 * 16) {
            scroll_x = WIDTH as i32;
        }

        frame = frame.wrapping_add(1);
    }
}

fn run_plasma_demo(p: &Peripherals, frames: u32) {
    // Fill screen black
    tft_fill_dma(p, 0x0000);

    // 1. Generate Palette into COMMON_BUF[0..512]
    unsafe {
        let buf = get_buf_u16();
        for i in 0..256 {
            let idx = i as u8;
            let r = (sin(idx) as i16 + 127) >> 3; // 0..31
            let g = (sin(idx.wrapping_add(85)) as i16 + 127) >> 2; // 0..63
            let b = (sin(idx.wrapping_add(170)) as i16 + 127) >> 3; // 0..31
            let color = ((r as u16) << 11) | ((g as u16) << 5) | (b as u16);
            buf[i] = color.to_be();
        }
    }

    let mut t: u8 = 0;
    for _ in 0..frames {
        crate::ili9341::set_window(p, 0, 0, WIDTH, HEIGHT);
        crate::ili9341::dc_data();
        crate::ili9341::cs_low();

        for y in 0..HEIGHT {
            let vy = y as u8;
            let v2 = sin(vy.wrapping_add(t.wrapping_mul(2)));
            let offset1 = t;
            let offset2 = vy.wrapping_add(t);

            for x in 0..WIDTH {
                let vx = x as u8;
                let v1 = sin(vx.wrapping_add(offset1));
                let v3 = sin(vx.wrapping_add(offset2));
                let idx = v1.wrapping_add(v2).wrapping_add(v3) as u8;

                // Optimized palette-free calculation
                let r = (sin(idx) as i16 + 127) >> 3; // 0..31
                let g = (sin(idx.wrapping_add(85)) as i16 + 127) >> 2; // 0..63
                let b = (sin(idx.wrapping_add(170)) as i16 + 127) >> 3; // 0..31
                let color = ((r as u16) << 11) | ((g as u16) << 5) | (b as u16);

                crate::spi::spi_tx_byte(p, (color >> 8) as u8);
                crate::spi::spi_tx_byte(p, (color & 0xFF) as u8);
            }
        }
        crate::ili9341::cs_high();
        t = t.wrapping_add(1);
    }
}

fn run_cube_demo(p: &Peripherals, frames: u32) {
    tft_fill_dma(p, 0x0000);

    let vertices = [
        (-15, -15, -15),
        (15, -15, -15),
        (15, 15, -15),
        (-15, 15, -15),
        (-15, -15, 15),
        (15, -15, 15),
        (15, 15, 15),
        (-15, 15, 15),
    ];

    let edges = [
        (0, 1),
        (1, 2),
        (2, 3),
        (3, 0),
        (4, 5),
        (5, 6),
        (6, 7),
        (7, 4),
        (0, 4),
        (1, 5),
        (2, 6),
        (3, 7),
    ];

    let mut angle_x: u8 = 0;
    let mut angle_y: u8 = 0;

    const BOX_SIZE: usize = 80;
    const BOX_X: u16 = (WIDTH - BOX_SIZE as u16) / 2;
    const BOX_Y: u16 = (HEIGHT - BOX_SIZE as u16) / 2;

    for _ in 0..frames {
        unsafe {
            let buf = get_buf_u8();
            // Clear bitmask area (80x80 / 8 = 800 bytes)
            for i in 0..800 {
                buf[i] = 0;
            }

            // 1. Project and draw to bitmask
            for edge in edges.iter() {
                let v1 = vertices[edge.0];
                let v2 = vertices[edge.1];

                let p1 = project(v1, angle_x, angle_y);
                let p2 = project(v2, angle_x, angle_y);

                draw_line_bitmask(buf, p1.0 + 40, p1.1 + 40, p2.0 + 40, p2.1 + 40);
            }

            // 2. Transfer bitmask to TFT (flicker-free!)
            let buf16 = get_buf_u16();
            let red_be = 0xF800u16.to_be();
            let black_be = 0x0000u16.to_be();

            crate::ili9341::set_window(p, BOX_X, BOX_Y, BOX_SIZE as u16, BOX_SIZE as u16);
            crate::ili9341::dc_data();
            crate::ili9341::cs_low();

            for y in 0..BOX_SIZE {
                for x in 0..BOX_SIZE {
                    let byte_idx = (y * BOX_SIZE + x) / 8;
                    let bit_idx = (y * BOX_SIZE + x) % 8;
                    let color = if (buf[byte_idx] & (1 << bit_idx)) != 0 {
                        red_be
                    } else {
                        black_be
                    };
                    buf16[400 + x] = color;
                }

                let ptr = buf16.as_ptr().add(400) as *const u8;
                let slice = core::slice::from_raw_parts(ptr, BOX_SIZE * 2);
                crate::spi::spi_dma_tx(p, slice);
            }
            crate::ili9341::cs_high();
        }

        angle_x = angle_x.wrapping_add(2);
        angle_y = angle_y.wrapping_add(3);
    }
}

fn run_cone_demo(p: &Peripherals, frames: u32) {
    tft_fill_dma(p, 0x0000);

    // Cone vertices: 0 = apex, 1..13 = base points (12-point circle)
    let mut vertices: [(i8, i8, i8); 13] = [(0, 0, 0); 13];
    vertices[0] = (0, 0, 20); // Apex
    for i in 0..12 {
        let angle = (i * 256 / 12) as u8;
        vertices[i + 1] = (
            (sin(angle.wrapping_add(64)) as i32 * 15 / 128) as i8,
            (sin(angle) as i32 * 15 / 128) as i8,
            -15,
        );
    }

    // Cone edges: 12 base edges + 12 apex edges
    let mut edges: [(usize, usize); 24] = [(0, 0); 24];
    for i in 0..12 {
        edges[i] = (i + 1, if i == 11 { 1 } else { i + 2 }); // Base
        edges[i + 12] = (0, i + 1); // To Apex
    }

    let mut angle_x: u8 = 0;
    let mut angle_y: u8 = 0;

    const BOX_SIZE: usize = 80;
    const BOX_X: u16 = (WIDTH - BOX_SIZE as u16) / 2;
    const BOX_Y: u16 = (HEIGHT - BOX_SIZE as u16) / 2;

    for _ in 0..frames {
        unsafe {
            let buf = get_buf_u8();
            // Clear bitmask area
            for i in 0..800 {
                buf[i] = 0;
            }

            // Project and draw to bitmask
            for edge in edges.iter() {
                let v1 = vertices[edge.0];
                let v2 = vertices[edge.1];
                let p1 = project(v1, angle_x, angle_y);
                let p2 = project(v2, angle_x, angle_y);
                draw_line_bitmask(buf, p1.0 + 40, p1.1 + 40, p2.0 + 40, p2.1 + 40);
            }

            // Transfer bitmask to TFT (Cyan color for variety)
            let buf16 = get_buf_u16();
            let cyan_be = 0x07FFu16.to_be();
            let black_be = 0x0000u16.to_be();

            crate::ili9341::set_window(p, BOX_X, BOX_Y, BOX_SIZE as u16, BOX_SIZE as u16);
            crate::ili9341::dc_data();
            crate::ili9341::cs_low();

            for y in 0..BOX_SIZE {
                for x in 0..BOX_SIZE {
                    let byte_idx = (y * BOX_SIZE + x) / 8;
                    let bit_idx = (y * BOX_SIZE + x) % 8;
                    let color = if (buf[byte_idx] & (1 << bit_idx)) != 0 {
                        cyan_be
                    } else {
                        black_be
                    };
                    buf16[400 + x] = color;
                }

                let ptr = buf16.as_ptr().add(400) as *const u8;
                let slice = core::slice::from_raw_parts(ptr, BOX_SIZE * 2);
                crate::spi::spi_dma_tx(p, slice);
            }
            crate::ili9341::cs_high();
        }

        angle_x = angle_x.wrapping_add(3);
        angle_y = angle_y.wrapping_add(1);
    }
}

fn run_torus_demo(p: &Peripherals, frames: u32) {
    tft_fill_dma(p, 0x0000);

    // Torus parameters
    const R: i32 = 20; // Major radius
    const R2: i32 = 8; // Minor radius
    const STEPS_T: usize = 12; // Theta segments
    const STEPS_P: usize = 8; // Phi segments

    // Precalculate vertices (12*8 = 96 vertices)
    let mut vertices: [(i8, i8, i8); 96] = [(0, 0, 0); 96];
    for i in 0..STEPS_T {
        let theta = (i * 256 / STEPS_T) as u8;
        let cos_t = cos(theta) as i32;
        let sin_t = sin(theta) as i32;

        for j in 0..STEPS_P {
            let phi = (j * 256 / STEPS_P) as u8;
            let cos_p = cos(phi) as i32;
            let sin_p = sin(phi) as i32;

            // x = (R + r*cos_p) * cos_t
            // y = (R + r*cos_p) * sin_t
            // z = r * sin_p
            let dist = R + (R2 * cos_p / 128);
            vertices[i * STEPS_P + j] = (
                (dist * cos_t / 128) as i8,
                (dist * sin_t / 128) as i8,
                (R2 * sin_p / 128) as i8,
            );
        }
    }

    let mut angle_x: u8 = 0;
    let mut angle_y: u8 = 0;

    const BOX_SIZE: usize = 80;
    const BOX_X: u16 = (WIDTH - BOX_SIZE as u16) / 2;
    const BOX_Y: u16 = (HEIGHT - BOX_SIZE as u16) / 2;

    for _ in 0..frames {
        unsafe {
            let buf = get_buf_u8();
            // Clear bitmask area
            for i in 0..800 {
                buf[i] = 0;
            }

            // Project all vertices
            let mut projected: [(i16, i16); 96] = [(0, 0); 96];
            for i in 0..96 {
                let p = project(vertices[i], angle_x, angle_y);
                projected[i] = (p.0 + 40, p.1 + 40);
            }

            // Draw edges
            for i in 0..STEPS_T {
                for j in 0..STEPS_P {
                    let idx = i * STEPS_P + j;
                    // Edge along phi (minor circle)
                    let next_p = i * STEPS_P + (j + 1) % STEPS_P;
                    let p1 = projected[idx];
                    let p2 = projected[next_p];
                    draw_line_bitmask(buf, p1.0, p1.1, p2.0, p2.1);

                    // Edge along theta (major circle)
                    let next_t = ((i + 1) % STEPS_T) * STEPS_P + j;
                    let p3 = projected[next_t];
                    draw_line_bitmask(buf, p1.0, p1.1, p3.0, p3.1);
                }
            }

            // Transfer bitmask to TFT (Magenta color)
            let buf16 = get_buf_u16();
            let magenta_be = 0xF81Fu16.to_be();
            let black_be = 0x0000u16.to_be();

            crate::ili9341::set_window(p, BOX_X, BOX_Y, BOX_SIZE as u16, BOX_SIZE as u16);
            crate::ili9341::dc_data();
            crate::ili9341::cs_low();

            for y in 0..BOX_SIZE {
                for x in 0..BOX_SIZE {
                    let byte_idx = (y * BOX_SIZE + x) / 8;
                    let bit_idx = (y * BOX_SIZE + x) % 8;
                    let color = if (buf[byte_idx] & (1 << bit_idx)) != 0 {
                        magenta_be
                    } else {
                        black_be
                    };
                    buf16[400 + x] = color;
                }

                let ptr = buf16.as_ptr().add(400) as *const u8;
                let slice = core::slice::from_raw_parts(ptr, BOX_SIZE * 2);
                crate::spi::spi_dma_tx(p, slice);
            }
            crate::ili9341::cs_high();
        }

        angle_x = angle_x.wrapping_add(2);
        angle_y = angle_y.wrapping_add(3);
    }
}

fn run_word_demo(p: &Peripherals, frames: u32) {
    tft_fill_dma(p, 0x0000);

    // Manual vector font for "SuDnI"
    // Each letter is a set of vertices (x, y, z)
    // We'll use z=2 and z=-2 to give it some depth
    let mut v: [(i8, i8, i8); 60] = [(0, 0, 0); 60];
    let mut e: [(usize, usize); 80] = [(0, 0); 80];
    let mut v_count = 0;
    let mut e_count = 0;

    macro_rules! add_seg {
        ($x1:expr, $y1:expr, $x2:expr, $y2:expr) => {
            v[v_count] = ($x1, $y1, 2);
            v[v_count + 1] = ($x2, $y2, 2);
            v[v_count + 2] = ($x1, $y1, -2);
            v[v_count + 3] = ($x2, $y2, -2);

            e[e_count] = (v_count, v_count + 1); // Front
            e[e_count + 1] = (v_count + 2, v_count + 3); // Back
            e[e_count + 2] = (v_count, v_count + 2); // Connect 1
            e[e_count + 3] = (v_count + 1, v_count + 3); // Connect 2

            v_count += 4;
            e_count += 4;
        };
    }

    // S
    add_seg!(-30, 10, -20, 10);
    add_seg!(-30, 10, -30, 0);
    add_seg!(-30, 0, -20, 0);
    add_seg!(-20, 0, -20, -10);
    add_seg!(-20, -10, -30, -10);
    // u
    add_seg!(-15, 0, -15, -10);
    add_seg!(-15, -10, -5, -10);
    add_seg!(-5, -10, -5, 0);
    // D
    add_seg!(0, 10, 0, -10);
    add_seg!(0, 10, 10, 0);
    add_seg!(10, 0, 0, -10);
    // n
    add_seg!(15, -10, 15, 0);
    add_seg!(15, 0, 25, 0);
    add_seg!(25, 0, 25, -10);
    // I
    add_seg!(30, 10, 30, -10);

    let mut angle_x: u8 = 0;
    let mut angle_y: u8 = 0;

    const BOX_SIZE: usize = 80;
    const BOX_X: u16 = (WIDTH - BOX_SIZE as u16) / 2;
    const BOX_Y: u16 = (HEIGHT - BOX_SIZE as u16) / 2;

    for _ in 0..frames {
        unsafe {
            let buf = get_buf_u8();
            for i in 0..800 {
                buf[i] = 0;
            }

            for i in 0..e_count {
                let v1 = v[e[i].0];
                let v2 = v[e[i].1];
                let p1 = project(v1, angle_x, angle_y);
                let p2 = project(v2, angle_x, angle_y);
                draw_line_bitmask(buf, p1.0 + 40, p1.1 + 40, p2.0 + 40, p2.1 + 40);
            }

            let buf16 = get_buf_u16();
            let green_be = 0x07E0u16.to_be();
            let black_be = 0x0000u16.to_be();

            crate::ili9341::set_window(p, BOX_X, BOX_Y, BOX_SIZE as u16, BOX_SIZE as u16);
            crate::ili9341::dc_data();
            crate::ili9341::cs_low();

            for y in 0..BOX_SIZE {
                for x in 0..BOX_SIZE {
                    let byte_idx = (y * BOX_SIZE + x) / 8;
                    let bit_idx = (y * BOX_SIZE + x) % 8;
                    let color = if (buf[byte_idx] & (1 << bit_idx)) != 0 {
                        green_be
                    } else {
                        black_be
                    };
                    buf16[400 + x] = color;
                }
                let ptr = buf16.as_ptr().add(400) as *const u8;
                crate::spi::spi_dma_tx(p, core::slice::from_raw_parts(ptr, BOX_SIZE * 2));
            }
            crate::ili9341::cs_high();
        }
        angle_x = angle_x.wrapping_add(1);
        angle_y = angle_y.wrapping_add(2);
    }
}

fn project(v: (i8, i8, i8), angle_x: u8, angle_y: u8) -> (i16, i16) {
    let mut x = v.0 as i32;
    let mut y = v.1 as i32;
    let mut z = v.2 as i32;

    // Rotate X
    let s = sin(angle_x) as i32;
    let c = cos(angle_x) as i32;
    let ny = (y * c - z * s) / 128;
    let nz = (y * s + z * c) / 128;
    y = ny;
    z = nz;

    // Rotate Y
    let s = sin(angle_y) as i32;
    let c = cos(angle_y) as i32;
    let nx = (x * c + z * s) / 128;
    let nz = (-x * s + z * c) / 128;
    x = nx;
    z = nz;

    // Project
    let focal = 200;
    let pz = z + 160; // Increased from 100 to 160 to prevent clipping
    (
        ((x * focal) / pz as i32) as i16,
        ((y * focal) / pz as i32) as i16,
    )
}

fn draw_line_bitmask(buf: &mut [u8], mut x0: i16, mut y0: i16, x1: i16, y1: i16) {
    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;

    loop {
        if x0 >= 0 && x0 < 80 && y0 >= 0 && y0 < 80 {
            let idx = y0 as usize * 80 + x0 as usize;
            buf[idx / 8] |= 1 << (idx % 8);
        }
        if x0 == x1 && y0 == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x0 += sx;
        }
        if e2 <= dx {
            err += dx;
            y0 += sy;
        }
    }
}

// Helper to erase a block using DMA
fn erase_block(p: &Peripherals, mut x: u16, mut y: u16, mut w: u16, mut h: u16) {
    // Clipping
    if x >= WIDTH || y >= HEIGHT {
        return;
    }
    if x + w > WIDTH {
        w = WIDTH - x;
    }
    if y + h > HEIGHT {
        h = HEIGHT - y;
    }
    if w == 0 || h == 0 {
        return;
    }

    crate::ili9341::set_window(p, x, y, w, h);
    crate::ili9341::dc_data();
    crate::ili9341::cs_low();

    let count = w as u32 * h as u32 * 2;
    for _ in 0..count {
        crate::spi::spi_tx_byte(p, 0x00);
    }

    crate::ili9341::cs_high();
}

fn draw_char_block(p: &Peripherals, ch: u8, x: i32, y: i32, color: u16) {
    let color_be = color.to_be();
    let bg_be = 0x0000;

    unsafe {
        let buf = get_buf_u16();
        for i in 0..256 {
            buf[i] = bg_be;
        }

        let glyph = &crate::font::FONT8X8[(ch & 0x7F) as usize];
        for (row_idx, &row_val) in glyph.iter().enumerate() {
            for col_idx in 0..8 {
                if (row_val & (1 << col_idx)) != 0 {
                    let base_idx = row_idx * 2 * 16 + col_idx * 2;
                    buf[base_idx] = color_be;
                    buf[base_idx + 1] = color_be;
                    buf[base_idx + 16] = color_be;
                    buf[base_idx + 17] = color_be;
                }
            }
        }

        // --- Clipping Logic ---
        let mut clip_x = x;
        let mut clip_y = y as i32;
        let mut clip_w = 16;
        let mut clip_h = 16;
        let mut src_x = 0;

        if clip_x < 0 {
            src_x = -clip_x;
            clip_w -= src_x as u16;
            clip_x = 0;
        }
        if clip_x + clip_w as i32 > WIDTH as i32 {
            clip_w = (WIDTH as i32 - clip_x) as u16;
        }
        if clip_y < 0 || clip_y >= HEIGHT as i32 {
            return;
        }

        if clip_w > 0 {
            crate::ili9341::set_window(p, clip_x as u16, clip_y as u16, clip_w, clip_h);
            crate::ili9341::dc_data();
            crate::ili9341::cs_low();

            // Send row by row if clipped, otherwise full block
            if clip_w == 16 {
                let ptr = buf.as_ptr() as *const u8;
                crate::spi::spi_dma_tx(p, core::slice::from_raw_parts(ptr, 512));
            } else {
                for row in 0..16 {
                    let row_ptr = buf.as_ptr().add(row * 16 + src_x as usize) as *const u8;
                    crate::spi::spi_dma_tx(
                        p,
                        core::slice::from_raw_parts(row_ptr, clip_w as usize * 2),
                    );
                }
            }
            crate::ili9341::cs_high();
        }
    }
}
