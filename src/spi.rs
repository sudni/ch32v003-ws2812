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
    // Wait for SPI to finish shifting out the last byte
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
