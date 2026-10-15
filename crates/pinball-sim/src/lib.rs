//! A browser simulator for the Fun House game logic.
//!
//! This wraps the real [`pinball_core::Machine`] with a simulated clock and
//! simulated switches, and exports a tiny numeric API to WebAssembly for the
//! page in `sim/web/`. There's no ball physics: the player triggers switches by
//! clicking parts of the playfield or using the keyboard.

use pinball_core::{Instant, Machine, Outputs, Phase, SwitchId, SwitchStates};

/// How long a clicked switch stays closed, in simulated milliseconds. Long
/// enough to get through the debouncer.
pub const TAP_MS: u32 = 50;

/// The most simulated time a single [`Simulator::advance`] will run. Stops a tab
/// that was in the background from replaying minutes of game time at once.
pub const MAX_ADVANCE_MS: u32 = 250;

/// Converts a dense switch index (as used in `data-switch` attributes:
/// `column * 8 + row` for the matrix, `64 + n` for direct inputs) to a
/// [`SwitchId`].
#[must_use]
pub fn switch_from_index(index: u32) -> Option<SwitchId> {
    match index {
        0..64 => Some(SwitchId::matrix((index / 8) as u8, (index % 8) as u8)),
        64..72 => Some(SwitchId::direct((index - 64) as u8)),
        _ => None,
    }
}

/// A [`Machine`] driven by simulated time and switches.
#[derive(Clone, Debug)]
pub struct Simulator {
    machine: Machine,
    switches: SwitchStates,
    now: Instant,
    /// Switches closed by [`Simulator::tap`], with when to release each one.
    taps: Vec<(SwitchId, Instant)>,
    outputs: Outputs,
    /// Every coil that was on at any point during the last [`Simulator::advance`].
    coils_seen: u32,
}

impl Default for Simulator {
    fn default() -> Self {
        Self::new()
    }
}

impl Simulator {
    /// A freshly powered-up machine.
    #[must_use]
    pub fn new() -> Self {
        Self {
            machine: Machine::new(),
            switches: SwitchStates::new(),
            now: Instant::from_millis(0),
            taps: Vec::new(),
            outputs: Outputs::default(),
            coils_seen: 0,
        }
    }

    /// The game state, for reading score, ball, phase and so on.
    #[must_use]
    pub const fn machine(&self) -> &Machine {
        &self.machine
    }

    /// The coil and lamp outputs after the last simulated millisecond.
    #[must_use]
    pub const fn outputs(&self) -> Outputs {
        self.outputs
    }

    /// Every coil that was on at any point during the last
    /// [`Simulator::advance`]. A 20 ms pulse can start and end between two
    /// browser frames; this lets the page still show it.
    #[must_use]
    pub const fn coils_seen(&self) -> u32 {
        self.coils_seen
    }

    /// Holds a switch closed or open, e.g. a flipper button.
    pub fn set_switch(&mut self, id: SwitchId, closed: bool) {
        self.taps.retain(|&(tapped, _)| tapped != id);
        self.switches.set(id, closed);
    }

    /// Closes a switch briefly, as a ball rolling over it would.
    pub fn tap(&mut self, id: SwitchId) {
        self.set_switch(id, true);
        self.taps.push((id, self.now.plus(TAP_MS)));
    }

    /// Opens or closes trapdoor `n` (0–2). Until the rule for opening the flaps
    /// is known (see #24), the page lets the player do it by hand. Out-of-range
    /// `n` is ignored.
    pub fn set_trapdoor(&mut self, n: usize, open: bool) {
        if n < 3 {
            self.machine.set_trapdoor(n, open);
        }
    }

    /// Runs the machine for `ms` simulated milliseconds (capped at
    /// [`MAX_ADVANCE_MS`]), one 1 ms tick at a time as the firmware does.
    pub fn advance(&mut self, ms: u32) {
        self.coils_seen = 0;
        for _ in 0..ms.min(MAX_ADVANCE_MS) {
            self.now = self.now.plus(1);
            let now = self.now;
            let switches = &mut self.switches;
            self.taps.retain(|&(id, release_at)| {
                let due = now.reached(release_at);
                if due {
                    switches.set(id, false);
                }
                !due
            });
            self.outputs = self.machine.tick(now, &self.switches);
            self.coils_seen |= self.outputs.coils;
        }
    }
}

/// The WebAssembly API used by `sim/web/sim.js`. Everything is a plain number so
/// the page needs no generated bindings.
#[allow(unsafe_code)] // `#[unsafe(no_mangle)]` only; no `unsafe` blocks.
mod exports {
    use std::cell::RefCell;

    use super::{Phase, Simulator, switch_from_index};

    thread_local! {
        static SIM: RefCell<Simulator> = RefCell::new(Simulator::new());
    }

