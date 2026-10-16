//! The game rules: a state machine driven by switch events and time.
//!
//! Scores follow the original's 1950s scale (a replay at 400 points). Point
//! values printed on the playfield are used where visible; the rest, and the
//! bonus-wheel rules, are assumptions to check against the schematic.
//!
//! From the flyer: "1 to 4 can play", "adjustable to 3-4-or 5 ball play",
//! "Novelty or Replay", and a "Multi-Tilt feature [that] permits player to
//! continue playing in turn if one player tilts game".

use crate::coils::{CoilId, Coils};
use crate::lamps::{LampMode, Lamps};
use crate::playfield::{COIL_KINDS, coil, lamp, sw};
use crate::switches::{Debouncer, SwitchEvent, SwitchId, SwitchStates};
use crate::time::Instant;

/// Most players in one game ("1 to 4 can play").
pub const MAX_PLAYERS: usize = 4;
/// The credit window shows two digits (*verify*).
pub const MAX_CREDITS: u8 = 99;
/// Tilt warnings allowed before the ball is tilted. Classic EM rules: none.
pub const TILT_WARNINGS: u8 = 0;
/// Consecutive matching scans before a switch change is accepted.
pub const DEBOUNCE_SCANS: u8 = 2;
/// Delay between a ball draining and the next one being served, giving the
/// ball time to settle in the outhole.
pub const SERVE_DELAY_MS: u32 = 1_000;

/// From the instruction card: one replay for each score of 400, 600, 700,
/// 800 and 900 points.
pub const REPLAY_SCORES: [u32; 5] = [400, 600, 700, 800, 900];

const FLIPPER_POWER_MS: u16 = 40;

/// Operator settings, set inside the cabinet on the original.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Settings {
    /// Balls per player: 3, 4 or 5 ("adjustable to 3-4-or 5 ball play").
    pub balls_per_game: u8,
    /// Replay mode awards replays; novelty mode only scores points.
    pub replays: bool,
}

impl Settings {
    /// As the flyer advertises it: 5-ball, replay.
    pub const DEFAULT: Self = Self {
        balls_per_game: 5,
        replays: true,
    };
}

impl Default for Settings {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// What the machine is doing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    /// No game running; waiting for the start button.
    Attract,
    /// A ball is in play (or waiting to be served).
    Playing,
    /// The current ball was tilted: flippers and playfield coils are dead
    /// until it drains.
    Tilted,
}

/// Everything the firmware must drive for this tick.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Outputs {
    /// Coil driver bitmap; bit `n` energises coil `n`.
    pub coils: u32,
    /// Lamp bitmap; bit `n` lights lamp `n`.
    pub lamps: u64,
}

/// The whole machine: switch filtering, rules, coils and lamps.
#[derive(Clone, Debug)]
pub struct Machine {
    debouncer: Debouncer,
    coils: Coils,
    lamps: Lamps,
    settings: Settings,
    phase: Phase,
    ball: u8,
    /// Players in the current (or last) game; 0 before the first game.
    players: u8,
    /// Whose turn it is, 0-based.
    player: u8,
    scores: [u32; MAX_PLAYERS],
    replays: [u8; MAX_PLAYERS],
    /// Credits on the backboard's credit window (in the clown's nose).
    credits: u8,
    /// Lit position on the bonus wheel: 0 = 10 points ... 9 = 100 points.
    bonus_step: u8,
    tilt_warnings: u8,
    serve_at: Option<Instant>,
}

impl Default for Machine {
    fn default() -> Self {
        Self::new()
    }
}

impl Machine {
    /// A powered-up machine in attract mode, with the default settings.
    #[must_use]
    pub fn new() -> Self {
        Self::with_settings(Settings::DEFAULT)
    }

