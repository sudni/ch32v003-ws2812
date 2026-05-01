use ch32v0::ch32v003::Peripherals;

// DMA1 Channel 3 is hardwired to SPI1_TX on CH32V003.
// SPI1_DATAR address (from RM, offset 0x0C from SPI1 base 0x4001_3000)
pub const SPI1_DATAR: u32 = 0x4001_300C;

/// Send `len` **bytes** from `buf` over SPI1 via DMA1 Ch3 (8-bit, MINC=1).
/// Used for command argument payloads where every byte differs.
/// Caller must assert CS and set DC beforehand.
#[allow(dead_code)]
pub fn spi_dma_tx(p: &Peripherals, buf: &[u8]) {
    if buf.is_empty() {
        return;
    }
    let dma = &p.DMA1;

    // Disable channel before reconfiguring
    dma.cfgr3().modify(|_, w| w.en().clear_bit());

    // Clear all interrupt flags for ch3 (bits 8..11 in INTFCR)
    dma.intfcr().write(|w| unsafe { w.bits(0x0F00) });

    // Peripheral address → SPI1 data register
    dma.paddr3().write(|w| unsafe { w.bits(SPI1_DATAR) });

    // Memory address → start of buffer
    dma.maddr3()
        .write(|w| unsafe { w.bits(buf.as_ptr() as u32) });

    // Transfer count (bytes)
    dma.cntr3().write(|w| unsafe { w.bits(buf.len() as u32) });

    // Configure channel:
    //   DIR=1   memory→peripheral
    //   MINC=1  auto-increment memory address
    //   PINC=0  peripheral address fixed
    //   PSIZE=00 (8-bit)  MSIZE=00 (8-bit)
    //   PL=10   high priority
    //   CIRC=0  one-shot
    dma.cfgr3().write(|w| unsafe {
        w.dir()
            .set_bit() // mem→periph
            .minc()
            .set_bit() // increment memory ptr
            .pinc()
            .clear_bit()
            .psize()
            .bits(0b00) // 8-bit peripheral
            .msize()
            .bits(0b00) // 8-bit memory
            .pl()
            .bits(0b10) // high priority
            .circ()
            .clear_bit()
            .mem2mem()
            .clear_bit()
            .en()
            .set_bit()
    });

    // Wait for transfer complete (TC flag = bit 9 of INTFR)
    while p.DMA1.intfr().read().bits() & (1 << 9) == 0 {}

    // Wait until SPI is not busy before deasserting CS
    while p.SPI1.statr().read().bsy().bit_is_set() {}
}

/// Repeat a single **16-bit** pixel value `count` times over SPI1 via DMA1 Ch3.
///
/// Key differences from `spi_dma_tx`:
///   - PSIZE = MSIZE = 01  → 16-bit transfers (SPI DFF must be 1)
///   - MINC  = 0           → DMA reads the *same* memory address every time
///
/// This lets us fill the whole screen from a single `u16` without any buffer.
/// The SPI peripheral must already be configured for 16-bit frames (DFF=1)
/// before calling this, and restored to 8-bit afterwards.
/// Caller must assert CS and set DC=data beforehand.
pub fn spi_dma_fill16(p: &Peripherals, pixel: *const u16, count: u32) {
    if count == 0 {
        return;
    }
    let dma = &p.DMA1;

    // Disable channel before reconfiguring
    dma.cfgr3().modify(|_, w| w.en().clear_bit());

    // Clear all interrupt flags for ch3
    dma.intfcr().write(|w| unsafe { w.bits(0x0F00) });

    // Peripheral address → SPI1 data register
    dma.paddr3().write(|w| unsafe { w.bits(SPI1_DATAR) });

    // Memory address → the single pixel variable
    dma.maddr3().write(|w| unsafe { w.bits(pixel as u32) });

    // Transfer count (number of 16-bit words)
    dma.cntr3().write(|w| unsafe { w.bits(count) });

    // Configure channel:
    //   DIR=1   memory→peripheral
    //   MINC=0  fixed source address (repeat same pixel)
    //   PINC=0  peripheral address fixed
    //   PSIZE=01 (16-bit)  MSIZE=01 (16-bit)
    //   PL=10   high priority
    //   CIRC=0  one-shot
    dma.cfgr3().write(|w| unsafe {
        w.dir()
            .set_bit() // mem→periph
            .minc()
            .clear_bit() // ← fixed: no memory increment
            .pinc()
            .clear_bit()
            .psize()
            .bits(0b01) // 16-bit peripheral
            .msize()
            .bits(0b01) // 16-bit memory
            .pl()
            .bits(0b10) // high priority
            .circ()
            .clear_bit()
            .mem2mem()
            .clear_bit()
            .en()
            .set_bit()
    });

    // Wait for transfer complete (TC flag = bit 9 of INTFR)
    while p.DMA1.intfr().read().bits() & (1 << 9) == 0 {}

    // Wait until SPI is not busy before deasserting CS
    while p.SPI1.statr().read().bsy().bit_is_set() {}
}

// ── Low-level SPI (blocking, no DMA) for short commands ──────────────────────
pub fn spi_tx_byte(p: &Peripherals, byte: u8) {
    // Wait for TXE
    while p.SPI1.statr().read().txe().bit_is_clear() {}
    p.SPI1.datar().write(|w| unsafe { w.bits(byte as u32) });
    // Wait until not busy
    while p.SPI1.statr().read().bsy().bit_is_set() {}
}
