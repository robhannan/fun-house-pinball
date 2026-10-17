//! The trapdoor flaps' hobby servos, driven by Timer/Counter 3 in hardware.
//!
//! The timer runs fast PWM with ICR3 as the top (mode 14), from the 16 MHz
//! clock divided by 8: 2 ticks per microsecond and a 20 ms (50 Hz) frame. Its
//! three compare outputs give each servo its pulse:
//!
//! | Flap | Pin | Output |
//! |---|---|---|
//! | 1 (left) | D5 | OC3A |
//! | 2 (centre) | D2 | OC3B |
//! | 3 (right) | D3 | OC3C |
//!
//! The hardware repeats the pulses by itself, so the main loop only writes a
//! pulse width and never waits. Pin assignments must match `docs/hardware.md`.

use arduino_hal::pac::TC3;
use arduino_hal::port::Pin;
use arduino_hal::port::mode::Output;
use pinball_core::flaps::{FLAP_COUNT, pulse_us};

/// Timer ticks per microsecond: 16 MHz divided by 8.
const TICKS_PER_US: u16 = 2;
/// One 20 ms servo frame, in timer ticks.
const FRAME_TICKS: u16 = 20_000 * TICKS_PER_US;

/// The three flap servos.
pub struct FlapServos {
    timer: TC3,
    /// The signal pins, kept as outputs so the timer can drive them.
    _signals: [Pin<Output>; FLAP_COUNT],
}

impl FlapServos {
    /// Takes the timer and the signal pins for flaps 1–3 (D5, D2, D3, as
    /// outputs), and starts the pulses with each flap at `angles`.
    pub fn new(timer: TC3, signals: [Pin<Output>; FLAP_COUNT], angles: [u8; FLAP_COUNT]) -> Self {
        let mut servos = Self {
            timer,
            _signals: signals,
        };
        // Set the pulse widths before the timer starts, so the first pulse is
        // already right.
        servos.set(angles);
        servos.timer.icr3().write(|w| w.set(FRAME_TICKS));
        servos.timer.tccr3a().write(|w| {
            w.wgm3().set(0b10);
            w.com3a().match_clear();
            w.com3b().match_clear();
            w.com3c().match_clear()
        });
        servos.timer.tccr3b().write(|w| {
            w.wgm3().set(0b11);
            w.cs3().prescale_8()
        });
        servos
    }

    /// Points each flap's servo at its angle, in degrees. The timer picks up
    /// a new width at the start of the next frame.
    pub fn set(&mut self, angles: [u8; FLAP_COUNT]) {
        let [a, b, c] = angles.map(|angle| pulse_us(angle) * TICKS_PER_US);
        self.timer.ocr3a().write(|w| w.set(a));
        self.timer.ocr3b().write(|w| w.set(b));
        self.timer.ocr3c().write(|w| w.set(c));
    }
}
