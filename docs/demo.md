# C64-Style Demo Documentation

This document describes the demoscene effects implemented in this project, optimized for the CH32V003 microcontroller (16KB Flash, 2KB SRAM).

## Overview

The demo showcases classic demoscene techniques using direct TFT register access and DMA, without a full framebuffer.

## Memory Budget Allocation

**Current Usage Profile (Release Build):**
*   **Flash (text)**: ~9.5 KB (6.5 KB free)
    *   32x32 Sprite Array: 2,048 bytes
    *   8x8 ASCII Font Array: 1,024 bytes
    *   Sine Look-Up Table: 256 bytes
    *   3D Math & Line Drawing: ~1.5 KB
*   **SRAM (bss/data)**: ~1,240 bytes + Stack
    *   `COMMON_BUF`: 1,152 bytes (Shared between all effects to maximize efficiency).
    *   Available stack: ~800 bytes

---

## Effects

### 1. C64 Scene (Starfield & Scroller)
- **3D Starfield**: 40 stars with perspective projection.
- **Bouncing Sprite**: 32x32 sprite oscillating via Sine LUT.
- **Scrolling Text**: Anti-flicker banner using scaled font rendering into `BUF`.

### 2. Procedural Plasma
- **Math**: Three overlapping sine waves generating color indices.
- **Optimization**: Row-by-row calculation and DMA streaming for high framerate.

### 3. Rotating 3D Wireframe Cube
- **3D Engine**: Uses fixed-point integer math and the precomputed `SINE_LUT` for rotations.
- **Flicker-Free Rendering**: Uses a 1-bit local bitmask (80x80) stored in `COMMON_BUF`.
- **DMA Streaming**: The bitmask is expanded to 16-bit colors row-by-row and streamed via DMA. This ensures perfectly smooth animation with zero flickering.

---

## Key Functions (src/demo.rs)

*   **`run_demo(p)`**: Main sequencer (Scene 1 → Scene 2 → Scene 3).
*   **`run_cube_demo(p, frames)`**: Manages the 3D vertex rotation and edge rendering.
*   **`draw_line(p, x0, y0, x1, y1, color)`**: Bresenham line algorithm implementation.
