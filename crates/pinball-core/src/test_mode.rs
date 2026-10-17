//! Operator test mode, for bringing the machine up one part at a time.
//!
//! Press the test button (inside the coin door) in attract mode to enter it.
//! Each press moves on a stage, and the last one returns to attract mode:
//!
//! 1. **Switches:** closing any switch rings a bell, lights the part's own lamp
//!    while it's held, and reports which switch it was. Coil power is off.
//! 2. **Coils:** the flipper buttons choose a coil and Start fires it once, at
//!    its normal pulse length (holding coils get [`HOLD_TEST_MS`]). Only the
//!    chosen coil can fire, and only when Start is pressed.
//! 3. **Lamps:** the flipper buttons step through the lamps one at a time, and
//!    Start turns every lamp on or off together. Coil power is off.
//!
//! Each step produces a [`TestReport`]. The firmware prints it as a line of
//! text on the USB serial port ([`describe`]), and the simulator shows it.

use crate::coils::{COIL_COUNT, CoilId, CoilKind};
use crate::lamps::LampId;
use crate::playfield::{COIL_KINDS, PARTS, Part};
use crate::switches::SwitchId;

/// How long a holding coil (a flipper's hold winding or a trapdoor flap) is
/// energised by one press of Start. Pulse coils use their own limit instead.
pub const HOLD_TEST_MS: u16 = 250;

/// The stages of test mode, in order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TestStage {
    /// Report every switch that closes or opens.
    Switches,
    /// Choose a coil with the flipper buttons and fire it with Start.
    Coils,
    /// Step through the lamps with the flipper buttons.
    Lamps,
}

impl TestStage {
    /// The stage after this one, or `None` after the last.
    #[must_use]
    pub const fn next(self) -> Option<Self> {
        match self {
            Self::Switches => Some(Self::Coils),
            Self::Coils => Some(Self::Lamps),
            Self::Lamps => None,
        }
    }
}

/// Something test mode wants the operator to know.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TestReport {
    /// Entered a stage.
    Stage(TestStage),
    /// A switch closed or opened.
    Switch {
        /// Which switch.
        id: SwitchId,
        /// Whether it closed (rather than opened).
        closed: bool,
    },
    /// A coil was chosen, or Start tried to fire it.
    Coil {
        /// Which coil.
        id: CoilId,
        /// `None` when just chosen; otherwise whether the pulse started (it
        /// doesn't while the coil is still recycling from the last one).
        fired: Option<bool>,
    },
    /// A lamp was chosen and is lit on its own.
    Lamp(LampId),
    /// Every lamp was turned on or off.
    AllLamps(bool),
    /// Left test mode.
    Done,
}

/// The test reports from one tick, oldest first.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TestReports {
    reports: [Option<TestReport>; Self::CAPACITY],
    len: usize,
}

impl TestReports {
    /// Most reports kept per tick; later ones replace the newest.
    pub const CAPACITY: usize = 4;

    /// No reports.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            reports: [None; Self::CAPACITY],
            len: 0,
        }
    }

    /// Adds a report, replacing the newest if already full.
    pub fn push(&mut self, report: TestReport) {
        let i = self.len.min(Self::CAPACITY - 1);
        self.reports[i] = Some(report);
        self.len = i + 1;
    }

    /// Whether there are none.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// The reports, oldest first.
    pub fn iter(&self) -> impl Iterator<Item = TestReport> + '_ {
        self.reports[..self.len].iter().flatten().copied()
    }
}

/// The coils test mode can fire: every one with a limit in `COIL_KINDS`, in
/// number order.
fn usable_coil(n: usize) -> bool {
    COIL_KINDS[n] != CoilKind::Unused
}

/// The next usable coil after (or, with `forward == false`, before) `from`,
/// wrapping round.
#[must_use]
pub fn step_coil(from: CoilId, forward: bool) -> CoilId {
    let mut n = from.index();
    for _ in 0..COIL_COUNT {
        n = if forward {
            (n + 1) % COIL_COUNT
        } else {
            (n + COIL_COUNT - 1) % COIL_COUNT
        };
        if usable_coil(n) {
            break;
        }
    }
    CoilId::new(n as u8)
}