    /// A powered-up machine in attract mode. `balls_per_game` is clamped to
    /// 3–5.
    #[must_use]
    pub fn with_settings(settings: Settings) -> Self {
        let settings = Settings {
            balls_per_game: settings.balls_per_game.clamp(3, 5),
            ..settings
        };
        let mut machine = Self {
            debouncer: Debouncer::new(DEBOUNCE_SCANS),
            coils: Coils::new(COIL_KINDS),
            lamps: Lamps::new(),
            settings,
            phase: Phase::Attract,
            ball: 0,
            players: 0,
            player: 0,
            scores: [0; MAX_PLAYERS],
            replays: [0; MAX_PLAYERS],
            credits: 0,
            bonus_step: 0,
            tilt_warnings: 0,
            serve_at: None,
        };
        machine.enter(Phase::Attract);
        machine
    }

    /// Current phase.
    #[must_use]
    pub const fn phase(&self) -> Phase {
        self.phase
    }

    /// Current ball number (1-based; 0 in attract mode).
    #[must_use]
    pub const fn ball(&self) -> u8 {
        self.ball
    }

    /// The operator settings in force.
    #[must_use]
    pub const fn settings(&self) -> Settings {
        self.settings
    }

    /// Players in the current game (or the last one, in attract mode); 0
    /// before the first game.
    #[must_use]
    pub const fn players(&self) -> u8 {
        self.players
    }

    /// Whose turn it is (1-based; 0 in attract mode).
    #[must_use]
    pub const fn current_player(&self) -> u8 {
        if matches!(self.phase, Phase::Attract) {
            0
        } else {
            self.player + 1
        }
    }

    /// The current player's score (in attract mode, player 1's last score).
    #[must_use]
    pub const fn score(&self) -> u32 {
        self.scores[self.player as usize]
    }

    /// Player `n`'s score (0-based). Scores stay on show after the game ends,
    /// as they did on the score reels.
    ///
    /// # Panics
    ///
    /// If `n` is 4 or more.
    #[must_use]
    pub const fn score_of(&self, n: usize) -> u32 {
        self.scores[n]
    }

    /// Credits shown in the backboard's credit window: one per replay won,
    /// kept across games, up to [`MAX_CREDITS`].
    ///
    /// TODO (#27): spend a credit per player once coins and credits are
    /// wired; until then starting a game is free.
    #[must_use]
    pub const fn credits(&self) -> u8 {
        self.credits
    }

    /// Replays earned this game, by all players.
    #[must_use]
    pub fn replays(&self) -> u8 {
        self.replays.iter().sum()
    }

    /// Value of the lit bonus-wheel position.
    #[must_use]
    pub const fn bonus(&self) -> u32 {
        10 * (self.bonus_step as u32 + 1)
    }

    /// Advances the machine by one scan. Call roughly every millisecond with
    /// the freshly read raw switch states, then drive the returned outputs.
    pub fn tick(&mut self, now: Instant, raw: &SwitchStates) -> Outputs {
        for event in self.debouncer.update(raw) {
            self.on_switch(event, now);
        }
        if self.serve_at.is_some_and(|at| now.reached(at)) {
            self.serve_at = None;
            self.coils.pulse(coil::BALL_SERVE, u16::MAX, now);
        }
        Outputs {
            coils: self.coils.outputs(now),
            lamps: self.lamps.frame(now),
        }
    }

    fn on_switch(&mut self, event: SwitchEvent, now: Instant) {
        match (self.phase, event) {
            (_, SwitchEvent::Closed(sw::SLAM_TILT)) => self.enter(Phase::Attract),

            (Phase::Attract, SwitchEvent::Closed(sw::START_BUTTON)) => self.start_game(now),
            // Assumed (*verify*): players can be added while ball 1 is in play.
            (Phase::Playing | Phase::Tilted, SwitchEvent::Closed(sw::START_BUTTON)) => {
                if self.ball == 1 && usize::from(self.players) < MAX_PLAYERS {
                    self.players += 1;
                }
            }

            (Phase::Playing | Phase::Tilted, SwitchEvent::Closed(sw::OUTHOLE)) => {
                self.end_ball(now);
            }

            (Phase::Playing, SwitchEvent::Closed(id)) => self.on_playfield_hit(id, now),
            (Phase::Playing, SwitchEvent::Opened(id)) => self.on_flipper_button(id, false, now),

            _ => {}
        }
    }