    fn with<R>(f: impl FnOnce(&mut Simulator) -> R) -> R {
        SIM.with_borrow_mut(f)
    }

    /// Power-cycles the machine.
    #[unsafe(no_mangle)]
    pub extern "C" fn sim_reset() {
        with(|sim| *sim = Simulator::new());
    }

    /// Holds switch `index` closed (`closed != 0`) or open.
    #[unsafe(no_mangle)]
    pub extern "C" fn sim_set_switch(index: u32, closed: u32) {
        if let Some(id) = switch_from_index(index) {
            with(|sim| sim.set_switch(id, closed != 0));
        }
    }

    /// Closes switch `index` briefly.
    #[unsafe(no_mangle)]
    pub extern "C" fn sim_tap(index: u32) {
        if let Some(id) = switch_from_index(index) {
            with(|sim| sim.tap(id));
        }
    }

    /// Opens (`open != 0`) or closes trapdoor `n`.
    #[unsafe(no_mangle)]
    pub extern "C" fn sim_set_trapdoor(n: u32, open: u32) {
        with(|sim| sim.set_trapdoor(n as usize, open != 0));
    }

    /// Runs the machine for `ms` simulated milliseconds.
    #[unsafe(no_mangle)]
    pub extern "C" fn sim_advance(ms: u32) {
        with(|sim| sim.advance(ms));
    }

    /// Coil output bitmap.
    #[unsafe(no_mangle)]
    pub extern "C" fn sim_coils() -> u32 {
        with(|sim| sim.outputs().coils)
    }

    /// Every coil that was on during the last advance, for showing short pulses.
    #[unsafe(no_mangle)]
    pub extern "C" fn sim_coils_seen() -> u32 {
        with(|sim| sim.coils_seen())
    }

    /// Lamps 0–31 as a bitmap.
    #[unsafe(no_mangle)]
    pub extern "C" fn sim_lamps_low() -> u32 {
        with(|sim| sim.outputs().lamps as u32)
    }

    /// Lamps 32–63 as a bitmap.
    #[unsafe(no_mangle)]
    pub extern "C" fn sim_lamps_high() -> u32 {
        with(|sim| (sim.outputs().lamps >> 32) as u32)
    }

    /// Current score.
    #[unsafe(no_mangle)]
    pub extern "C" fn sim_score() -> u32 {
        with(|sim| sim.machine().score())
    }

    /// Current ball (0 when no game is running).
    #[unsafe(no_mangle)]
    pub extern "C" fn sim_ball() -> u32 {
        with(|sim| u32::from(sim.machine().ball()))
    }

    /// Replays earned this game.
    #[unsafe(no_mangle)]
    pub extern "C" fn sim_replays() -> u32 {
        with(|sim| u32::from(sim.machine().replays()))
    }

    /// Value of the lit bonus-wheel position.
    #[unsafe(no_mangle)]
    pub extern "C" fn sim_bonus() -> u32 {
        with(|sim| sim.machine().bonus())
    }