/// The first usable coil.
#[must_use]
pub fn first_coil() -> CoilId {
    step_coil(CoilId::new((COIL_COUNT - 1) as u8), true)
}

/// The next lamp used by a part after (or before) `from`, in number order,
/// wrapping round.
#[must_use]
pub fn step_lamp(from: LampId, forward: bool) -> LampId {
    let lamps = || PARTS.iter().filter_map(|p| p.lamp);
    let after = |a: LampId, b: LampId| a.index() > b.index();
    let found = if forward {
        lamps()
            .filter(|&l| after(l, from))
            .min_by_key(|l| l.index())
            .or_else(|| lamps().min_by_key(|l| l.index()))
    } else {
        lamps()
            .filter(|&l| after(from, l))
            .max_by_key(|l| l.index())
            .or_else(|| lamps().max_by_key(|l| l.index()))
    };
    found.unwrap_or(from)
}

/// The lowest-numbered lamp used by a part.
#[must_use]
pub fn first_lamp() -> LampId {
    PARTS
        .iter()
        .filter_map(|p| p.lamp)
        .min_by_key(|l| l.index())
        .unwrap_or(LampId::new(0))
}

/// The part with this switch.
#[must_use]
pub fn part_with_switch(id: SwitchId) -> Option<&'static Part> {
    PARTS.iter().find(|p| p.switch == Some(id))
}

/// The part with this coil.
#[must_use]
pub fn part_with_coil(id: CoilId) -> Option<&'static Part> {
    PARTS.iter().find(|p| p.coils.contains(&id))
}

/// The part with this lamp.
#[must_use]
pub fn part_with_lamp(id: LampId) -> Option<&'static Part> {
    PARTS.iter().find(|p| p.lamp == Some(id))
}

/// A report as one line of plain ASCII text, without a line ending.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReportText {
    bytes: [u8; Self::CAPACITY],
    len: usize,
}

impl ReportText {
    /// Longest line; anything longer is cut short.
    pub const CAPACITY: usize = 80;

    const fn new() -> Self {
        Self {
            bytes: [0; Self::CAPACITY],
            len: 0,
        }
    }

    /// The text as bytes.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes[..self.len]
    }

    /// The text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        // Only ASCII is ever pushed, and lines are cut at byte boundaries.
        core::str::from_utf8(self.as_bytes()).unwrap_or("")
    }

    fn push(&mut self, text: &str) {
        for &byte in text.as_bytes() {
            if self.len < Self::CAPACITY {
                self.bytes[self.len] = if byte.is_ascii() { byte } else { b'?' };
                self.len += 1;
            }
        }
    }

    fn push_number(&mut self, n: usize) {
        if n >= 10 {
            self.push_number(n / 10);
        }
        let digit = [b'0' + (n % 10) as u8];
        self.push(core::str::from_utf8(&digit).unwrap_or("?"));
    }

    fn push_part(&mut self, part: Option<&Part>) {
        self.push(": ");
        self.push(part.map_or("not in the parts list", |p| p.name));
    }
}

/// Writes a switch's position, such as `C2 R3` or `direct 5`.
fn push_switch(text: &mut ReportText, id: SwitchId) {
    let i = id.index();
    if i < 64 {
        text.push("C");
        text.push_number(i / 8);
        text.push(" R");
        text.push_number(i % 8);
    } else {
        text.push("direct ");
        text.push_number(i - 64);
    }
}

