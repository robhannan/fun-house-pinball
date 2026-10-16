//! A browser simulator for the Fun House game logic.
//!
//! This wraps the real [`pinball_core::Machine`] with a simulated clock and
//! simulated switches, and exports a tiny numeric API to WebAssembly for the
//! page in `sim/web/`. There's no ball physics: the player triggers switches by
//! clicking parts of the playfield or using the keyboard.

use pinball_core::{
    Instant, MAX_PLAYERS, Machine, Outputs, Phase, Settings, SwitchId, SwitchStates,
};

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
    /// A freshly powered-up machine with the default operator settings.
    #[must_use]
    pub fn new() -> Self {
        Self::with_settings(Settings::DEFAULT)
    }

    /// A freshly powered-up machine with the given operator settings.
    #[must_use]
    pub fn with_settings(settings: Settings) -> Self {
        Self {
            machine: Machine::with_settings(settings),
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

    use super::{MAX_PLAYERS, Phase, Settings, Simulator, switch_from_index};

    thread_local! {
        static SIM: RefCell<Simulator> = RefCell::new(Simulator::new());
    }

    fn with<R>(f: impl FnOnce(&mut Simulator) -> R) -> R {
        SIM.with_borrow_mut(f)
    }

    /// Power-cycles the machine with the default operator settings.
    #[unsafe(no_mangle)]
    pub extern "C" fn sim_reset() {
        with(|sim| *sim = Simulator::new());
    }

    /// Power-cycles the machine with `balls` per game (clamped to 3–5) and
    /// replays on (`replays != 0`) or novelty mode.
    #[unsafe(no_mangle)]
    pub extern "C" fn sim_reset_with(balls: u32, replays: u32) {
        let settings = Settings {
            balls_per_game: balls.min(5) as u8,
            replays: replays != 0,
        };
        with(|sim| *sim = Simulator::with_settings(settings));
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

    /// Current player's score.
    #[unsafe(no_mangle)]
    pub extern "C" fn sim_score() -> u32 {
        with(|sim| sim.machine().score())
    }

    /// Player `n`'s score (0-based); 0 for an out-of-range `n`.
    #[unsafe(no_mangle)]
    pub extern "C" fn sim_score_of(n: u32) -> u32 {
        let n = n as usize;
        if n < MAX_PLAYERS {
            with(|sim| sim.machine().score_of(n))
        } else {
            0
        }
    }

    /// Players in the current (or last) game.
    #[unsafe(no_mangle)]
    pub extern "C" fn sim_players() -> u32 {
        with(|sim| u32::from(sim.machine().players()))
    }

    /// Whose turn it is (1-based; 0 when no game is running).
    #[unsafe(no_mangle)]
    pub extern "C" fn sim_current_player() -> u32 {
        with(|sim| u32::from(sim.machine().current_player()))
    }

    /// Credits shown in the backboard's credit window.
    #[unsafe(no_mangle)]
    pub extern "C" fn sim_credits() -> u32 {
        with(|sim| u32::from(sim.machine().credits()))
    }

    /// Balls per game in force.
    #[unsafe(no_mangle)]
    pub extern "C" fn sim_balls_per_game() -> u32 {
        with(|sim| u32::from(sim.machine().settings().balls_per_game))
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

    use pinball_core::LampId;
    use pinball_core::playfield::{PARTS, Part, coil, sw};

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
    // `data-lamp` attributes in `docs/playfield.svg` and `sim/web/index.html`
    // (keyboard shortcuts live on the index.html buttons). These tests check
    // the page against `PARTS` in `playfield.rs`, so adding a part there without
    // drawing it, or drawing one with the wrong numbers, fails CI.

    const SVG: &str = include_str!("../../../docs/playfield.svg");
    const BACKBOARD: &str = include_str!("../../../docs/backboard.svg");
    const HTML: &str = include_str!("../../../sim/web/index.html");

    /// The I/O one page element refers to.
    #[derive(Debug, Default)]
    struct Element {
        switch: Option<String>,
        coils: Option<BTreeSet<usize>>,
        lamp: Option<usize>,
    }

    /// Every element on the page with at least one of the attributes.
    fn page_elements() -> Vec<Element> {
        fn attribute<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
            let start = tag.find(&format!(" {name}=\""))? + name.len() + 3;
            tag[start..].split('"').next()
        }
        [SVG, BACKBOARD, HTML]
            .iter()
            .flat_map(|doc| doc.split('<').skip(1))
            .map(|tag| tag.split('>').next().unwrap_or(""))
            .map(|tag| Element {
                switch: attribute(tag, "data-switch").map(str::to_owned),
                coils: attribute(tag, "data-coil").map(|coils| {
                    coils
                        .split_whitespace()
                        .map(|c| c.parse().expect("data-coil is a number"))
                        .collect()
                }),
                lamp: attribute(tag, "data-lamp")
                    .map(|l| l.parse().expect("data-lamp is a number")),
            })
            .filter(|e| e.switch.is_some() || e.coils.is_some() || e.lamp.is_some())
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

    fn part_coils(part: &Part) -> BTreeSet<usize> {
        part.coils.iter().map(|c| c.index()).collect()
    }

    /// The part an element belongs to: by its switch, else its coils, else its lamp.
    fn part_for(element: &Element) -> Option<&'static Part> {
        PARTS.iter().find(|part| {
            if let Some(switch) = &element.switch {
                part.switch.map(switch_label).as_ref() == Some(switch)
            } else if let Some(coils) = &element.coils {
                &part_coils(part) == coils
            } else {
                part.lamp.map(LampId::index) == element.lamp
            }
        })
    }

    #[test]
    fn every_page_element_matches_one_part() {
        for element in page_elements() {
            let part = part_for(&element)
                .unwrap_or_else(|| panic!("{element:?} doesn't match any part in PARTS"));
            if let Some(coils) = &element.coils {
                assert_eq!(coils, &part_coils(part), "{}: wrong data-coil", part.name);
            }
            if let Some(lamp) = element.lamp {
                assert_eq!(
                    Some(lamp),
                    part.lamp.map(LampId::index),
                    "{}: wrong data-lamp",
                    part.name
                );
            }
        }
    }

    #[test]
    fn backboard_has_three_score_reels_per_player_and_a_credit_window() {
        for player in 0..MAX_PLAYERS {
            for digit in 0..3 {
                let reel = format!("data-score=\"{player}\" data-digit=\"{digit}\"");
                assert_eq!(BACKBOARD.matches(&reel).count(), 1, "missing {reel}");
            }
        }
        assert_eq!(BACKBOARD.matches("data-credits=").count(), 1);
    }

    #[test]
    fn simulator_settings_reach_the_machine() {
        let sim = Simulator::with_settings(Settings {
            balls_per_game: 3,
            replays: false,
        });
        assert_eq!(sim.machine().settings().balls_per_game, 3);
        assert!(!sim.machine().settings().replays);
    }

    #[test]
    fn every_part_is_on_the_page() {
        let elements = page_elements();
        for part in PARTS {
            if let Some(switch) = part.switch {
                let label = switch_label(switch);
                assert!(
                    elements.iter().any(|e| e.switch.as_ref() == Some(&label)),
                    "{}: no element with data-switch=\"{label}\"",
                    part.name
                );
            }
            if !part.coils.is_empty() {
                assert!(
                    elements
                        .iter()
                        .any(|e| e.coils.as_ref() == Some(&part_coils(part))),
                    "{}: no element with its data-coil",
                    part.name
                );
            }
            if let Some(lamp) = part.lamp {
                assert!(
                    elements.iter().any(|e| e.lamp == Some(lamp.index())),
                    "{}: no element with data-lamp=\"{}\"",
                    part.name,
                    lamp.index()
                );
            }
        }
    }
}