    fn on_playfield_hit(&mut self, id: SwitchId, now: Instant) {
        let kick = |coils: &mut Coils, coil: CoilId| coils.pulse(coil, u16::MAX, now);
        match id {
            sw::LEFT_FLIPPER_BUTTON | sw::RIGHT_FLIPPER_BUTTON => {
                self.on_flipper_button(id, true, now);
            }
            sw::TILT => {
                self.tilt_warnings += 1;
                if self.tilt_warnings > TILT_WARNINGS {
                    self.enter(Phase::Tilted);
                }
            }
            sw::LEFT_KICKER => {
                kick(&mut self.coils, coil::LEFT_KICKER);
                self.add_score(1);
            }
            sw::RIGHT_KICKER => {
                kick(&mut self.coils, coil::RIGHT_KICKER);
                self.add_score(1);
            }
            sw::TOP_CENTRE_BUMPER => self.add_score(1),
            sw::BONUS_GATE => self.collect_bonus(),
            // Assumed (*verify*): the ball drops under the playfield and rolls
            // to the outhole, which ends the ball as usual.
            id if sw::TRAPDOORS.contains(&id) => self.close_trapdoors(),
            _ => {
                if let Some(n) = sw::POP_BUMPERS.iter().position(|&pop| pop == id) {
                    kick(&mut self.coils, coil::POP_BUMPERS[n]);
                    self.add_score(1);
                } else if sw::SIDE_LANES.contains(&id) || sw::TOP_LANES.contains(&id) {
                    self.add_score(10);
                    self.advance_bonus();
                } else if let Some(n) = sw::THIRTY_POINT_ROLLOVERS.iter().position(|&r| r == id) {
                    // TODO: find out what lights these on the original.
                    let lit = self.lamps.mode(lamp::THIRTY_POINTS[n]) != LampMode::Off;
                    self.add_score(if lit { 30 } else { 1 });
                }
            }
        }
    }

    /// Opens or closes trapdoor `n` (0–2, left to right) under the "Scores
    /// Bonus" gate. Only works while a ball is in play.
    ///
    /// TODO: call this from the rules once we know what opens the flaps on
    /// the original.
    ///
    /// # Panics
    ///
    /// If `n` is not 0, 1 or 2.
    pub fn set_trapdoor(&mut self, n: usize, open: bool) {
        if self.phase == Phase::Playing {
            self.coils.hold(coil::TRAPDOOR_FLAPS[n], open);
        }
    }

    fn close_trapdoors(&mut self) {
        for flap in coil::TRAPDOOR_FLAPS {
            self.coils.hold(flap, false);
        }
    }

    /// Adds to the current player's score, awarding replays at the card's
    /// thresholds.
    fn add_score(&mut self, points: u32) {
        let player = usize::from(self.player);
        let before = self.scores[player];
        self.scores[player] += points;
        let after = self.scores[player];
        let earned = REPLAY_SCORES
            .iter()
            .filter(|&&threshold| before < threshold && threshold <= after)
            .count();
        for _ in 0..earned {
            self.award_replay();
        }
    }

    /// A replay for the current player, unless the machine is set to novelty.
    fn award_replay(&mut self) {
        // TODO: fire a knocker and/or drive a credit unit.
        if self.settings.replays {
            self.replays[usize::from(self.player)] += 1;
            self.credits = (self.credits + 1).min(MAX_CREDITS);
        }
    }

    /// Assumed rule: each lane advances the bonus wheel one step; reaching
    /// 100 lights the special.
    fn advance_bonus(&mut self) {
        if self.bonus_step < 9 {
            self.lamps.set(
                lamp::BONUS_WHEEL[usize::from(self.bonus_step)],
                LampMode::Off,
            );
            self.bonus_step += 1;
            self.lamps.set(
                lamp::BONUS_WHEEL[usize::from(self.bonus_step)],
                LampMode::On,
            );
        }
        if self.bonus_step == 9 {
            self.lamps.set(lamp::SPECIAL, LampMode::On);
        }
    }

