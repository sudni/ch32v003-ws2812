# TFT Display Protocol — ILI9341 / ST7789

The QVGA 2.2″ module uses an **ILI9341** (or compatible ST7789) controller.  
Protocol: 4-wire SPI — **SCK, MOSI, CS, DC**.

## DC pin logic

| DC state | Meaning |
|----------|---------|
| LOW | **Command** byte |
| HIGH | **Data** byte (parameter or pixel) |

---

## Initialisation sequence

| Step | Command | Data | Delay | Effect |
|------|---------|------|-------|--------|
| HW Reset | RST pulse | — | 150 ms | Full hardware reset |
| SW Reset | `0x01` | — | 150 ms | Register defaults |
| Sleep Out | `0x11` | — | 255 ms | Oscillator startup |
| Pixel Format | `0x3A` | `0x55` | — | RGB565 (16 bpp) |
| Mem Access Ctrl | `0x36` | `0x48` | — | BGR, col order |
| Column Addr | `0x2A` | `00 00 00 EF` | — | X: 0→239 |
| Row Addr | `0x2B` | `00 00 01 3F` | — | Y: 0→319 |
| Display ON | `0x29` | — | 10 ms | Enable display |

### Memory access control byte `0x48`

| Bit | Name | Value | Effect |
|-----|------|-------|--------|
| 7 | MY | 0 | Top→bottom row order |
| 6 | MX | 1 | Right→left column order |
| 5 | MV | 0 | No row/column exchange |
| 3 | BGR | 1 | **BGR** colour order (required for this module) |

> If colours appear swapped, change `0x48` → `0x40` to disable BGR.

---

## Pixel write sequence

```
CMD 0x2A → column window (4 bytes)
CMD 0x2B → row window    (4 bytes)
CMD 0x2C → begin pixel stream
DC HIGH, CS LOW
  [send N × 2 bytes RGB565 MSB-first via SPI/DMA]
CS HIGH
```

## RGB565 colour table

| Colour | Hex | Bytes |
|--------|-----|-------|
| Red | `0xF800` | `F8 00` |
| Green | `0x07E0` | `07 E0` |
| Blue | `0x001F` | `00 1F` |
| White | `0xFFFF` | `FF FF` |
| Black | `0x0000` | `00 00` |
| Yellow | `0xFFE0` | `FF E0` |
| Cyan | `0x07FF` | `07 FF` |
| Magenta | `0xF81F` | `F8 1F` |

## Useful commands

| Hex | Command | Notes |
|-----|---------|-------|
| `0x01` | Software Reset | Wait 150 ms |
| `0x10` | Sleep In | |
| `0x11` | Sleep Out | Wait 255 ms |
| `0x20` | Display Inversion Off | |
| `0x21` | Display Inversion On | |
| `0x28` | Display Off | |
| `0x29` | Display On | |
| `0x2A` | Column Address Set | 4 data bytes |
| `0x2B` | Row Address Set | 4 data bytes |
| `0x2C` | Memory Write | Pixel stream follows |
| `0x2E` | Memory Read | Read pixels back |
| `0x36` | Memory Access Control | Orientation |
| `0x3A` | Pixel Format | `0x55`=16bpp |
