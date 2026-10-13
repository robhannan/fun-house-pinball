//! Solenoid, flasher and motor drive with built-in safety limits.
//!
//! A coil left energised will cook itself (and possibly its driver board) in
//! seconds, so every [`CoilKind::Pulse`] coil has a hard maximum on-time and a
//! minimum recycle time enforced here, regardless of what game code asks for.
//! The firmware adds a second line of defence with the hardware watchdog.

use crate::time::Instant;

/// Number of coil driver outputs (4 × 8-bit shift registers).
pub const COIL_COUNT: usize = 32;

/// Identifies one coil driver output.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CoilId(u8);

impl CoilId {
    /// Coil driver output `n`.
    ///
    /// # Panics
    ///
    /// If out of range. Used in a `const`, this is a compile-time error.
    #[must_use]
    pub const fn new(n: u8) -> Self {
        assert!((n as usize) < COIL_COUNT, "coil number out of range");
        Self(n)
    }

    /// Dense index in `0..COIL_COUNT`.
    #[must_use]
    pub const fn index(self) -> usize {
        self.0 as usize
    }
}

/// How a driver output may be used.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CoilKind {
    /// Output not connected; requests are ignored.
    Unused,
    /// May only be pulsed. On-time is clamped to `max_ms`, and the coil will not
    /// re-fire until `recycle_ms` after its last pulse ended (stops a stuck
    /// slingshot switch machine-gunning the coil).
    Pulse {
        /// Longest permitted pulse.
        max_ms: u16,
        /// Minimum off-time between pulses.
        recycle_ms: u16,
    },
    /// May be held on indefinitely: flipper hold windings, motors, relays.
    Hold,
}

/// State of every coil driver output.
#[derive(Clone, Debug)]
pub struct Coils {
    kinds: [CoilKind; COIL_COUNT],
    pulse_until: [Option<Instant>; COIL_COUNT],
    ready_at: [Instant; COIL_COUNT],
    held: u32,
    enabled: bool,
}

impl Coils {
    /// Creates the driver state with every output off and drive disabled.
    #[must_use]
    pub const fn new(kinds: [CoilKind; COIL_COUNT]) -> Self {
        Self {
            kinds,
            pulse_until: [None; COIL_COUNT],
            ready_at: [Instant::from_millis(0); COIL_COUNT],
            held: 0,
            enabled: false,
        }
    }

    /// Master enable. Disabling immediately drops every output and forgets any
    /// pending pulses or holds (the software equivalent of the coil power relay).
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if !enabled {
            self.pulse_until = [None; COIL_COUNT];
            self.held = 0;
        }
    }

    /// Fires `id` for `ms` milliseconds, clamped to the coil's limit. Returns
    /// whether the pulse actually started.
    pub fn pulse(&mut self, id: CoilId, ms: u16, now: Instant) -> bool {
        let i = id.index();
        let ms = match self.kinds[i] {
            CoilKind::Unused => return false,
            CoilKind::Pulse { max_ms, .. } => ms.min(max_ms),
            CoilKind::Hold => ms,
        };
        if !self.enabled || ms == 0 || !now.reached(self.ready_at[i]) {
            return false;
        }
        self.pulse_until[i] = Some(now.plus(u32::from(ms)));
        true
    }

    /// Turns a [`CoilKind::Hold`] output on or off. Ignored for other kinds.
    pub fn hold(&mut self, id: CoilId, on: bool) {
        if self.kinds[id.index()] != CoilKind::Hold || !self.enabled {
            return;
        }
        if on {
            self.held |= 1 << id.0;
        } else {
            self.held &= !(1 << id.0);
        }
    }

    /// Expires finished pulses and returns the output bitmap (bit `n` = coil `n`).
    pub fn outputs(&mut self, now: Instant) -> u32 {
        if !self.enabled {
            return 0;
        }
        let mut bits = self.held;
        for (i, until) in self.pulse_until.iter_mut().enumerate() {
            let Some(end) = *until else { continue };
            if now.reached(end) {
                *until = None;
                if let CoilKind::Pulse { recycle_ms, .. } = self.kinds[i] {
                    self.ready_at[i] = end.plus(u32::from(recycle_ms));
                }
            } else {
                bits |= 1 << i;
            }
        }
        bits
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SLING: CoilId = CoilId::new(0);
    const HOLD: CoilId = CoilId::new(1);

    fn coils() -> Coils {
        let mut kinds = [CoilKind::Unused; COIL_COUNT];
        kinds[SLING.index()] = CoilKind::Pulse {
            max_ms: 20,
            recycle_ms: 50,
        };
        kinds[HOLD.index()] = CoilKind::Hold;
        let mut coils = Coils::new(kinds);
        coils.set_enabled(true);
        coils
    }

    const fn at(ms: u32) -> Instant {
        Instant::from_millis(ms)
    }

    #[test]
    fn pulse_is_clamped_to_max_on_time() {
        let mut c = coils();
        assert!(c.pulse(SLING, 10_000, at(100)));
        assert_eq!(c.outputs(at(119)), 1);
        assert_eq!(c.outputs(at(120)), 0);
    }

    #[test]
    fn coil_cannot_refire_until_recycled() {
        let mut c = coils();
        c.pulse(SLING, 20, at(0));
        let _ = c.outputs(at(20));
        assert!(!c.pulse(SLING, 20, at(30)));
        assert!(c.pulse(SLING, 20, at(70)));
    }

    #[test]
    fn disabling_drops_everything() {
        let mut c = coils();
        c.pulse(SLING, 20, at(0));
        c.hold(HOLD, true);
        assert_eq!(c.outputs(at(1)), 0b11);
        c.set_enabled(false);
        assert_eq!(c.outputs(at(2)), 0);
        assert!(!c.pulse(SLING, 20, at(1_000)));
    }

    #[test]
    fn pulse_only_coils_cannot_be_held() {
        let mut c = coils();
        c.hold(SLING, true);
        assert_eq!(c.outputs(at(0)), 0);
    }
}
