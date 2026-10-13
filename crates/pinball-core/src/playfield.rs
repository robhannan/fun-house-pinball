//! How the 1956 Williams *Fun House* (IPDB #967) is wired to *our* controller.
//!
//! The original is an electromechanical game with no switch matrix, so these
//! assignments describe the new harness and are free to change as parts go in.
//! Keep this file the single source of truth and update `docs/hardware.md`
//! alongside it.
//!
//! The feature list comes from a photo of a restored table. Counts marked
//! *verify* are best guesses from that photo; check them against the real
//! playfield and schematic.
//!
//! Playfield, top to bottom:
//!
//! * top lanes either side of a red centre bumper
//! * three pop bumpers across the top
//! * "10 points" side lanes and the centre "Scores Bonus" gate
//! * "30 points when lit" rollovers, left and right
//! * two lower pop bumpers, one each side
//! * the 10–100 bonus wheel (lamps only), with "Special when lit" above it
//! * two kicking rubbers above two inward-facing flippers, no outlanes, and a
//!   centre post above the outhole

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
    /// Left kicking rubber, above the left flipper.
    pub const LEFT_KICKER: SwitchId = SwitchId::matrix(1, 0);
    /// Right kicking rubber, above the right flipper.
    pub const RIGHT_KICKER: SwitchId = SwitchId::matrix(1, 1);

    /// Pop bumper skirt switches: three across the top (left, centre, right),
    /// then the lower left and lower right.
    pub const POP_BUMPERS: [SwitchId; 5] = [
        SwitchId::matrix(2, 0),
        SwitchId::matrix(2, 1),
        SwitchId::matrix(2, 2),
        SwitchId::matrix(2, 3),
        SwitchId::matrix(2, 4),
    ];

    /// Top lanes, left to right (*verify*: count and whether the outer corner
    /// units are lanes).
    pub const TOP_LANES: [SwitchId; 4] = [
        SwitchId::matrix(3, 0),
        SwitchId::matrix(3, 1),
        SwitchId::matrix(3, 2),
        SwitchId::matrix(3, 3),
    ];
    /// Red bumper at the top centre (*verify*: passive bumper or target).
    pub const TOP_CENTRE_BUMPER: SwitchId = SwitchId::matrix(3, 4);

    /// "10 points" side lanes, left to right (*verify* count).
    pub const SIDE_LANES: [SwitchId; 4] = [
        SwitchId::matrix(4, 0),
        SwitchId::matrix(4, 1),
        SwitchId::matrix(4, 2),
        SwitchId::matrix(4, 3),
    ];
    /// "Scores Bonus" gate in the centre.
    pub const BONUS_GATE: SwitchId = SwitchId::matrix(4, 4);
    /// "30 points when lit" rollovers, left then right.
    pub const THIRTY_POINT_ROLLOVERS: [SwitchId; 2] =
        [SwitchId::matrix(5, 0), SwitchId::matrix(5, 1)];
}

/// Coil driver assignments.
pub mod coil {
    use super::CoilId;

    /// Kicks a drained ball from the outhole back to the shooter lane.
    /// Leave unused if keeping the original manual ball lift.
    pub const BALL_SERVE: CoilId = CoilId::new(0);
    /// Left kicking rubber.
    pub const LEFT_KICKER: CoilId = CoilId::new(1);
    /// Right kicking rubber.
    pub const RIGHT_KICKER: CoilId = CoilId::new(2);
    /// Pop bumpers, matching [`super::sw::POP_BUMPERS`].
    pub const POP_BUMPERS: [CoilId; 5] = [
        CoilId::new(3),
        CoilId::new(4),
        CoilId::new(5),
        CoilId::new(6),
        CoilId::new(7),
    ];
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
    /// "Special when lit", above the bonus wheel.
    pub const SPECIAL: LampId = LampId::new(2);
    /// The bonus wheel: 10, 20, ... 100 points.
    pub const BONUS_WHEEL: [LampId; 10] = [
        LampId::new(8),
        LampId::new(9),
        LampId::new(10),
        LampId::new(11),
        LampId::new(12),
        LampId::new(13),
        LampId::new(14),
        LampId::new(15),
        LampId::new(16),
        LampId::new(17),
    ];
    /// "10 points when lit" in the centre of the bonus wheel.
    pub const BONUS_WHEEL_CENTRE: LampId = LampId::new(18);
    /// "30 points when lit", matching [`super::sw::THIRTY_POINT_ROLLOVERS`].
    pub const THIRTY_POINTS: [LampId; 2] = [LampId::new(20), LampId::new(21)];
    /// Pop bumper caps, matching [`super::sw::POP_BUMPERS`].
    pub const POP_BUMPERS: [LampId; 5] = [
        LampId::new(24),
        LampId::new(25),
        LampId::new(26),
        LampId::new(27),
        LampId::new(28),
    ];
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
    let mut i = 0;
    while i < coil::POP_BUMPERS.len() {
        k[coil::POP_BUMPERS[i].index()] = pulse(20, 60);
        i += 1;
    }
    k[coil::LEFT_FLIPPER_POWER.index()] = pulse(40, 0);
    k[coil::LEFT_FLIPPER_HOLD.index()] = CoilKind::Hold;
    k[coil::RIGHT_FLIPPER_POWER.index()] = pulse(40, 0);
    k[coil::RIGHT_FLIPPER_HOLD.index()] = CoilKind::Hold;
    k
};
