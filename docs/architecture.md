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
        └─ tft_fill_dma(color)   ← repeated every 500 ms, cycling 4 colors
              ├─ Set window (0x2A / 0x2B / 0x2C)
              ├─ Fill ROW_BUF[480] with RGB565 big-endian bytes
              └─ For each of 320 rows → spi_dma_tx(ROW_BUF)
```

## Module overview

| Function | Role | Transfer method |
|----------|------|-----------------|
| `spi_tx_byte()` | Send 1 byte (polling) | Blocking / no DMA |
| `tft_cmd()` | Send command byte | Blocking |
| `tft_data()` | Send data byte | Blocking |
| `tft_cmd_data()` | Send command + N data bytes | Blocking |
| `tft_init()` | ILI9341 init sequence | Blocking |
| `spi_dma_tx()` | Send N bytes via DMA1 Ch3 | DMA + poll TC flag |
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

Instead, a single 480-byte row tile (`static mut ROW_BUF`) is filled once per unique colour  
and then DMA'd 320 times. For arbitrary pixel-level graphics, the window would be set  
to individual tiles or lines before each DMA burst.

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
