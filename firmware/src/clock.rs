//! A 1 ms system tick from timer TC0, in the style of Arduino's `millis()`.
//! See <https://blog.rahix.de/005-avr-hal-millis/> for a walkthrough.

use core::cell::Cell;

use avr_device::interrupt::Mutex;
use pinball_core::Instant;

// 16 MHz / 64 prescaler / 250 counts = 1 kHz.
const TIMER_COUNTS: u8 = 250;

static MILLIS: Mutex<Cell<u32>> = Mutex::new(Cell::new(0));

/// Starts the tick. Interrupts must be enabled afterwards.
pub fn init(tc0: arduino_hal::pac::TC0) {
    tc0.tccr0a().write(|w| w.wgm0().ctc());
    tc0.ocr0a().write(|w| w.set(TIMER_COUNTS - 1));
    tc0.tccr0b().write(|w| w.cs0().prescale_64());
    tc0.timsk0().write(|w| w.ocie0a().set_bit());
}

#[avr_device::interrupt(atmega2560)]
fn TIMER0_COMPA() {
    avr_device::interrupt::free(|cs| {
        let millis = MILLIS.borrow(cs);
        millis.set(millis.get().wrapping_add(1));
    });
}

/// Milliseconds since [`init`].
pub fn now() -> Instant {
    Instant::from_millis(avr_device::interrupt::free(|cs| MILLIS.borrow(cs).get()))
}
