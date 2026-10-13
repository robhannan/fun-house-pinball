//! How the 1956 Williams *Fun House* (IPDB #967) is wired to *our* controller.
//!
//! The original is an electromechanical game with no switch matrix, so these
//! assignments describe the new harness and are free to change as parts go in.
//! Keep this file the single source of truth and update `docs/hardware.md`
//! alongside it.
//!
//! The feature list below is a generic 1950s EM starter set (flippers, pop
//! bumpers, kickers, rollovers, outlanes, outhole). Replace and extend it to
//! match what is actually on the playfield as each assembly is restored.

use crate::coils::{COIL_COUNT, CoilId, CoilKind};
use crate::lamps::LampId;
use crate::switches::SwitchId;

/// Switch assignments. Matrix positions are `(column, row)`.
pub mod sw {
    use super::SwitchId;

    // Cabinet: direct inputs, so flippers never wait on a matrix scan.
    /// Left flipper cabinet button.
    pub const LEFT_FLIPPER_BUTTON: SwitchId = SwitchId::direct(0);
    /// Right flipper cabinet button.
    pub const RIGHT_FLIPPER_BUTTON: SwitchId = SwitchId::direct(1);
    /// Start (replay) button.
    pub const START_BUTTON: SwitchId = SwitchId::direct(2);
    /// Plumb-bob / ball-roll tilt.
    pub const TILT: SwitchId = SwitchId::direct(3);
    /// Slam tilt (coin door / cabinet). Ends the game immediately.
    pub const SLAM_TILT: SwitchId = SwitchId::direct(4);

    // Ball handling.
    /// Outhole: a drained ball rests here.
    pub const OUTHOLE: SwitchId = SwitchId::matrix(0, 0);
    /// Shooter lane (ball ready to plunge).
    pub const SHOOTER_LANE: SwitchId = SwitchId::matrix(0, 1);

    // Lower playfield.
    /// Left outlane.
    pub const LEFT_OUTLANE: SwitchId = SwitchId::matrix(1, 0);
    /// Right outlane.
    pub const RIGHT_OUTLANE: SwitchId = SwitchId::matrix(1, 1);
    /// Left kicker (slingshot-style kicking rubber) switch.
    pub const LEFT_KICKER: SwitchId = SwitchId::matrix(1, 2);
    /// Right kicker switch.
    pub const RIGHT_KICKER: SwitchId = SwitchId::matrix(1, 3);

    // Upper playfield.
    /// Pop bumper skirt switches (adjust the count to match the playfield).
    pub const POP_BUMPERS: [SwitchId; 3] = [
        SwitchId::matrix(2, 0),
        SwitchId::matrix(2, 1),
        SwitchId::matrix(2, 2),
    ];
    /// Top rollover lanes (adjust the count to match the playfield).
    pub const TOP_ROLLOVERS: [SwitchId; 3] = [
        SwitchId::matrix(3, 0),
        SwitchId::matrix(3, 1),
        SwitchId::matrix(3, 2),
    ];
}

/// Coil driver assignments.
pub mod coil {
    use super::CoilId;

    /// Kicks a drained ball from the outhole back to the shooter lane.
    /// Leave unused if keeping the original manual ball lift.
    pub const BALL_SERVE: CoilId = CoilId::new(0);
    /// Left kicker.
    pub const LEFT_KICKER: CoilId = CoilId::new(1);
    /// Right kicker.
    pub const RIGHT_KICKER: CoilId = CoilId::new(2);
    /// Pop bumpers, matching [`super::sw::POP_BUMPERS`].
    pub const POP_BUMPERS: [CoilId; 3] = [CoilId::new(3), CoilId::new(4), CoilId::new(5)];
    /// Left flipper power (high-current) winding.
    pub const LEFT_FLIPPER_POWER: CoilId = CoilId::new(8);
    /// Left flipper hold (low-current) winding.
    pub const LEFT_FLIPPER_HOLD: CoilId = CoilId::new(9);
    /// Right flipper power winding.
    pub const RIGHT_FLIPPER_POWER: CoilId = CoilId::new(10);
    /// Right flipper hold winding.
    pub const RIGHT_FLIPPER_HOLD: CoilId = CoilId::new(11);
}

/// Lamp assignments.
pub mod lamp {
    use super::LampId;

    /// "Game over" backglass lamp.
    pub const GAME_OVER: LampId = LampId::new(0);
    /// "Tilt" backglass lamp.
    pub const TILT: LampId = LampId::new(1);
    /// Top rollover lane lamps, matching [`super::sw::TOP_ROLLOVERS`].
    pub const TOP_ROLLOVERS: [LampId; 3] = [LampId::new(8), LampId::new(9), LampId::new(10)];
}

const fn pulse(max_ms: u16, recycle_ms: u16) -> CoilKind {
    CoilKind::Pulse { max_ms, recycle_ms }
}

/// Electrical limits for every coil output. Start conservative and tune on the
/// real machine; never raise a `max_ms` without checking the coil's rating.
/// Original 1950s coils were wound for AC — use DC-rated replacements.
pub const COIL_KINDS: [CoilKind; COIL_COUNT] = {
    let mut k = [CoilKind::Unused; COIL_COUNT];
    k[coil::BALL_SERVE.index()] = pulse(30, 500);
    k[coil::LEFT_KICKER.index()] = pulse(20, 100);
    k[coil::RIGHT_KICKER.index()] = pulse(20, 100);
    k[coil::POP_BUMPERS[0].index()] = pulse(20, 60);
    k[coil::POP_BUMPERS[1].index()] = pulse(20, 60);
    k[coil::POP_BUMPERS[2].index()] = pulse(20, 60);
    k[coil::LEFT_FLIPPER_POWER.index()] = pulse(40, 0);
    k[coil::LEFT_FLIPPER_HOLD.index()] = CoilKind::Hold;
    k[coil::RIGHT_FLIPPER_POWER.index()] = pulse(40, 0);
    k[coil::RIGHT_FLIPPER_HOLD.index()] = CoilKind::Hold;
    k
};
