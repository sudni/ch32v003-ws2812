# Peripheral Configuration — SPI1 & DMA1

## Clock enable (RCC)

| Bus | Register | Bit | Peripheral |
|-----|----------|-----|------------|
| AHB | `RCC.AHBPCENR` | `DMA1EN` | DMA1 controller |
| APB2 | `RCC.APB2PCENR` | `SPI1EN` | SPI1 |
| APB2 | `RCC.APB2PCENR` | `IOPCEN` | GPIOC |
| APB2 | `RCC.APB2PCENR` | `IOPDEN` | GPIOD |
| APB2 | `RCC.APB2PCENR` | `AFIOEN` | Alternate Function I/O |

---

## GPIO configuration

### GPIOC CFGLR (address `0x4001_1000`)

| Pin | Bit field | MODE | CNF | Mode description |
|-----|-----------|------|-----|-----------------|
| PC1 (CS) | `cnf1`, `mode1` | `01` | `00` | GP Output push-pull, 10 MHz |
| PC5 (SCK) | `cnf5`, `mode5` | `11` | `10` | AF push-pull, 50 MHz |
| PC6 (MOSI) | `cnf6`, `mode6` | `11` | `10` | AF push-pull, 50 MHz |

### GPIOD CFGLR (address `0x4001_1400`)

| Pin | Bit field | MODE | CNF | Mode description |
|-----|-----------|------|-----|-----------------|
| PD3 (DC) | `cnf3`, `mode3` | `01` | `00` | GP Output push-pull, 10 MHz |
| PD4 (RST) | `cnf4`, `mode4` | `01` | `00` | GP Output push-pull, 10 MHz |

**GPIO MODE encoding (CH32V003):**

| MODE[1:0] | Speed |
|-----------|-------|
| `00` | Input |
| `01` | Output 10 MHz |
| `10` | Output 2 MHz |
| `11` | Output 50 MHz |

**GPIO CNF encoding (output mode):**

| CNF[1:0] | Type |
|----------|------|
| `00` | General-purpose push-pull |
| `01` | General-purpose open-drain |
| `10` | Alternate function push-pull |
| `11` | Alternate function open-drain |

---

## SPI1 configuration

**Base address:** `0x4001_3000`

### SPI1_CTLR1 — Control Register 1

| Bit | Name | Value | Description |
|-----|------|-------|-------------|
| 11 | `DFF` | `0` | 8-bit data frame |
| 10 | `RXONLY` | `0` | Full duplex |
| 9 | `SSM` | `1` | Software NSS management |
| 8 | `SSI` | `1` | Internal NSS = HIGH (CS driven manually) |
| 7 | `LSBFIRST` | `0` | MSB first |
| 6 | `SPE` | `1` | SPI enable |
| 5:3 | `BR[2:0]` | `000` | fPCLK / 2 = 24 MHz |
| 2 | `MSTR` | `1` | Master mode |
| 1 | `CPOL` | `0` | Clock idle LOW (SPI Mode 0) |
| 0 | `CPHA` | `0` | Data captured on 1st edge (SPI Mode 0) |

### SPI1_CTLR2 — Control Register 2

| Bit | Name | Value | Description |
|-----|------|-------|-------------|
| 1 | `TXDMAEN` | `1` | Enable TX DMA request → links SPI to DMA1 Ch3 |
| 0 | `RXDMAEN` | `0` | RX DMA disabled (write-only) |

### SPI1_STATR — Status Register (read-only flags)

| Bit | Name | Meaning |
|-----|------|---------|
| 7 | `BSY` | SPI is busy (transfer in progress) |
| 1 | `TXE` | TX buffer empty (ready to write) |
| 0 | `RXNE` | RX buffer not empty |

### SPI1_DATAR — Data Register

- **Address:** `0x4001_300C`
- Write 8-bit data here to transmit; also the DMA destination address for TX.

---

## DMA1 — Channel 3 (SPI1 TX)

**Base address:** `0x4002_0000`

> DMA channel mapping is fixed in hardware on CH32V003:
> - Channel 3 → SPI1 TX
> - Channel 2 → SPI1 RX

### DMA1_CFGR3 — Channel 3 Configuration

| Bit(s) | Name | Value | Description |
|--------|------|-------|-------------|
| 14 | `MEM2MEM` | `0` | Peripheral ↔ memory mode |
| 13:12 | `PL[1:0]` | `10` | High priority |
| 11:10 | `MSIZE[1:0]` | `00` | Memory data width = 8-bit (`01` for 16-bit fills) |
| 9:8 | `PSIZE[1:0]` | `00` | Peripheral data width = 8-bit (`01` for 16-bit fills) |
| 7 | `MINC` | `1` | Memory address auto-increment (`0` to repeat single value) |
| 6 | `PINC` | `0` | Peripheral address fixed |
| 5 | `CIRC` | `0` | One-shot (no circular mode) |
| 4 | `DIR` | `1` | Direction: memory → peripheral |
| 3 | `TEIE` | `0` | Transfer error interrupt disabled |
| 2 | `HTIE` | `0` | Half-transfer interrupt disabled |
| 1 | `TCIE` | `0` | Transfer complete interrupt disabled (polling) |
| 0 | `EN` | `1` | Channel enable |

### DMA1_CNTR3 — Channel 3 Transfer Count

- Set to `buf.len()` (or element count for 16-bit mode) before enabling channel.
- Counts down to 0; channel auto-disables when done.
- **Important:** This is a 16-bit register! Maximum value is `65,535`. A full screen transfer (`240×320 = 76,800`) will overflow this counter. Send data in chunks (e.g., row-by-row) to avoid truncation.

### DMA1_PADDR3 — Channel 3 Peripheral Address

- Set to `0x4001_300C` (`SPI1_DATAR`).

### DMA1_MADDR3 — Channel 3 Memory Address

- Set to `buf.as_ptr()`.

### DMA1_INTFR — Interrupt Flag Register (read)

| Bit | Flag | Meaning |
|-----|------|---------|
| 9 | `TC3` | Channel 3 transfer complete ← polled to detect end |
| 8 | `HT3` | Channel 3 half-transfer |
| 10 | `TE3` | Channel 3 transfer error |

### DMA1_INTFCR — Interrupt Flag Clear Register (write)

- Write `0x0F00` to clear all flags for Channel 3.

---

## Memory map summary

| Peripheral | Base Address |
|-----------|-------------|
| DMA1 | `0x4002_0000` |
| RCC | `0x4002_1000` |
| GPIOC | `0x4001_1000` |
| GPIOD | `0x4001_1400` |
| AFIO | `0x4001_0000` |
| SPI1 | `0x4001_3000` |
