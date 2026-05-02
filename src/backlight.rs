use ch32v0::ch32v003::Peripherals;

// Initialize TIM1 CH1 on PD2 for PWM
pub fn init_backlight(p: &Peripherals) {
    // 1. Enable TIM1 clock (APB2)
    p.RCC.apb2pcenr().modify(|_, w| w.tim1en().set_bit());

    // 2. Configure PD2 as Alternate Function Push-Pull (AF PP 50MHz)
    // PD2 is MODE2=11, CNF2=10
    p.GPIOD.cfglr().modify(|_, w| unsafe {
        w.mode2().bits(0b11).cnf2().bits(0b10)
    });

    // 3. Configure TIM1
    // Target PWM freq: e.g., 10 kHz
    // System clock is 48 MHz.
    // PSC = 48-1  -> 1 MHz counter clock
    // ARR = 100-1 -> 10 kHz PWM freq. Max duty cycle = 100.
    p.TIM1.psc().write(|w| unsafe { w.bits(47) });
    p.TIM1.atrlr().write(|w| unsafe { w.bits(100) });

    // 4. Configure CCMR1 for PWM Mode 1 on CH1
    // OC1M = 110 (PWM Mode 1), OC1PE = 1 (Preload enable)
    p.TIM1.chctlr1_output().modify(|_, w| unsafe {
        w.oc1m().bits(0b110).oc1pe().set_bit()
    });

    // 5. Enable CH1 output (CC1E)
    p.TIM1.ccer().modify(|_, w| w.cc1e().set_bit());

    // 6. Main Output Enable (MOE) - required for TIM1
    p.TIM1.bdtr().modify(|_, w| w.moe().set_bit());

    // 7. Enable counter (CEN)
    p.TIM1.ctlr1().modify(|_, w| w.cen().set_bit());

    // Start with backlight OFF
    set_backlight(p, 0);
}

// Set backlight brightness: 0 (off) to 100 (full brightness)
pub fn set_backlight(p: &Peripherals, level: u16) {
    let mut safe_level = level;
    if safe_level > 100 {
        safe_level = 100;
    }
    p.TIM1.ch1cvr().write(|w| unsafe { w.bits(safe_level as u32) });
}
