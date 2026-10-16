//! Optional real bells, rung by coils, as on 1950s machines.
//!
//! None are fitted yet, so this is off by default ([`crate::Settings::bells`]).
//! When it's on, scoring rings real bells the way an electromechanical score
//! motor did: one stroke per unit, so 30 points is three strokes on the "10s"
//! bell and a 100-point bonus is a run of ten. The strokes come at a steady
//! [`RING_INTERVAL_MS`] rhythm. The WAV Trigger then skips its recorded chimes,
//! because the real bells replace them.

use crate::coils::Coils;
use crate::playfield::coil;
use crate::time::Instant;

/// Time between bell strokes, like a score motor's steps.
pub const RING_INTERVAL_MS: u32 = 120;
/// How long each bell coil is pulsed. Its limit is also in `COIL_KINDS`.
pub const BELL_PULSE_MS: u16 = 15;
/// Most strokes queued per bell, so a burst of scoring can't ring on for ages.
pub const MAX_QUEUED: u8 = 20;

/// Queues bell strokes and rings them in time.
#[derive(Clone, Copy, Debug, Default)]
pub struct BellRinger {
    /// Strokes still to ring on the 1s, 10s and 100s bells.
    queued: [u8; 3],
    next_at: Option<Instant>,
}

impl BellRinger {
    /// Nothing queued.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            queued: [0; 3],
            next_at: None,
        }
    }

    /// Queues the strokes for `points`: one per unit on each bell.
    pub fn score(&mut self, points: u32) {
        let digits = [
            points % 10,
            (points / 10) % 10,
            (points / 100) % 10 + points / 1_000 * 10,
        ];
        for (queued, digit) in self.queued.iter_mut().zip(digits) {
            *queued = queued
                .saturating_add(digit.min(u32::from(MAX_QUEUED)) as u8)
                .min(MAX_QUEUED);
        }
    }

    /// Drops any strokes still queued.
    pub fn clear(&mut self) {
        self.queued = [0; 3];
    }

    /// Strokes still to ring, for the 1s, 10s and 100s bells.
    #[must_use]
    pub const fn queued(&self) -> [u8; 3] {
        self.queued
    }

    /// Rings the next stroke on each bell that has one queued, if it's time.
    pub fn tick(&mut self, now: Instant, coils: &mut Coils) {
        if self.queued == [0; 3] {
            self.next_at = None;
            return;
        }
        if self.next_at.is_some_and(|at| !now.reached(at)) {
            return;
        }
        for (queued, bell) in self.queued.iter_mut().zip(coil::BELLS) {
            if *queued > 0 {
                *queued -= 1;
                coils.pulse(bell, BELL_PULSE_MS, now);
            }
        }
        self.next_at = Some(now.plus(RING_INTERVAL_MS));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::playfield::COIL_KINDS;

    #[test]
    fn points_ring_one_stroke_per_unit() {
        let mut bells = BellRinger::new();
        bells.score(30);
        bells.score(1);
        assert_eq!(bells.queued(), [1, 3, 0]);
        bells.score(100);
        assert_eq!(bells.queued(), [1, 3, 1]);
    }

    #[test]
    fn strokes_come_at_the_score_motor_rhythm() {
        let mut coils = Coils::new(COIL_KINDS);
        coils.set_enabled(true);
        let mut bells = BellRinger::new();
        bells.score(20);
        let tens = 1 << coil::BELLS[1].index();

        bells.tick(Instant::from_millis(0), &mut coils);
        assert_ne!(coils.outputs(Instant::from_millis(1)) & tens, 0);
        assert_eq!(bells.queued(), [0, 1, 0]);

        // Not again until the interval has passed.
        bells.tick(Instant::from_millis(60), &mut coils);
        assert_eq!(bells.queued(), [0, 1, 0]);
        bells.tick(Instant::from_millis(RING_INTERVAL_MS), &mut coils);
        assert_eq!(bells.queued(), [0, 0, 0]);
    }

    #[test]
    fn a_burst_of_scoring_is_capped() {
        let mut bells = BellRinger::new();
        for _ in 0..10 {
            bells.score(90);
        }
        assert_eq!(bells.queued()[1], MAX_QUEUED);
    }
}