    /// Assumed rule: the "Scores Bonus" gate scores the lit wheel value, plus
    /// a replay (once) when the special is lit.
    fn collect_bonus(&mut self) {
        self.add_score(self.bonus());
        if self.lamps.mode(lamp::SPECIAL) != LampMode::Off {
            self.lamps.set(lamp::SPECIAL, LampMode::Off);
            self.award_replay();
        }
    }

    /// Flippers: a short burst on the power winding, then the hold winding
    /// for as long as the button is held.
    fn on_flipper_button(&mut self, id: SwitchId, pressed: bool, now: Instant) {
        let (power, hold) = match id {
            sw::LEFT_FLIPPER_BUTTON => (coil::LEFT_FLIPPER_POWER, coil::LEFT_FLIPPER_HOLD),
            sw::RIGHT_FLIPPER_BUTTON => (coil::RIGHT_FLIPPER_POWER, coil::RIGHT_FLIPPER_HOLD),
            _ => return,
        };
        if pressed {
            self.coils.pulse(power, FLIPPER_POWER_MS, now);
        }
        self.coils.hold(hold, pressed);
    }

    fn start_game(&mut self, now: Instant) {
        self.scores = [0; MAX_PLAYERS];
        self.replays = [0; MAX_PLAYERS];
        self.bonus_step = 0;
        self.ball = 1;
        self.players = 1;
        self.player = 0;
        self.enter(Phase::Playing);
        self.lamps.set(lamp::BONUS_WHEEL[0], LampMode::On);
        self.serve_at = Some(now);
    }

    /// Passes play to the next player, and to the next ball after the last
    /// player. A tilted ball ends here too, so a tilt only costs the tilting
    /// player their ball ("Multi-Tilt").
    fn end_ball(&mut self, now: Instant) {
        if self.player + 1 < self.players {
            self.player += 1;
        } else {
            self.player = 0;
            self.ball += 1;
        }
        if self.ball > self.settings.balls_per_game {
            self.enter(Phase::Attract);
        } else {
            self.enter(Phase::Playing);
            self.serve_at = Some(now.plus(SERVE_DELAY_MS));
        }
    }

    /// Player-up and ball-in-play lamps on the backboard.
    fn update_backboard_lamps(&mut self) {
        let playing = !matches!(self.phase, Phase::Attract);
        for (n, &lamp) in lamp::PLAYER_UP.iter().enumerate() {
            let on = playing && n == usize::from(self.player);
            self.lamps
                .set(lamp, if on { LampMode::On } else { LampMode::Off });
        }
        for (n, &lamp) in lamp::BALL_IN_PLAY.iter().enumerate() {
            let on = playing && n + 1 == usize::from(self.ball);
            self.lamps
                .set(lamp, if on { LampMode::On } else { LampMode::Off });
        }
    }