/// Describes a report in words, for the serial console and the simulator.
#[must_use]
pub fn describe(report: TestReport) -> ReportText {
    let mut text = ReportText::new();
    match report {
        TestReport::Stage(TestStage::Switches) => {
            text.push("TEST 1/3 switches: close any switch. Test button: next");
        }
        TestReport::Stage(TestStage::Coils) => {
            text.push("TEST 2/3 coils: flippers choose, Start fires. Test button: next");
        }
        TestReport::Stage(TestStage::Lamps) => {
            text.push("TEST 3/3 lamps: flippers choose, Start: all. Test button: exit");
        }
        TestReport::Switch { id, closed } => {
            text.push("switch ");
            push_switch(&mut text, id);
            text.push(if closed { " closed" } else { " opened" });
            text.push_part(part_with_switch(id));
        }
        TestReport::Coil { id, fired } => {
            text.push("coil ");
            text.push_number(id.index());
            text.push_part(part_with_coil(id));
            text.push(match fired {
                None => " (Start to fire)",
                Some(true) => " - fired",
                Some(false) => " - recycling, try again",
            });
        }
        TestReport::Lamp(id) => {
            text.push("lamp ");
            text.push_number(id.index());
            text.push_part(part_with_lamp(id));
        }
        TestReport::AllLamps(on) => {
            text.push(if on { "all lamps on" } else { "all lamps off" });
        }
        TestReport::Done => text.push("TEST done"),
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::playfield::{coil, lamp, sw};

    #[test]
    fn coil_selection_skips_unused_outputs_and_wraps() {
        let first = first_coil();
        assert_eq!(first, coil::BALL_SERVE);
        // Every usable coil is reached once on the way round.
        let mut seen = 0;
        let mut c = first;
        loop {
            assert!(usable_coil(c.index()), "{c:?}");
            seen += 1;
            c = step_coil(c, true);
            if c == first {
                break;
            }
        }
        let usable = (0..COIL_COUNT).filter(|&n| usable_coil(n)).count();
        assert_eq!(seen, usable);
        // Backwards from the first wraps to the last.
        let last = step_coil(first, false);
        assert_eq!(step_coil(last, true), first);
    }

    #[test]
    fn lamp_selection_visits_every_part_lamp_in_order() {
        let first = first_lamp();
        assert_eq!(first, lamp::GAME_OVER);
        let mut seen = vec![first];
        let mut l = step_lamp(first, true);
        while l != first {
            assert!(l.index() > seen.last().unwrap().index() || seen.len() > 1);
            seen.push(l);
            l = step_lamp(l, true);
        }
        let mut expected: Vec<_> = PARTS.iter().filter_map(|p| p.lamp).collect();
        expected.sort_by_key(|l| l.index());
        assert_eq!(seen, expected);
        assert_eq!(step_lamp(first, false), *expected.last().unwrap());
    }

    #[test]
    fn reports_name_the_part() {
        let switch = TestReport::Switch {
            id: sw::POP_BUMPERS[3],
            closed: true,
        };
        assert_eq!(
            describe(switch).as_str(),
            "switch C2 R3 closed: Pop bumper 4 (lower left)"
        );
        assert_eq!(
            describe(TestReport::Switch {
                id: sw::START_BUTTON,
                closed: false
            })
            .as_str(),
            "switch direct 2 opened: Start button"
        );
        assert_eq!(
            describe(TestReport::Coil {
                id: coil::TOP_BUMPER,
                fired: Some(true)
            })
            .as_str(),
            "coil 18: Top bumper (10 points) - fired"
        );
        assert_eq!(
            describe(TestReport::Lamp(lamp::SPECIAL)).as_str(),
            "lamp 2: Special when lit"
        );
    }

    #[test]
    fn every_report_fits_on_one_line() {
        let mut reports = vec![TestReport::AllLamps(true), TestReport::Done];
        for stage in [TestStage::Switches, TestStage::Coils, TestStage::Lamps] {
            reports.push(TestReport::Stage(stage));
        }
        for part in PARTS {
            if let Some(id) = part.switch {
                reports.push(TestReport::Switch { id, closed: true });
            }
            for &id in part.coils {
                reports.push(TestReport::Coil {
                    id,
                    fired: Some(false),
                });
            }
            if let Some(id) = part.lamp {
                reports.push(TestReport::Lamp(id));
            }
        }
        for report in reports {
            let text = describe(report);
            assert!(
                text.len < ReportText::CAPACITY,
                "{report:?}: {}",
                text.as_str()
            );
        }
    }
}