    /// 0 = attract, 1 = playing, 2 = tilted.
    #[unsafe(no_mangle)]
    pub extern "C" fn sim_phase() -> u32 {
        with(|sim| match sim.machine().phase() {
            Phase::Attract => 0,
            Phase::Playing => 1,
            Phase::Tilted => 2,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use pinball_core::playfield::{coil, lamp, sw};
    use pinball_core::{CoilId, LampId};

    use super::*;

    #[test]
    fn a_tap_is_long_enough_to_register_and_then_releases() {
        let mut sim = Simulator::new();
        sim.tap(sw::START_BUTTON);
        sim.advance(TAP_MS + 10);
        assert_eq!(sim.machine().phase(), Phase::Playing);

        // Released: tapping again mid-game doesn't restart anything.
        sim.tap(sw::LEFT_KICKER);
        sim.advance(TAP_MS + 10);
        assert_eq!(sim.machine().score(), 1);
    }

    #[test]
    fn a_held_flipper_button_holds_the_flipper_up() {
        let mut sim = Simulator::new();
        sim.tap(sw::START_BUTTON);
        sim.advance(100);
        sim.set_switch(sw::LEFT_FLIPPER_BUTTON, true);
        sim.advance(200);
        let hold = 1 << coil::LEFT_FLIPPER_HOLD.index();
        assert_ne!(sim.outputs().coils & hold, 0);
        sim.set_switch(sw::LEFT_FLIPPER_BUTTON, false);
        sim.advance(10);
        assert_eq!(sim.outputs().coils & hold, 0);
    }

    #[test]
    fn trapdoors_can_be_opened_by_hand() {
        let mut sim = Simulator::new();
        sim.tap(sw::START_BUTTON);
        sim.advance(100);
        sim.set_trapdoor(1, true);
        sim.set_trapdoor(7, true); // ignored
        sim.advance(10);
        assert_eq!(sim.outputs().coils, 1 << coil::TRAPDOOR_FLAPS[1].index());
    }

    #[test]
    fn a_pulse_inside_one_advance_is_still_seen() {
        let mut sim = Simulator::new();
        sim.tap(sw::START_BUTTON);
        sim.advance(100);
        sim.tap(sw::LEFT_KICKER);
        // Long enough for the 20 ms kicker pulse to start and finish.
        sim.advance(100);
        let kicker = 1 << coil::LEFT_KICKER.index();
        assert_eq!(sim.outputs().coils & kicker, 0);
        assert_ne!(sim.coils_seen() & kicker, 0);
    }

    #[test]
    fn advance_is_capped() {
        let mut sim = Simulator::new();
        sim.advance(u32::MAX);
        assert_eq!(sim.now.as_millis(), MAX_ADVANCE_MS);
    }

    #[test]
    fn switch_indices_round_trip() {
        assert_eq!(switch_from_index(2 * 8 + 3), Some(SwitchId::matrix(2, 3)));
        assert_eq!(switch_from_index(64 + 1), Some(sw::RIGHT_FLIPPER_BUTTON));
        assert_eq!(switch_from_index(72), None);
    }

    // The simulator page finds parts by the `data-switch`, `data-coil` and
    // `data-lamp` attributes in `docs/playfield.svg` and `sim/web/index.html`.
    // These tests keep them in step with `playfield.rs`: they fail if a part is
    // missing from the page, or if the page refers to one that doesn't exist.
    // Add new parts to the lists below as well as to the SVG.

    const SVG: &str = include_str!("../../../docs/playfield.svg");
    const HTML: &str = include_str!("../../../sim/web/index.html");
    // The tests below compare the page against lists of `playfield.rs`
    // constants written out by hand, so a new constant must be added to them.
    // Keyboard shortcuts live on the `index.html` buttons (`data-key`), so
    // they're covered too.

    fn attribute_values(attribute: &str) -> BTreeSet<String> {
        let needle = format!("{attribute}=\"");
        [SVG, HTML]
            .iter()
            .flat_map(|doc| doc.split(needle.as_str()).skip(1))
            .flat_map(|rest| rest.split('"').next().unwrap_or("").split_whitespace())
            .map(str::to_owned)
            .collect()
    }

    fn switch_label(id: SwitchId) -> String {
        let i = id.index();
        if i < 64 {
            format!("C{}R{}", i / 8, i % 8)
        } else {
            format!("D{}", i - 64)
        }
    }

    #[test]
    fn page_switches_match_the_playfield() {
        let mut expected = vec![
            sw::LEFT_FLIPPER_BUTTON,
            sw::RIGHT_FLIPPER_BUTTON,
            sw::START_BUTTON,
            sw::TILT,
            sw::SLAM_TILT,
            sw::OUTHOLE,
            sw::SHOOTER_LANE,
            sw::LEFT_KICKER,
            sw::RIGHT_KICKER,
            sw::TOP_CENTRE_BUMPER,
            sw::BONUS_GATE,
        ];
        expected.extend(sw::POP_BUMPERS);
        expected.extend(sw::TOP_LANES);
        expected.extend(sw::SIDE_LANES);
        expected.extend(sw::THIRTY_POINT_ROLLOVERS);
        expected.extend(sw::TRAPDOORS);
        let expected: BTreeSet<_> = expected.into_iter().map(switch_label).collect();
        assert_eq!(attribute_values("data-switch"), expected);
    }

    #[test]
    fn page_coils_match_the_playfield() {
        let mut expected = vec![
            coil::BALL_SERVE,
            coil::LEFT_KICKER,
            coil::RIGHT_KICKER,
            coil::LEFT_FLIPPER_POWER,
            coil::LEFT_FLIPPER_HOLD,
            coil::RIGHT_FLIPPER_POWER,
            coil::RIGHT_FLIPPER_HOLD,
        ];
        expected.extend(coil::POP_BUMPERS);
        expected.extend(coil::TRAPDOOR_FLAPS);
        let expected: BTreeSet<_> = expected
            .into_iter()
            .map(|c: CoilId| c.index().to_string())
            .collect();
        assert_eq!(attribute_values("data-coil"), expected);
    }

    #[test]
    fn page_lamps_match_the_playfield() {
        let mut expected = vec![
            lamp::GAME_OVER,
            lamp::TILT,
            lamp::SPECIAL,
            lamp::BONUS_WHEEL_CENTRE,
        ];
        expected.extend(lamp::BONUS_WHEEL);
        expected.extend(lamp::THIRTY_POINTS);
        expected.extend(lamp::POP_BUMPERS);
        let expected: BTreeSet<_> = expected
            .into_iter()
            .map(|l: LampId| l.index().to_string())
            .collect();
        assert_eq!(attribute_values("data-lamp"), expected);
    }
}
