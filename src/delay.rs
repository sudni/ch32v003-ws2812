// ── Tiny busy-wait delay ──────────────────────────────────────────────────────
#[inline(always)]
pub fn delay_us(us: u32) {
    // CH32V003 runs at 48 MHz after HSI trim; ~48 cycles/µs.
    // Each iteration is roughly 4 cycles → 12 iterations per µs.
    let loops = us * 12;
    for _ in 0..loops {
        unsafe { core::arch::asm!("nop") };
    }
}

#[inline(always)]
pub fn delay_ms(ms: u32) {
    delay_us(ms * 1000);
}
