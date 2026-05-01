# CH32V003 — TFT SPI Display with DMA

Bare-metal Rust project driving a **QVGA 2.2″ TFT SPI 240×320** display  
from a **WCH CH32V003** RISC-V microcontroller using **SPI1 + DMA1** for fast pixel transfers.

## Documentation index

| File | Contents |
|------|----------|
| [hardware.md](hardware.md) | Pin mapping, wiring table, schematic notes |
| [architecture.md](architecture.md) | Software architecture & execution flow |
| [peripherals.md](peripherals.md) | SPI1 & DMA1 register-level configuration |
| [tft_protocol.md](tft_protocol.md) | ILI9341/ST7789 command reference used |
| [build_flash.md](build_flash.md) | Toolchain setup, build & flash instructions |
| [memory.md](memory.md) | Memory layout & RAM usage analysis |

## Quick start

```bash
# Build
cargo build

# Flash (WCH-Link)
cargo build --release
wlink flash target/riscv32ec-unknown-none-elf/release/ch32v003-blinky
```

## Project overview

```
ch32v003-ws2812/
├── src/
│   ├── main.rs               # Main entry point and initialization
│   ├── ili9341.rs            # TFT display driver and DMA fill routines
│   ├── spi.rs                # SPI configuration and DMA transfers
│   └── delay.rs              # Busy-wait timing utilities
├── docs/                     # This documentation
├── Cargo.toml
├── memory.x                  # Linker script (16 KB Flash / 2 KB RAM)
├── rust-toolchain.toml       # Pinned nightly toolchain
└── riscv32ec-unknown-none-elf.json  # Custom RISC-V target spec
```