    fn enter(&mut self, phase: Phase) {
        self.phase = phase;
        match phase {
            Phase::Attract => {
                self.ball = 0;
                self.player = 0;
                self.serve_at = None;
                self.coils.set_enabled(false);
                self.lamps.all_off();
                self.lamps.set(lamp::GAME_OVER, LampMode::On);
            }
            Phase::Playing => {
                // TODO: decide which lamps (e.g. rollovers) reset per ball.
                self.tilt_warnings = 0;
                self.coils.set_enabled(true);
                self.close_trapdoors();
                self.lamps.set(lamp::GAME_OVER, LampMode::Off);
                self.lamps.set(lamp::TILT, LampMode::Off);
                for pop in lamp::POP_BUMPERS {
                    self.lamps.set(pop, LampMode::On);
                }
            }
            Phase::Tilted => {
                // Cut everything (including held flippers), then re-enable so
                // the ball can still be served once this one drains.
                self.coils.set_enabled(false);
                self.coils.set_enabled(true);
                self.lamps.set(lamp::TILT, LampMode::On);
            }
        }
        self.update_backboard_lamps();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Drives a [`Machine`] with simulated switches and a 1 ms clock.
    struct Sim {
        machine: Machine,
        switches: SwitchStates,
        now: u32,
        last: Outputs,
    }

    impl Sim {
        fn new() -> Self {
            Self {
                machine: Machine::new(),
                switches: SwitchStates::new(),
                now: 0,
                last: Outputs::default(),
            }
        }

        fn run(&mut self, ms: u32) {
            for _ in 0..ms {
                self.now += 1;
                self.last = self
                    .machine
                    .tick(Instant::from_millis(self.now), &self.switches);
            }
        }

        fn set(&mut self, id: SwitchId, closed: bool) {
            self.switches.set(id, closed);
            self.run(5);
        }

        fn tap(&mut self, id: SwitchId) {
            self.set(id, true);
            self.set(id, false);
        }

        fn coil_on(&self, id: CoilId) -> bool {
            self.last.coils & (1 << id.index()) != 0
        }
    }

    #[test]
    fn coils_are_dead_in_attract_mode() {
        let mut sim = Sim::new();
        sim.set(sw::LEFT_FLIPPER_BUTTON, true);
        assert_eq!(sim.last.coils, 0);
    }

    #[test]
    fn start_button_starts_a_game_and_serves_a_ball() {
        let mut sim = Sim::new();
        sim.set(sw::START_BUTTON, true);
        assert_eq!(sim.machine.phase(), Phase::Playing);
        assert_eq!(sim.machine.ball(), 1);
        assert!(sim.coil_on(coil::BALL_SERVE));
    }

    #[test]
    fn flipper_pulses_power_then_holds() {
        let mut sim = Sim::new();
        sim.tap(sw::START_BUTTON);
        sim.set(sw::LEFT_FLIPPER_BUTTON, true);
        assert!(sim.coil_on(coil::LEFT_FLIPPER_POWER));
        assert!(sim.coil_on(coil::LEFT_FLIPPER_HOLD));

        sim.run(100);
        assert!(!sim.coil_on(coil::LEFT_FLIPPER_POWER));
        assert!(sim.coil_on(coil::LEFT_FLIPPER_HOLD));

        sim.set(sw::LEFT_FLIPPER_BUTTON, false);
        assert!(!sim.coil_on(coil::LEFT_FLIPPER_HOLD));
    }

    #[test]
    fn game_ends_after_last_ball_drains() {
        let mut sim = Sim::new();
        sim.tap(sw::START_BUTTON);
        for ball in 1..=Settings::DEFAULT.balls_per_game {
            assert_eq!(sim.machine.ball(), ball);
            sim.tap(sw::OUTHOLE);
            sim.run(SERVE_DELAY_MS);
        }
        assert_eq!(sim.machine.phase(), Phase::Attract);
    }

    #[test]
    fn tilt_kills_flippers_until_ball_drains() {
        let mut sim = Sim::new();
        sim.tap(sw::START_BUTTON);
        for _ in 0..=TILT_WARNINGS {
            sim.tap(sw::TILT);
        }
        assert_eq!(sim.machine.phase(), Phase::Tilted);
        sim.set(sw::RIGHT_FLIPPER_BUTTON, true);
        assert!(!sim.coil_on(coil::RIGHT_FLIPPER_HOLD));
        sim.set(sw::RIGHT_FLIPPER_BUTTON, false);

        sim.tap(sw::OUTHOLE);
        assert_eq!(sim.machine.phase(), Phase::Playing);
        assert_eq!(sim.machine.ball(), 2);
    }

    #[test]
    fn lanes_advance_the_bonus_wheel_and_the_gate_collects_it() {
        let mut sim = Sim::new();
        sim.tap(sw::START_BUTTON);
        assert_eq!(sim.machine.bonus(), 10);

        sim.tap(sw::SIDE_LANES[0]);
        sim.tap(sw::TOP_LANES[1]);
        assert_eq!(sim.machine.bonus(), 30);
        assert_eq!(sim.machine.score(), 20);

        sim.tap(sw::BONUS_GATE);
        assert_eq!(sim.machine.score(), 50);
    }

    #[test]
    fn full_bonus_wheel_lights_special_for_one_replay() {
        let mut sim = Sim::new();
        sim.tap(sw::START_BUTTON);
        for _ in 0..12 {
            sim.tap(sw::SIDE_LANES[2]);
        }
        assert_eq!(sim.machine.bonus(), 100);
        sim.tap(sw::BONUS_GATE);
        sim.tap(sw::BONUS_GATE);
        // 120 lane points + 2 × 100 bonus = 320: no score replay yet.
        assert_eq!(sim.machine.score(), 320);
        assert_eq!(sim.machine.replays(), 1);
    }

    #[test]
    fn replays_are_awarded_at_card_thresholds() {
        let mut sim = Sim::new();
        sim.tap(sw::START_BUTTON);
        for _ in 0..40 {
            sim.tap(sw::SIDE_LANES[0]);
        }
        // 400 lane points crosses the first threshold.
        assert_eq!(sim.machine.replays(), 1);
        for _ in 0..3 {
            sim.tap(sw::BONUS_GATE);
        }
        // + 3 × 100 = 700: crosses 600 and 700.
        assert_eq!(sim.machine.score(), 700);
        assert_eq!(sim.machine.replays(), 4);
    }

    #[test]
    fn trapdoor_flap_stays_open_until_a_ball_drops_through() {
        let mut sim = Sim::new();
        sim.tap(sw::START_BUTTON);
        sim.machine.set_trapdoor(1, true);
        sim.run(5_000);
        assert!(sim.coil_on(coil::TRAPDOOR_FLAPS[1]));

        sim.tap(sw::TRAPDOORS[1]);
        assert!(!sim.coil_on(coil::TRAPDOOR_FLAPS[1]));
        // The ball isn't over until it reaches the outhole.
        assert_eq!(sim.machine.ball(), 1);
        sim.tap(sw::OUTHOLE);
        assert_eq!(sim.machine.ball(), 2);
    }

    #[test]
    fn trapdoors_close_on_tilt_and_stay_shut() {
        let mut sim = Sim::new();
        sim.tap(sw::START_BUTTON);
        sim.machine.set_trapdoor(0, true);
        sim.tap(sw::TILT);
        assert!(!sim.coil_on(coil::TRAPDOOR_FLAPS[0]));
        sim.machine.set_trapdoor(0, true);
        sim.run(5);
        assert!(!sim.coil_on(coil::TRAPDOOR_FLAPS[0]));
    }

    #[test]
    fn trapdoors_are_closed_for_each_new_ball() {
        let mut sim = Sim::new();
        sim.tap(sw::START_BUTTON);
        sim.machine.set_trapdoor(2, true);
        sim.tap(sw::OUTHOLE);
        assert!(!sim.coil_on(coil::TRAPDOOR_FLAPS[2]));
    }

    #[test]
    fn start_during_ball_one_adds_players_up_to_four() {
        let mut sim = Sim::new();
        sim.tap(sw::START_BUTTON);
        for _ in 0..5 {
            sim.tap(sw::START_BUTTON);
        }
        assert_eq!(sim.machine.players(), 4);
    }

    #[test]
    fn players_can_only_be_added_during_ball_one() {
        let mut sim = Sim::new();
        sim.tap(sw::START_BUTTON);
        sim.tap(sw::OUTHOLE);
        assert_eq!(sim.machine.ball(), 2);
        sim.tap(sw::START_BUTTON);
        assert_eq!(sim.machine.players(), 1);
    }

    #[test]
    fn players_take_turns_and_keep_their_own_scores() {
        let mut sim = Sim::new();
        sim.tap(sw::START_BUTTON);
        sim.tap(sw::START_BUTTON);
        assert_eq!(sim.machine.current_player(), 1);
        sim.tap(sw::SIDE_LANES[0]);

        sim.tap(sw::OUTHOLE);
        assert_eq!((sim.machine.current_player(), sim.machine.ball()), (2, 1));
        sim.tap(sw::SIDE_LANES[0]);
        sim.tap(sw::SIDE_LANES[0]);

        sim.tap(sw::OUTHOLE);
        assert_eq!((sim.machine.current_player(), sim.machine.ball()), (1, 2));
        assert_eq!(sim.machine.score_of(0), 10);
        assert_eq!(sim.machine.score_of(1), 20);
    }

    #[test]
    fn game_lasts_the_set_number_of_balls_for_every_player() {
        let settings = Settings {
            balls_per_game: 3,
            replays: true,
        };
        let mut sim = Sim::new();
        sim.machine = Machine::with_settings(settings);
        sim.tap(sw::START_BUTTON);
        sim.tap(sw::START_BUTTON);
        for _ in 0..(3 * 2 - 1) {
            sim.tap(sw::OUTHOLE);
            assert_eq!(sim.machine.phase(), Phase::Playing);
        }
        sim.tap(sw::OUTHOLE);
        assert_eq!(sim.machine.phase(), Phase::Attract);
    }

    #[test]
    fn balls_per_game_is_clamped_to_three_to_five() {
        let settings = |balls_per_game| Settings {
            balls_per_game,
            replays: true,
        };
        assert_eq!(
            Machine::with_settings(settings(1))
                .settings()
                .balls_per_game,
            3
        );
        assert_eq!(
            Machine::with_settings(settings(9))
                .settings()
                .balls_per_game,
            5
        );
    }

    #[test]
    fn a_tilt_only_costs_the_tilting_player_their_ball() {
        let mut sim = Sim::new();
        sim.tap(sw::START_BUTTON);
        sim.tap(sw::START_BUTTON);
        sim.tap(sw::TILT);
        assert_eq!(sim.machine.phase(), Phase::Tilted);
        sim.tap(sw::OUTHOLE);
        assert_eq!(sim.machine.phase(), Phase::Playing);
        assert_eq!((sim.machine.current_player(), sim.machine.ball()), (2, 1));
    }

    #[test]
    fn novelty_mode_awards_no_replays() {
        let mut sim = Sim::new();
        sim.machine = Machine::with_settings(Settings {
            balls_per_game: 5,
            replays: false,
        });
        sim.tap(sw::START_BUTTON);
        for _ in 0..40 {
            sim.tap(sw::SIDE_LANES[0]);
        }
        assert_eq!(sim.machine.score(), 400);
        assert_eq!(sim.machine.replays(), 0);
    }

    #[test]
    fn replays_add_credits_that_last_between_games() {
        let mut sim = Sim::new();
        sim.tap(sw::START_BUTTON);
        for _ in 0..40 {
            sim.tap(sw::SIDE_LANES[0]);
        }
        assert_eq!(sim.machine.credits(), 1);
        sim.tap(sw::SLAM_TILT);
        sim.set(sw::SLAM_TILT, false);
        sim.tap(sw::START_BUTTON);
        assert_eq!(sim.machine.credits(), 1);
        assert_eq!(sim.machine.score(), 0);
    }

    #[test]
    fn backboard_lamps_show_player_up_and_ball_in_play() {
        let lit = |sim: &Sim, id: crate::lamps::LampId| sim.last.lamps & (1 << id.index()) != 0;
        let mut sim = Sim::new();
        sim.tap(sw::START_BUTTON);
        sim.tap(sw::START_BUTTON);
        assert!(lit(&sim, lamp::PLAYER_UP[0]) && !lit(&sim, lamp::PLAYER_UP[1]));
        assert!(lit(&sim, lamp::BALL_IN_PLAY[0]));

        sim.tap(sw::OUTHOLE);
        assert!(!lit(&sim, lamp::PLAYER_UP[0]) && lit(&sim, lamp::PLAYER_UP[1]));
        sim.tap(sw::OUTHOLE);
        assert!(lit(&sim, lamp::BALL_IN_PLAY[1]) && !lit(&sim, lamp::BALL_IN_PLAY[0]));

        sim.tap(sw::SLAM_TILT);
        assert!(
            lamp::PLAYER_UP
                .iter()
                .chain(&lamp::BALL_IN_PLAY)
                .all(|&l| !lit(&sim, l))
        );
    }

    #[test]
    fn slam_tilt_ends_the_game() {
        let mut sim = Sim::new();
        sim.tap(sw::START_BUTTON);
        sim.set(sw::LEFT_FLIPPER_BUTTON, true);
        sim.set(sw::SLAM_TILT, true);
        assert_eq!(sim.machine.phase(), Phase::Attract);
        assert_eq!(sim.last.coils, 0);
    }
}
