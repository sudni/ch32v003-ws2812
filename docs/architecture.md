# Software Architecture

## Execution flow

```
main()
  │
  ├─ 1. Enable clocks
  │     ├─ RCC.AHBPCENR  → DMA1EN  (DMA1 on AHB bus)
  │     └─ RCC.APB2PCENR → SPI1EN + IOPCEN + IOPDEN + AFIOEN
  │
  ├─ 2. Configure GPIO
  │     ├─ GPIOC: PC1=GP-OUT(CS), PC5=AF-PP(SCK), PC6=AF-PP(MOSI)
  │     └─ GPIOD: PD3=GP-OUT(DC), PD4=GP-OUT(RST)
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
  └─ 5. Main loop
        └─ tft_fill_dma(color)   ← repeated every 100 ms, cycling 8 colors
              ├─ Turn Display OFF (0x28) to hide drawing
              ├─ Set window (0x2A / 0x2B / 0x2C)
              ├─ Switch SPI to 16-bit frames (DFF=1)
              ├─ For each of 320 rows → spi_dma_fill16(&PIXEL)
              ├─ Restore SPI to 8-bit frames (DFF=0)
              └─ Turn Display ON (0x29) for instant reveal

## Project Structure

The codebase is split into specific modules for modularity and maintainability:

- `src/main.rs`: Entry point, clock setup, GPIO config, SPI init, and the main draw loop.
- `src/ili9341.rs`: TFT control pins (`cs`, `dc`, `rst`), init sequence, and `tft_fill_dma`.
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
| `tft_fill_dma()` | Fill full screen, row by row | DMA |

## DMA transfer sequence (`spi_dma_tx`)

```
1.  Disable DMA1 Channel 3
2.  Clear TC/HT/TE interrupt flags for Ch3
3.  Write PADDR3 ← SPI1_DATAR (0x4001_300C)
4.  Write MADDR3 ← buf.as_ptr()
5.  Write CNTR3  ← buf.len()
6.  Write CFGR3  ← DIR=1, MINC=1, PSIZE=00, MSIZE=00, PL=10, EN=1
7.  Poll INTFR bit 9 (TC3) until set
8.  Poll SPI1.STATR.BSY until clear
```

## Why row-by-row DMA instead of a full framebuffer?

The CH32V003 has only **2 KB of RAM**.  
A full 240×320 RGB565 framebuffer would require **153 600 bytes** — 75× more than available.

Instead, a single 2-byte pixel variable (`static mut PIXEL: u16`) is used with DMA `MINC=0` (fixed memory address increment) and 16-bit transfers to fill the screen without any row buffers. This provides a minimal memory footprint while still getting the speed benefits of DMA.

To hide the drawing process, the display is temporarily turned off (`0x28`) and turned back on (`0x29`) when the fill is complete.

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
