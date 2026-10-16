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
//! * "10 points" side lanes and the centre "Scores Bonus" gate, with three
//!   coil-driven trapdoors beneath it
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
    /// The three trapdoors under the "Scores Bonus" gate, left to right. Each
    /// closes when a ball drops through its open flap.
    pub const TRAPDOORS: [SwitchId; 3] = [
        SwitchId::matrix(4, 5),
        SwitchId::matrix(4, 6),
        SwitchId::matrix(4, 7),
    ];
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
    /// Trapdoor flaps under the "Scores Bonus" gate, matching
    /// [`super::sw::TRAPDOORS`]. Assumed (*verify*): energised = flap open,
    /// so every flap closes whenever coil power is cut.
    pub const TRAPDOOR_FLAPS: [CoilId; 3] = [CoilId::new(12), CoilId::new(13), CoilId::new(14)];
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
    // A flap can be held open indefinitely, so these coils must be rated for
    // continuous duty at the coil supply voltage.
    let mut i = 0;
    while i < coil::TRAPDOOR_FLAPS.len() {
        k[coil::TRAPDOOR_FLAPS[i].index()] = CoilKind::Hold;
        i += 1;
    }
    k
};

/// One physical part and the I/O it uses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Part {
    /// Human-readable name.
    pub name: &'static str,
    /// The part's switch, if it has one.
    pub switch: Option<SwitchId>,
    /// The part's coil windings (a flipper has two).
    pub coils: &'static [CoilId],
    /// The part's lamp, if it has one.
    pub lamp: Option<LampId>,
}

const fn part(
    name: &'static str,
    switch: Option<SwitchId>,
    coils: &'static [CoilId],
    lamp: Option<LampId>,
) -> Part {
    Part {
        name,
        switch,
        coils,
        lamp,
    }
}

