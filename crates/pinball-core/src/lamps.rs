//! Playfield and cabinet lamps (LED replacements driven via shift registers).

use crate::time::Instant;

/// Number of lamp outputs (8 × 8-bit shift registers).
pub const LAMP_COUNT: usize = 64;

/// Identifies one lamp output.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LampId(u8);

impl LampId {
    /// Lamp output `n`.
    ///
    /// # Panics
    ///
    /// If out of range. Used in a `const`, this is a compile-time error.
    #[must_use]
    pub const fn new(n: u8) -> Self {
        assert!((n as usize) < LAMP_COUNT, "lamp number out of range");
        Self(n)
    }

    /// Dense index in `0..LAMP_COUNT`.
    #[must_use]
    pub const fn index(self) -> usize {
        self.0 as usize
    }
}

/// What a lamp is doing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LampMode {
    /// Off.
    #[default]
    Off,
    /// Steady on.
    On,
    /// Blinking with a 50% duty cycle.
    Blink {
        /// Full on+off period.
        period_ms: u16,
    },
}

/// State of every lamp.
#[derive(Clone, Debug)]
pub struct Lamps {
    modes: [LampMode; LAMP_COUNT],
}

impl Default for Lamps {
    fn default() -> Self {
        Self::new()
    }
}

impl Lamps {
    /// All lamps off.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            modes: [LampMode::Off; LAMP_COUNT],
        }
    }

    /// Sets one lamp's mode.
    pub fn set(&mut self, id: LampId, mode: LampMode) {
        self.modes[id.index()] = mode;
    }

    /// One lamp's current mode.
    #[must_use]
    pub const fn mode(&self, id: LampId) -> LampMode {
        self.modes[id.index()]
    }

    /// Turns every lamp off.
    pub fn all_off(&mut self) {
        self.modes = [LampMode::Off; LAMP_COUNT];
    }

    /// The lamp output bitmap at `now` (bit `n` = lamp `n`).
    #[must_use]
    pub fn frame(&self, now: Instant) -> u64 {
        self.modes.iter().enumerate().fold(0, |bits, (i, mode)| {
            let lit = match *mode {
                LampMode::Off => false,
                LampMode::On => true,
                LampMode::Blink { period_ms } => {
                    let half = u32::from(period_ms / 2).max(1);
                    (now.as_millis() / half) % 2 == 0
                }
            };
            bits | (u64::from(lit) << i)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blink_alternates_each_half_period() {
        let mut lamps = Lamps::new();
        lamps.set(LampId::new(3), LampMode::Blink { period_ms: 500 });
        lamps.set(LampId::new(4), LampMode::On);
        assert_eq!(lamps.frame(Instant::from_millis(0)), 0b11000);
        assert_eq!(lamps.frame(Instant::from_millis(250)), 0b10000);
        assert_eq!(lamps.frame(Instant::from_millis(500)), 0b11000);
    }
}
