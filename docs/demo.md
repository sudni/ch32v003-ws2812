# C64-Style Demo Documentation

This document describes the C64-style demo implemented in this project, focusing on the graphic effects, functions used, and how it strictly fits within the memory budget of the CH32V003 microcontroller (16KB Flash, 2KB SRAM).

## Overview

The demo showcases classic demoscene techniques, tailored for extreme memory constraints. Instead of utilizing a framebuffer—which would require at least 153.6 KB of SRAM for a 320x240 16-bit display—the demo leverages math (via a precomputed LUT) and localized drawing updates directly to the TFT over SPI with DMA.

The project is configured for **Landscape Mode (320x240)**.

## Memory Budget Allocation

Given the CH32V003 specifications:
- **Total Flash**: 16 KB
- **Total SRAM**: 2 KB

**Current Usage Profile (Release Build):**
*   **Flash (text)**: ~7.3 KB (8.7 KB free)
    *   32x32 Sprite Array (`assets.rs`): 2,048 bytes
    *   8x8 ASCII Font Array (`font.rs`): 1,024 bytes
    *   Sine Look-Up Table (`demo.rs`): 256 bytes
    *   ILI9341 Initialization & System code: ~4 KB
*   **SRAM (bss/data)**: ~518 bytes + Stack
    *   The `Star` array uses 120 bytes (40 stars * 3 bytes) allocated on the stack.
    *   `BUF` for characters uses 512 bytes (16x16 pixels * 2 bytes) as a static mut buffer.
    *   A healthy ~1.3 KB of SRAM remains available for future expansion.

---

## Effects

### 1. 3D Starfield
A classic infinite starfield simulation using a perspective projection.
- **Logic**: A fixed number of stars (40) are tracked in 3D space (`x`, `y`, `z`). As the z-coordinate decreases, the star moves "closer" to the camera.
- **Rendering**: The `(x, y)` world coordinates are divided by the depth (`z`) to generate the on-screen 2D coordinates `(sx, sy)`.
- **Erase Strategy**: Rather than clearing the whole screen, the previous position of each star is overwritten with a single black pixel just before computing and drawing the new position. This saves significant bandwidth and CPU time.
- **Depth Shading**: The color of the star dims based on the `z` distance to simulate atmospheric depth.

### 2. Bouncing Sprite (Sine Wave Movement)
A 32x32 pixel pre-rendered circular sprite smoothly animates around the screen, bouncing mathematically.
- **Sine LUT**: A `SINE_LUT` containing 256 entries is used to generate realistic movement oscillations. `cos(x)` is derived by reading `SINE_LUT[x + 64]`.
- **Erase Strategy**: The sprite's previous position is erased by utilizing `spi_dma_fill16` windowed drawing over the exact 32x32 area. This blasts 1024 black pixels extremely quickly using DMA before rendering the new frame.

### 3. Anti-Flicker Scrolling Text
A smooth-scrolling banner "SuDni like Rust & AI" moves across the bottom of the screen.
- **Double-Buffered Blocks**: To prevent flickering, each 8x8 character is scaled 2x and rendered into a local 16x16 SRAM buffer (`BUF`). This buffer includes both the character pixels and the black background pixels.
- **Flicker-Free Rendering**: The entire 16x16 block is sent to the TFT in a single DMA burst. This eliminates the "erase-then-draw" flicker common in low-memory systems.
- **Trail Eraser**: A specialized 2-pixel wide vertical strip is erased at the trailing edge of the text string to cleanly remove the pixels left behind by the scroll motion without flickering the rest of the banner.

---

## Key Functions

### In `src/ili9341.rs`

*   **`set_window(p, x, y, w, h)`**
    Configures the ILI9341 drawing window (Column `0x2A` and Row `0x2B`).
*   **`tft_draw_pixel(p, x, y, color)`**
    Sets a 1x1 window and sends a single 16-bit color.
*   **`tft_draw_sprite(p, x, y, w, h, data)`**
    Sets a window corresponding to the sprite bounds and blasts the data over SPI using DMA.

### In `src/demo.rs`

*   **`run_demo(p)`**
    The main execution loop for the demo.
*   **`erase_block(p, x, y, w, h)`**
    An optimized function that clears a specific rectangular area using `spi_dma_fill16` via DMA.
*   **`draw_char_block(p, ch, x, y, color)`**
    The anti-flicker character renderer. It renders a scaled 8x8 character into a 16x16 SRAM buffer and performs a block DMA transfer. It includes built-in X-axis clipping for smooth entries/exits.
*   **`sin(angle)` & `cos(angle)`**
    Helper functions to index the 256-byte `SINE_LUT` in flash memory.

## Future Expansion
The remaining ~9 KB of flash and ~1.3 KB of SRAM easily allows for:
- Plasma effects (streaming procedural rows of pixels using DMA).
- Wireframe 3D cubes.
- More sprites and complex movement patterns.