/// Every part on the machine. **Add new parts here** as well as to the modules
/// above: tests check this list against `COIL_KINDS`, and against the
/// simulator's page and the docs.
pub const PARTS: &[Part] = &[
    // Cabinet
    part(
        "Left flipper button",
        Some(sw::LEFT_FLIPPER_BUTTON),
        &[],
        None,
    ),
    part(
        "Right flipper button",
        Some(sw::RIGHT_FLIPPER_BUTTON),
        &[],
        None,
    ),
    part("Start button", Some(sw::START_BUTTON), &[], None),
    part("Tilt", Some(sw::TILT), &[], None),
    part("Slam tilt", Some(sw::SLAM_TILT), &[], None),
    part("Game over lamp", None, &[], Some(lamp::GAME_OVER)),
    part("Tilt lamp", None, &[], Some(lamp::TILT)),
    // Ball handling
    part("Outhole", Some(sw::OUTHOLE), &[coil::BALL_SERVE], None),
    part("Shooter lane", Some(sw::SHOOTER_LANE), &[], None),
    // Lower playfield
    part(
        "Left flipper",
        None,
        &[coil::LEFT_FLIPPER_POWER, coil::LEFT_FLIPPER_HOLD],
        None,
    ),
    part(
        "Right flipper",
        None,
        &[coil::RIGHT_FLIPPER_POWER, coil::RIGHT_FLIPPER_HOLD],
        None,
    ),
    part(
        "Left kicker",
        Some(sw::LEFT_KICKER),
        &[coil::LEFT_KICKER],
        None,
    ),
    part(
        "Right kicker",
        Some(sw::RIGHT_KICKER),
        &[coil::RIGHT_KICKER],
        None,
    ),
    // Pop bumpers
    part(
        "Pop bumper 1 (top left)",
        Some(sw::POP_BUMPERS[0]),
        &[coil::POP_BUMPERS[0]],
        Some(lamp::POP_BUMPERS[0]),
    ),
    part(
        "Pop bumper 2 (top centre)",
        Some(sw::POP_BUMPERS[1]),
        &[coil::POP_BUMPERS[1]],
        Some(lamp::POP_BUMPERS[1]),
    ),
    part(
        "Pop bumper 3 (top right)",
        Some(sw::POP_BUMPERS[2]),
        &[coil::POP_BUMPERS[2]],
        Some(lamp::POP_BUMPERS[2]),
    ),
    part(
        "Pop bumper 4 (lower left)",
        Some(sw::POP_BUMPERS[3]),
        &[coil::POP_BUMPERS[3]],
        Some(lamp::POP_BUMPERS[3]),
    ),
    part(
        "Pop bumper 5 (lower right)",
        Some(sw::POP_BUMPERS[4]),
        &[coil::POP_BUMPERS[4]],
        Some(lamp::POP_BUMPERS[4]),
    ),
    // Upper playfield
    part("Top lane 1", Some(sw::TOP_LANES[0]), &[], None),
    part("Top lane 2", Some(sw::TOP_LANES[1]), &[], None),
    part("Top lane 3", Some(sw::TOP_LANES[2]), &[], None),
    part("Top lane 4", Some(sw::TOP_LANES[3]), &[], None),
    part("Top centre bumper", Some(sw::TOP_CENTRE_BUMPER), &[], None),
    part("Side lane 1", Some(sw::SIDE_LANES[0]), &[], None),
    part("Side lane 2", Some(sw::SIDE_LANES[1]), &[], None),
    part("Side lane 3", Some(sw::SIDE_LANES[2]), &[], None),
    part("Side lane 4", Some(sw::SIDE_LANES[3]), &[], None),
    part("Scores Bonus gate", Some(sw::BONUS_GATE), &[], None),
    part(
        "Trapdoor 1",
        Some(sw::TRAPDOORS[0]),
        &[coil::TRAPDOOR_FLAPS[0]],
        None,
    ),
    part(
        "Trapdoor 2",
        Some(sw::TRAPDOORS[1]),
        &[coil::TRAPDOOR_FLAPS[1]],
        None,
    ),
    part(
        "Trapdoor 3",
        Some(sw::TRAPDOORS[2]),
        &[coil::TRAPDOOR_FLAPS[2]],
        None,
    ),
    part(
        "30 points rollover (left)",
        Some(sw::THIRTY_POINT_ROLLOVERS[0]),
        &[],
        Some(lamp::THIRTY_POINTS[0]),
    ),
    part(
        "30 points rollover (right)",
        Some(sw::THIRTY_POINT_ROLLOVERS[1]),
        &[],
        Some(lamp::THIRTY_POINTS[1]),
    ),
    part("Special when lit", None, &[], Some(lamp::SPECIAL)),
    // Bonus wheel
    part("Bonus wheel 10", None, &[], Some(lamp::BONUS_WHEEL[0])),
    part("Bonus wheel 20", None, &[], Some(lamp::BONUS_WHEEL[1])),
    part("Bonus wheel 30", None, &[], Some(lamp::BONUS_WHEEL[2])),
    part("Bonus wheel 40", None, &[], Some(lamp::BONUS_WHEEL[3])),
    part("Bonus wheel 50", None, &[], Some(lamp::BONUS_WHEEL[4])),
    part("Bonus wheel 60", None, &[], Some(lamp::BONUS_WHEEL[5])),
    part("Bonus wheel 70", None, &[], Some(lamp::BONUS_WHEEL[6])),
    part("Bonus wheel 80", None, &[], Some(lamp::BONUS_WHEEL[7])),
    part("Bonus wheel 90", None, &[], Some(lamp::BONUS_WHEEL[8])),
    part("Bonus wheel 100", None, &[], Some(lamp::BONUS_WHEEL[9])),
    part(
        "Bonus wheel centre",
        None,
        &[],
        Some(lamp::BONUS_WHEEL_CENTRE),
    ),
];

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn no_two_parts_share_a_switch_coil_or_lamp() {
        let mut switches = HashSet::new();
        let mut coils = HashSet::new();
        let mut lamps = HashSet::new();
        for part in PARTS {
            if let Some(s) = part.switch {
                assert!(switches.insert(s), "{}: switch {s:?} reused", part.name);
            }
            for &c in part.coils {
                assert!(coils.insert(c), "{}: coil {c:?} reused", part.name);
            }
            if let Some(l) = part.lamp {
                assert!(lamps.insert(l), "{}: lamp {l:?} reused", part.name);
            }
        }
    }

    #[test]
    fn part_names_are_unique() {
        let mut names = HashSet::new();
        for part in PARTS {
            assert!(names.insert(part.name), "duplicate part {}", part.name);
        }
    }

    #[test]
    fn every_configured_coil_belongs_to_a_part_and_vice_versa() {
        let in_parts: HashSet<usize> = PARTS
            .iter()
            .flat_map(|p| p.coils.iter().map(|c| c.index()))
            .collect();
        for (i, kind) in COIL_KINDS.iter().enumerate() {
            let used = *kind != CoilKind::Unused;
            assert_eq!(
                used,
                in_parts.contains(&i),
                "coil {i}: COIL_KINDS says {kind:?} but PARTS {} it",
                if in_parts.contains(&i) {
                    "uses"
                } else {
                    "doesn't use"
                },
            );
        }
    }

    #[test]
    fn pulse_coils_have_a_finite_on_time() {
        for (i, kind) in COIL_KINDS.iter().enumerate() {
            if let CoilKind::Pulse { max_ms, .. } = *kind {
                assert!(
                    (1..=100).contains(&max_ms),
                    "coil {i}: max_ms {max_ms} outside 1–100 ms; check the coil rating"
                );
            }
        }
    }
}
