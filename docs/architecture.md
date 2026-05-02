# Software Architecture

## Execution flow

```
main()
  │
  ├─ 0. Configure System Clock
  │     ├─ Enable HSE (24 MHz External Crystal)
  │     ├─ Enable CSS (Clock Security System)
  │     ├─ Set Flash Latency to 1 wait state
  │     └─ Enable PLL (x2 multiplier) → 48 MHz SYSCLK
  │
  ├─ 1. Enable peripheral clocks
  │     ├─ RCC.AHBPCENR  → DMA1EN  (DMA1 on AHB bus)
  │     └─ RCC.APB2PCENR → SPI1EN + TIM1EN + IOPCEN + IOPDEN + AFIOEN
  │
  ├─ 2. Configure GPIO
  │     ├─ GPIOC: PC1=GP-OUT(CS), PC5=AF-PP(SCK), PC6=AF-PP(MOSI)
  │     └─ GPIOD: PD2=AF-PP(PWM), PD3=GP-OUT(DC), PD4=GP-OUT(RST)
  │
  ├─ 3. Configure SPI1
  │     ├─ Reset SPI1 via RCC.APB2PRSTR
  │     ├─ CTLR1: Master, Mode 0, 8-bit, fPCLK/2 (24 MHz), SSM/SSI
  │     └─ CTLR2: TXDMAEN=1  (link SPI TX-empty → DMA request)
  │
  ├─ 4. TFT init (tft_init)
  │     ├─ Hardware reset sequence (RST low 20 ms → high)
  │     ├─ Software reset (0x01) + sleep-out (0x11)
  │     ├─ Pixel format 16-bit RGB565 (0x3A, 0x55)
  │     ├─ Memory access control (0x36, 0x48)
  │     ├─ Column/Row address set (0x2A / 0x2B)
  │     └─ Display ON (0x29)
  │
  ├─ 5. Backlight init (init_backlight)
  │     ├─ TIM1_CH1 PWM config (PSC=47, ARR=100)
  │     └─ PD2 AF-PP mode
  │
  └─ 6. Main loop
        └─ tft_fill_dma(color)   ← repeated every 20 ms, cycling 224 rainbow colors
              ├─ Set window (0x2A / 0x2B / 0x2C)
              ├─ Switch SPI to 16-bit frames (DFF=1)
              ├─ 2 bursts of 38,400 words → spi_dma_fill16(&PIXEL)
              ├─ Wait for SPI BSY to clear
              └─ Restore SPI to 8-bit frames (DFF=0)

## Project Structure

The codebase is split into specific modules for modularity and maintainability:

- `src/main.rs`: Entry point, clock setup, GPIO config, SPI init, and the main draw loop.
- `src/ili9341.rs`: TFT control pins (`cs`, `dc`, `rst`), init sequence, and `tft_fill_dma`.
- `src/backlight.rs`: Backlight PWM control using TIM1_CH1 on PD2.
- `src/spi.rs`: SPI and DMA-specific functions (`spi_dma_tx`, `spi_dma_fill16`, `spi_tx_byte`).
- `src/delay.rs`: Low-level busy-wait delay functions (`delay_us`, `delay_ms`).

## Module overview

| Function | Role | Transfer method |
|----------|------|-----------------|
| `spi_tx_byte()` | Send 1 byte (polling) | Blocking / no DMA |
| `spi_dma_tx()` | Send N bytes via DMA1 Ch3 | DMA + poll TC flag |
| `spi_dma_fill16()` | Send repeating 16-bit word | DMA (MINC=0) |
| `tft_cmd()`, `tft_data()` | Send command / data byte | Blocking |
| `tft_init()` | ILI9341 init sequence | Blocking |
| `init_backlight()` | TIM1 PWM initialization | Blocking |
| `set_backlight()` | Set PWM duty cycle | Blocking |
| `tft_fill_dma()` | Fill full screen in 2 large bursts | DMA |

## DMA transfer sequence (`spi_dma_tx`)

```
1.  Disable DMA1 Channel 3
2.  Clear TC/HT/TE interrupt flags for Ch3
3.  Write PADDR3 ← SPI1_DATAR (0x4001_300C)
4.  Write MADDR3 ← buf.as_ptr()
5.  Write CNTR3  ← buf.len()
6.  Write CFGR3  ← DIR=1, MINC=1, PSIZE=00, MSIZE=00, PL=10, EN=1
7.  Poll INTFR bit 9 (TC3) until set (DMA is done, SPI may still be busy)
```

> **Note:** `spi_dma_tx` and `spi_dma_fill16` no longer wait for the SPI `BSY` flag. This allows for faster "chaining" of DMA blocks. The caller must manually check `BSY` before deasserting Chip Select (CS) or switching modes.

## Why row-by-row DMA instead of a full framebuffer?

The CH32V003 has only **2 KB of RAM**.  
A full 240×320 RGB565 framebuffer would require **153 600 bytes** — 75× more than available.

Instead, a single 2-byte pixel variable (`static mut PIXEL: u16`) is used with DMA `MINC=0` (fixed memory address increment) and 16-bit transfers to fill the screen. Since the DMA counter is 16-bit (max 65,535), the 76,800 pixel transfer is split into **two 38,400-pixel bursts**. This significantly reduces CPU overhead compared to row-by-row transfers.

The transfer speed is high enough that the display update is nearly invisible at 24 MHz SPI, allowing the backlight to remain at a constant level (e.g., 50%) without perceptible flicker.

## Debugging Configuration

The project is configured for advanced hardware debugging using:
- **Cortex-Debug**: VS Code extension for RISC-V/ARM debugging.
- **WCH-Link**: Hardware debugger support via `wlink` or `openocd`.
- **SVD Support**: `ch32v003.svd` is included to enable peripheral register viewing in the debugger.
- **Pre-launch Task**: Automatic `cargo build` before each debug session.

## Timing estimates

| Operation | Approx. time |
|-----------|-------------|
| 1 byte SPI @ 24 MHz | ~0.33 µs |
| 1 row (480 B) via DMA @ 24 MHz | ~160 µs |
| Full screen fill (320 rows) | ~51 ms |
| Init sequence delays | ~800 ms total |

## Key constants

```rust
const SPI1_DATAR: u32 = 0x4001_300C;  // SPI1 base 0x4001_3000 + offset 0x0C
const WIDTH:  u16 = 240;
const HEIGHT: u16 = 320;

// GPIO output registers (direct raw pointer access)
const GPIOC_OUTDR: *mut u32 = 0x4001_100C as *mut u32;
const GPIOD_OUTDR: *mut u32 = 0x4001_140C as *mut u32;
```
