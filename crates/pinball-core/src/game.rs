//! The game rules: a state machine driven by switch events and time.
//!
//! Scores follow the original's 1950s scale (the first replay at 500 points).
//! Point values printed on the playfield are used where visible; the rest, and
//! the bonus-wheel rules, are assumptions to check against the schematic.
//!
//! From the instruction card on the apron: replays at 500, 600, 700, 800 and
//! 900; "1 Replay for ball through center trap when special lite is lit"; and
//! "Any player that tilts game loses score of ball in play, but can continue
//! in his regular turn when more than one are playing."
//!
//! From the flyer: "1 to 4 can play", "adjustable to 3-4-or 5 ball play",
//! "Novelty or Replay", and a "Multi-Tilt feature [that] permits player to
//! continue playing in turn if one player tilts game".

use crate::attract::AttractShow;
use crate::bells::BellRinger;
use crate::coils::{CoilId, Coils};
use crate::flaps::{FLAP_COUNT, FlapTrim};
use crate::lamps::{LampId, LampMode, Lamps};
use crate::playfield::{COIL_KINDS, PARTS, coil, lamp, sw};
use crate::sound::{Sound, SoundScheduler, Sounds};
use crate::switches::{Debouncer, SwitchEvent, SwitchId, SwitchStates};
use crate::test_mode::{self, HOLD_TEST_MS, TestReport, TestReports, TestStage};
use crate::time::Instant;

/// Most players in one game ("1 to 4 can play").
pub const MAX_PLAYERS: usize = 4;
/// Most credits the machine holds. From the free-play card on the owner's
/// apron, "Maximum Credits - 30" (*verify*: that card is modern, so it may
/// come from a later conversion rather than the original credit unit).
pub const MAX_CREDITS: u8 = 30;
/// Tilt warnings allowed before the ball is tilted. Classic EM rules: none.
pub const TILT_WARNINGS: u8 = 0;
/// Consecutive matching scans before a switch change is accepted.
pub const DEBOUNCE_SCANS: u8 = 2;
/// Delay between a ball draining and the next one being served, giving the
/// ball time to settle in the outhole.
pub const SERVE_DELAY_MS: u32 = 1_000;
/// Serves of one ball before giving up on it. A serve that falls short drops
/// the ball back onto the outhole switch before it has reached the shooter
/// lane, and is tried again; after this many, the ball ends as if it had
/// drained, so a jammed ball lift can't fire its coil for ever.
pub const MAX_SERVES: u8 = 3;

/// From the instruction card: one replay for each score of 500, 600, 700,
/// 800 and 900 points.
pub const REPLAY_SCORES: [u32; 5] = [500, 600, 700, 800, 900];

/// The centre trapdoor, the only one that awards the special ("1 Replay for
/// ball through center trap when special lite is lit").
const CENTRE_TRAPDOOR: usize = 1;

const FLIPPER_POWER_MS: u16 = 40;

/// How long attract mode waits before the music starts (after the game-over
/// sound). It then repeats for as long as the machine is in attract mode.
pub const MUSIC_DELAY_MS: u32 = 5_000;

/// Operator settings, set inside the cabinet on the original.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Settings {
    /// Balls per player: 3, 4 or 5 ("adjustable to 3-4-or 5 ball play").
    pub balls_per_game: u8,
    /// Replay mode awards replays; novelty mode only scores points.
    pub replays: bool,
    /// Real bells are fitted on coils 15–17 (see [`crate::bells`]). Scoring then
    /// rings them, and the WAV Trigger skips its recorded chimes.
    pub bells: bool,
    /// Background music in attract mode, with the lamps dancing to it.
    pub attract_music: bool,
}

impl Settings {
    /// As the flyer advertises it: 5-ball, replay.
    pub const DEFAULT: Self = Self {
        balls_per_game: 5,
        replays: true,
        bells: false,
        attract_music: true,
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
    /// Operator test mode (see [`crate::test_mode`]).
    Test(TestStage),
}

/// Everything the firmware must drive for this tick.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Outputs {
    /// Coil driver bitmap; bit `n` energises coil `n`.
    pub coils: u32,
    /// Lamp bitmap; bit `n` lights lamp `n`.
    pub lamps: u64,
    /// The sounds to start now (several can play at once).
    pub sounds: Sounds,
    /// The sounds to stop now, such as the attract music when a game starts.
    pub stop_sounds: Sounds,
    /// In test mode, what just happened, for the firmware to print.
    pub reports: TestReports,
    /// Where each trapdoor flap's servo should point, in degrees.
    pub flap_angles: [u8; FLAP_COUNT],
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
    /// The current player's score when this ball was served, which a tilt
    /// takes them back to.
    ball_start_score: u32,
    /// Each player's highest score this game. Replays are paid against this,
    /// so a threshold crossed again after a tilt doesn't pay twice.
    best_scores: [u32; MAX_PLAYERS],
    replays: [u8; MAX_PLAYERS],
    /// Credits on the backboard's credit window (in the clown's nose).
    credits: u8,
    /// Lit position on the bonus wheel: 0 = 10 points ... 9 = 100 points.
    /// Assumed (*verify*): shared by all players, and not reset between balls.
    bonus_step: u8,
    tilt_warnings: u8,
    serve_at: Option<Instant>,
    /// Serves of the current ball since it was last seen in play (in the
    /// shooter lane or hitting anything): 0 once it has been.
    serves: u8,
    sound: SoundScheduler,
    bells: BellRinger,
    show: AttractShow,
    /// When the attract music starts; set when attract mode is first ticked.
    music_at: Option<Instant>,
    /// Test mode's chosen coil.
    test_coil: CoilId,
    /// Test mode's chosen lamp, or `None` while every lamp is on.
    test_lamp: Option<LampId>,
    /// Test reports waiting for the next [`Outputs`].
    reports: TestReports,
    /// Which trapdoor flaps are open: bit `n` is flap `n`.
    flaps_open: u8,
    /// Each flap's closed and open servo angles.
    flap_trim: FlapTrim,
    /// Test mode's chosen flap, and whether it's at its open position.
    test_flap: (usize, bool),
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
            ball_start_score: 0,
            best_scores: [0; MAX_PLAYERS],
            replays: [0; MAX_PLAYERS],
            credits: 0,
            bonus_step: 0,
            tilt_warnings: 0,
            serve_at: None,
            serves: 0,
            sound: SoundScheduler::new(),
            bells: BellRinger::new(),
            show: AttractShow::new(),
            music_at: None,
            test_coil: test_mode::first_coil(),
            test_lamp: None,
            reports: TestReports::new(),
            flaps_open: 0,
            flap_trim: FlapTrim::DEFAULT,
            test_flap: (0, false),
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
            self.serves = self.serves.saturating_add(1);
            self.coils.pulse(coil::BALL_SERVE, u16::MAX, now);
            self.sound.request(Sound::BallServe);
        }
        self.attract_music(now);
        self.bells.tick(now, &mut self.coils);
        Outputs {
            coils: self.coils.outputs(now),
            lamps: self.lamps.frame(now),
            sounds: self.sound.next(now),
            stop_sounds: self.sound.take_stops(),
            reports: core::mem::take(&mut self.reports),
            flap_angles: self.flap_angles(),
        }
    }

    /// Where each flap's servo should point: its open or closed angle, or in
    /// test mode's flaps stage, the position being set.
    fn flap_angles(&self) -> [u8; FLAP_COUNT] {
        let mut angles = [0; FLAP_COUNT];
        for (n, angle) in angles.iter_mut().enumerate() {
            let open = if self.phase == Phase::Test(TestStage::Flaps) {
                self.test_flap == (n, true)
            } else {
                self.trapdoor_open(n)
            };
            *angle = self.flap_trim.angle(n, open);
        }
        angles
    }

    /// Whether trapdoor `n` (0–2, left to right) is open.
    #[must_use]
    pub const fn trapdoor_open(&self, n: usize) -> bool {
        n < FLAP_COUNT && self.flaps_open & (1 << n) != 0
    }

    /// Each flap's closed and open servo angles.
    #[must_use]
    pub const fn flap_trim(&self) -> FlapTrim {
        self.flap_trim
    }

    /// Sets each flap's closed and open servo angles, as saved in the EEPROM.
    pub fn set_flap_trim(&mut self, trim: FlapTrim) {
        self.flap_trim = trim;
    }

    fn on_switch(&mut self, event: SwitchEvent, now: Instant) {
        match (self.phase, event) {
            // Test mode handles every switch itself, the slam tilt included.
            (Phase::Test(stage), event) => self.on_test_switch(stage, event, now),

            (_, SwitchEvent::Closed(sw::SLAM_TILT)) => {
                let was_playing = self.phase != Phase::Attract;
                self.enter(Phase::Attract);
                if was_playing {
                    self.sound.request(Sound::GameOver);
                }
            }

            (Phase::Attract, SwitchEvent::Closed(sw::TEST_BUTTON)) => {
                self.stop_show();
                self.enter(Phase::Test(TestStage::Switches));
            }
            (Phase::Attract, SwitchEvent::Closed(sw::START_BUTTON)) => self.start_game(now),
            // Assumed (*verify*): players can be added while ball 1 is in play.
            (Phase::Playing | Phase::Tilted, SwitchEvent::Closed(sw::START_BUTTON)) => {
                if self.ball == 1 && usize::from(self.players) < MAX_PLAYERS {
                    self.players += 1;
                    self.sound.request(Sound::PlayerAdded);
                }
            }

            // A serve that fell short, before the ball reached the shooter
            // lane: serve it again.
            (Phase::Playing, SwitchEvent::Closed(sw::OUTHOLE))
                if self.serves > 0 && self.serves < MAX_SERVES =>
            {
                self.serve_at = Some(now.plus(SERVE_DELAY_MS));
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
        // Anything but the cabinet's own switches means the ball is in play.
        if !matches!(
            id,
            sw::LEFT_FLIPPER_BUTTON | sw::RIGHT_FLIPPER_BUTTON | sw::TILT
        ) {
            self.serves = 0;
        }
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
            // Assumed (*verify*): 1 point, like the kickers above the flippers.
            sw::LEFT_SIDE_KICKER => {
                kick(&mut self.coils, coil::LEFT_SIDE_KICKER);
                self.add_score(1);
            }
            sw::RIGHT_SIDE_KICKER => {
                kick(&mut self.coils, coil::RIGHT_SIDE_KICKER);
                self.add_score(1);
            }
            // The jack-in-the-box kickers in the top corners. Assumed
            // (*verify*): 1 point.
            sw::LEFT_TOP_KICKER => {
                kick(&mut self.coils, coil::LEFT_TOP_KICKER);
                self.add_score(1);
            }
            sw::RIGHT_TOP_KICKER => {
                kick(&mut self.coils, coil::RIGHT_TOP_KICKER);
                self.add_score(1);
            }
            sw::TOP_BUMPER => {
                kick(&mut self.coils, coil::TOP_BUMPER);
                self.sound.request(Sound::PopBumper);
                self.add_score(10);
            }
            // The box "Scores Bonus". Assumed (*verify*): the ball then drops
            // under the playfield and rolls to the outhole, which ends the
            // ball as usual.
            id if sw::TRAPDOORS.contains(&id) => {
                // A ball can't drop through a closed flap, so ignore a noisy
                // switch. Each trapdoor's lamp is lit exactly while it's open.
                let n = sw::TRAPDOORS.iter().position(|&t| t == id).unwrap_or(0);
                if !self.lit(lamp::TRAPDOORS[n]) {
                    return;
                }
                self.close_trapdoors();
                self.sound.request(Sound::Trapdoor);
                self.collect_bonus(n == CENTRE_TRAPDOOR);
            }
            _ => {
                if let Some(n) = sw::POP_BUMPERS.iter().position(|&pop| pop == id) {
                    kick(&mut self.coils, coil::POP_BUMPERS[n]);
                    self.sound.request(Sound::PopBumper);
                    self.add_score(1);
                    // The lower two are marked "advance bonus when lit".
                    // Assumed (*verify*): "lit" means the bumper's own cap.
                    if n >= 3 && self.lit(lamp::POP_BUMPERS[n]) {
                        self.advance_bonus();
                    }
                } else if sw::TOP_LANES.contains(&id) {
                    self.add_score(10);
                    self.chime(Sound::Lane);
                } else if let Some(n) = sw::TOP_REBOUNDS.iter().position(|&r| r == id) {
                    // Assumed (*verify*): 1 point.
                    self.add_score(1);
                    let advance = self.lit(lamp::TOP_ADVANCE_BONUS[n]);
                    self.lane_bells(advance, false);
                } else if let Some(n) = sw::SIDE_LANES.iter().position(|&l| l == id) {
                    // TODO: find out what lights the "30 points" lamps (#23).
                    let thirty = self.lit(lamp::THIRTY_POINTS[n]);
                    self.add_score(if thirty { 30 } else { 10 });
                    self.lane_bells(false, thirty);
                }
            }
        }
    }

    /// Opens or closes trapdoor `n` (0–2, left to right) in the "Scores Bonus"
    /// box, and lights its lamp while it's open. Only works while a ball is in
    /// play.
    ///
    /// TODO: call this from the rules once we know what opens the flaps on
    /// the original.
    ///
    /// # Panics
    ///
    /// If `n` is not 0, 1 or 2.
    pub fn set_trapdoor(&mut self, n: usize, open: bool) {
        if self.phase == Phase::Playing {
            if open {
                self.flaps_open |= 1 << n;
            } else {
                self.flaps_open &= !(1 << n);
            }
            let mode = if open { LampMode::On } else { LampMode::Off };
            self.lamps.set(lamp::TRAPDOORS[n], mode);
        }
    }

    fn close_trapdoors(&mut self) {
        self.flaps_open = 0;
        for lamp in lamp::TRAPDOORS {
            self.lamps.set(lamp, LampMode::Off);
        }
    }

    fn lit(&self, lamp: LampId) -> bool {
        self.lamps.mode(lamp) != LampMode::Off
    }

    /// One bell per rollover: the wheel's climbing bell if `advance` moved
    /// it, else the "30 points" fanfare or the lane bell.
    fn lane_bells(&mut self, advance: bool, thirty: bool) {
        if advance && self.advance_bonus() {
            return;
        }
        if thirty {
            self.sound.request(Sound::ThirtyPoints);
        } else {
            self.chime(Sound::Lane);
        }
    }

    /// A chime sound, unless real bells are fitted: they ring instead.
    fn chime(&mut self, sound: Sound) {
        if !self.settings.bells {
            self.sound.request(sound);
        }
    }

    /// Adds to the current player's score, awarding replays at the card's
    /// thresholds, and ringing the bells if they're fitted.
    fn add_score(&mut self, points: u32) {
        if self.settings.bells {
            self.bells.score(points);
        }
        let player = usize::from(self.player);
        let before = self.best_scores[player];
        self.scores[player] += points;
        let after = self.scores[player];
        self.best_scores[player] = before.max(after);
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
            self.sound.request(Sound::Replay);
            self.replays[usize::from(self.player)] += 1;
            self.credits = (self.credits + 1).min(MAX_CREDITS);
        }
    }

    /// Assumed rule: a rollover whose "advance bonus" lamp is lit moves the
    /// bonus wheel on one step; reaching 100 lights the special. Returns
    /// whether the wheel moved.
    fn advance_bonus(&mut self) -> bool {
        let advanced = self.bonus_step < 9;
        if advanced {
            self.lamps.set(
                lamp::BONUS_WHEEL[usize::from(self.bonus_step)],
                LampMode::Off,
            );
            self.bonus_step += 1;
            self.lamps.set(
                lamp::BONUS_WHEEL[usize::from(self.bonus_step)],
                LampMode::On,
            );
            self.chime(Sound::BonusAdvance(self.bonus_step));
            if self.bonus_step == 9 {
                self.sound.request(Sound::Special);
            }
        }
        if self.bonus_step == 9 {
            self.lamps.set(lamp::SPECIAL, LampMode::On);
        }
        advanced
    }

    /// A ball through a trapdoor in the "Scores Bonus" box scores the lit
    /// wheel value (assumed: the card doesn't give the value). Through the
    /// centre trapdoor, it also awards a replay when the special is lit, and
    /// the special goes out (assumed: once per lighting).
    fn collect_bonus(&mut self, centre: bool) {
        self.sound.request(Sound::BonusCollect);
        self.add_score(self.bonus());
        if centre && self.lamps.mode(lamp::SPECIAL) != LampMode::Off {
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

    /// Plays the attract music, repeating it from the top, and runs the light
    /// show to it.
    fn attract_music(&mut self, now: Instant) {
        if self.phase == Phase::Attract && self.settings.attract_music {
            let at = *self.music_at.get_or_insert(now.plus(MUSIC_DELAY_MS));
            if !self.show.is_running() && now.reached(at) {
                self.sound.request(Sound::AttractMusic);
                self.show.start(now);
            }
        }
        if self.show.tick(now, &mut self.lamps) {
            // The music ended: start it again, in step with the show.
            self.sound.request(Sound::AttractMusic);
        }
    }

    /// Stops the attract light show and its music, if they're running.
    fn stop_show(&mut self) {
        if self.show.is_running() {
            self.show.stop(&mut self.lamps);
            self.sound.stop(Sound::AttractMusic);
        }
    }

    /// Test mode: see [`crate::test_mode`] for what each stage does.
    fn on_test_switch(&mut self, stage: TestStage, event: SwitchEvent, now: Instant) {
        let (id, closed) = match event {
            SwitchEvent::Closed(id) => (id, true),
            SwitchEvent::Opened(id) => (id, false),
        };
        if closed && id == sw::TEST_BUTTON {
            if let Some(next) = stage.next() {
                self.enter(Phase::Test(next));
                return;
            }
        }
        // The test button after the last stage, or the slam tilt, leaves.
        if closed && (id == sw::TEST_BUTTON || id == sw::SLAM_TILT) {
            self.enter(Phase::Attract);
            self.reports.push(TestReport::Done);
            return;
        }
        match stage {
            TestStage::Switches => {
                if id == sw::TEST_BUTTON {
                    return;
                }
                if closed {
                    self.sound.request(Sound::Lane);
                }
                // Light the part's own lamp while its switch is held.
                if let Some(lamp) = test_mode::part_with_switch(id).and_then(|p| p.lamp) {
                    let mode = if closed { LampMode::On } else { LampMode::Off };
                    self.lamps.set(lamp, mode);
                }
                self.reports.push(TestReport::Switch { id, closed });
            }
            TestStage::Coils if closed => match id {
                sw::LEFT_FLIPPER_BUTTON | sw::RIGHT_FLIPPER_BUTTON => {
                    let forward = id == sw::RIGHT_FLIPPER_BUTTON;
                    self.choose_test_coil(test_mode::step_coil(self.test_coil, forward));
                }
                sw::START_BUTTON => {
                    // Pulse coils are clamped to their own limit.
                    let fired = self.coils.pulse(self.test_coil, HOLD_TEST_MS, now);
                    self.reports.push(TestReport::Coil {
                        id: self.test_coil,
                        fired: Some(fired),
                    });
                }
                _ => {}
            },
            TestStage::Lamps if closed => match id {
                sw::LEFT_FLIPPER_BUTTON | sw::RIGHT_FLIPPER_BUTTON => {
                    let forward = id == sw::RIGHT_FLIPPER_BUTTON;
                    let from = self.test_lamp.unwrap_or_else(test_mode::first_lamp);
                    self.choose_test_lamp(test_mode::step_lamp(from, forward));
                }
                sw::START_BUTTON => {
                    let all_on = self.test_lamp.is_some();
                    self.reports.push(TestReport::AllLamps(all_on));
                    if all_on {
                        for lamp in PARTS.iter().filter_map(|p| p.lamp) {
                            self.lamps.set(lamp, LampMode::On);
                        }
                        self.test_lamp = None;
                    } else {
                        self.choose_test_lamp(test_mode::first_lamp());
                    }
                }
                _ => {}
            },
            TestStage::Flaps if closed => match id {
                sw::LEFT_FLIPPER_BUTTON | sw::RIGHT_FLIPPER_BUTTON => {
                    let (n, open) = self.test_flap;
                    let step = if id == sw::RIGHT_FLIPPER_BUTTON {
                        1
                    } else {
                        -1
                    };
                    self.flap_trim.nudge(n, open, step);
                    self.report_test_flap();
                }
                sw::START_BUTTON => {
                    // Flap 1 closed, flap 1 open, flap 2 closed, and so on.
                    let (n, open) = self.test_flap;
                    self.test_flap = if open {
                        ((n + 1) % FLAP_COUNT, false)
                    } else {
                        (n, true)
                    };
                    self.report_test_flap();
                }
                _ => {}
            },
            TestStage::Coils | TestStage::Flaps | TestStage::Lamps => {}
        }
    }

    /// Chooses the coil for test mode, lighting its part's lamp if it has one.
    fn choose_test_coil(&mut self, id: CoilId) {
        self.lamps.all_off();
        if let Some(lamp) = test_mode::part_with_coil(id).and_then(|p| p.lamp) {
            self.lamps.set(lamp, LampMode::On);
        }
        self.test_coil = id;
        self.reports.push(TestReport::Coil { id, fired: None });
    }

    fn report_test_flap(&mut self) {
        let (n, open) = self.test_flap;
        self.reports.push(TestReport::Flap {
            n: u8::try_from(n).unwrap_or(0),
            open,
            angle: self.flap_trim.angle(n, open),
        });
    }

    /// Lights one lamp on its own for test mode.
    fn choose_test_lamp(&mut self, id: LampId) {
        self.lamps.all_off();
        self.lamps.set(id, LampMode::On);
        self.test_lamp = Some(id);
        self.reports.push(TestReport::Lamp(id));
    }

    fn start_game(&mut self, now: Instant) {
        self.stop_show();
        self.scores = [0; MAX_PLAYERS];
        self.replays = [0; MAX_PLAYERS];
        self.best_scores = [0; MAX_PLAYERS];
        self.bonus_step = 0;
        self.serves = 0;
        self.ball = 1;
        self.players = 1;
        self.player = 0;
        self.enter(Phase::Playing);
        self.sound.request(Sound::GameStart);
        self.lamps.set(lamp::BONUS_WHEEL[0], LampMode::On);
        self.serve_at = Some(now);
    }

    /// Passes play to the next player, and to the next ball after the last
    /// player. A tilted ball ends here too, so a tilt only costs the tilting
    /// player their ball and its score ("Multi-Tilt"). Assumed (*verify*):
    /// a lone player carries on after a tilt too; the card only promises it
    /// "when more than one are playing".
    fn end_ball(&mut self, now: Instant) {
        self.serves = 0;
        if self.player + 1 < self.players {
            self.player += 1;
        } else {
            self.player = 0;
            self.ball += 1;
        }
        if self.ball > self.settings.balls_per_game {
            self.enter(Phase::Attract);
            self.sound.request(Sound::GameOver);
        } else {
            self.enter(Phase::Playing);
            self.sound.request(Sound::Drain);
            self.serve_at = Some(now.plus(SERVE_DELAY_MS));
        }
    }

    /// Player-up and ball-in-play lamps on the backboard.
    fn update_backboard_lamps(&mut self) {
        let playing = matches!(self.phase, Phase::Playing | Phase::Tilted);
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
                self.bells.clear();
                self.music_at = None;
                self.ball = 0;
                self.player = 0;
                self.serve_at = None;
                self.coils.set_enabled(false);
                self.flaps_open = 0;
                self.lamps.all_off();
                self.lamps.set(lamp::GAME_OVER, LampMode::On);
            }
            Phase::Playing => {
                // TODO: decide which lamps (e.g. rollovers) reset per ball.
                self.tilt_warnings = 0;
                self.ball_start_score = self.scores[usize::from(self.player)];
                self.coils.set_enabled(true);
                self.close_trapdoors();
                self.lamps.set(lamp::GAME_OVER, LampMode::Off);
                self.lamps.set(lamp::TILT, LampMode::Off);
                for pop in lamp::POP_BUMPERS {
                    self.lamps.set(pop, LampMode::On);
                }
                // Assumed until we know what lights them (#22, #25): the top
                // rubbers' "advance bonus" lamps are lit, like the pop bumper
                // caps, so they and the lower pop bumpers advance the wheel.
                // Nothing lights the "30 points" lamps yet.
                self.lamps.set(lamp::TOP_BUMPER, LampMode::On);
                for advance in lamp::TOP_ADVANCE_BONUS {
                    self.lamps.set(advance, LampMode::On);
                }
            }
            Phase::Test(stage) => {
                self.bells.clear();
                self.serve_at = None;
                self.lamps.all_off();
                // Coil power only while testing coils, and nothing fires
                // until Start is pressed.
                self.coils.set_enabled(false);
                self.flaps_open = 0;
                self.reports.push(TestReport::Stage(stage));
                match stage {
                    TestStage::Switches => {}
                    TestStage::Flaps => {
                        self.test_flap = (0, false);
                        self.report_test_flap();
                    }
                    TestStage::Coils => {
                        self.coils.set_enabled(true);
                        self.choose_test_coil(test_mode::first_coil());
                    }
                    TestStage::Lamps => {
                        self.choose_test_lamp(test_mode::first_lamp());
                    }
                }
            }
            Phase::Tilted => {
                self.bells.clear();
                // "Loses score of ball in play". Assumed (*verify*): replays
                // already won on this ball are kept.
                self.scores[usize::from(self.player)] = self.ball_start_score;
                // Cut everything (including held flippers), then re-enable so
                // the ball can still be served once this one drains.
                self.coils.set_enabled(false);
                self.coils.set_enabled(true);
                // And shut the flaps.
                self.close_trapdoors();
                self.lamps.set(lamp::TILT, LampMode::On);
                self.sound.request(Sound::Tilt);
            }
        }
        self.update_backboard_lamps();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::music_beats::ATTRACT_MUSIC_MS;

    /// Drives a [`Machine`] with simulated switches and a 1 ms clock.
    struct Sim {
        machine: Machine,
        switches: SwitchStates,
        now: u32,
        last: Outputs,
        sounds: Vec<Sound>,
        stops: Vec<Sound>,
        reports: Vec<TestReport>,
    }

    impl Sim {
        fn new() -> Self {
            Self {
                machine: Machine::new(),
                switches: SwitchStates::new(),
                now: 0,
                last: Outputs::default(),
                sounds: Vec::new(),
                stops: Vec::new(),
                reports: Vec::new(),
            }
        }

        fn run(&mut self, ms: u32) {
            for _ in 0..ms {
                self.now += 1;
                self.last = self
                    .machine
                    .tick(Instant::from_millis(self.now), &self.switches);
                self.sounds.extend(self.last.sounds.iter());
                self.stops.extend(self.last.stop_sounds.iter());
                self.reports.extend(self.last.reports.iter());
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

        /// The served ball reaches the shooter lane, then drains.
        fn drain(&mut self) {
            self.tap(sw::SHOOTER_LANE);
            self.tap(sw::OUTHOLE);
        }

        /// Opens trapdoor `n` and drops a ball through it.
        fn drop_through(&mut self, n: usize) {
            self.machine.set_trapdoor(n, true);
            self.tap(sw::TRAPDOORS[n]);
        }

        /// Whether trapdoor flap `n`'s servo is at its open angle.
        fn flap_open(&self, n: usize) -> bool {
            self.last.flap_angles[n] == self.machine.flap_trim().angle(n, true)
        }

        fn coil_on(&self, id: CoilId) -> bool {
            self.last.coils & (1 << id.index()) != 0
        }
    }

    /// Every test report made while running `f`.
    fn reports_during(sim: &mut Sim, f: impl FnOnce(&mut Sim)) -> Vec<TestReport> {
        sim.reports.clear();
        f(sim);
        std::mem::take(&mut sim.reports)
    }

    fn lamp_on(sim: &Sim, id: LampId) -> bool {
        sim.last.lamps & (1 << id.index()) != 0
    }

    #[test]
    fn the_test_button_steps_through_the_stages_and_back() {
        let mut sim = Sim::new();
        let stages = [
            TestReport::Stage(TestStage::Switches),
            TestReport::Stage(TestStage::Coils),
            TestReport::Stage(TestStage::Flaps),
            TestReport::Stage(TestStage::Lamps),
        ];
        for (n, stage) in stages.into_iter().enumerate() {
            let reports = reports_during(&mut sim, |s| s.tap(sw::TEST_BUTTON));
            assert_eq!(reports[0], stage, "press {n}");
        }
        assert_eq!(
            reports_during(&mut sim, |s| s.tap(sw::TEST_BUTTON)),
            [TestReport::Done]
        );
        assert_eq!(sim.machine.phase(), Phase::Attract);
        assert!(lamp_on(&sim, lamp::GAME_OVER));
    }

    #[test]
    fn the_test_button_does_nothing_during_a_game() {
        let mut sim = Sim::new();
        sim.tap(sw::START_BUTTON);
        sim.tap(sw::TEST_BUTTON);
        assert_eq!(sim.machine.phase(), Phase::Playing);
    }

    #[test]
    fn switch_test_reports_switches_and_lights_their_lamps() {
        let mut sim = Sim::new();
        sim.tap(sw::TEST_BUTTON);
        sim.set(sw::POP_BUMPERS[3], true);
        assert!(lamp_on(&sim, lamp::POP_BUMPERS[3]));
        // Nothing fires and nothing scores.
        assert_eq!(sim.last.coils, 0);
        let reports = reports_during(&mut sim, |s| s.set(sw::POP_BUMPERS[3], false));
        assert_eq!(
            reports,
            [TestReport::Switch {
                id: sw::POP_BUMPERS[3],
                closed: false
            }]
        );
        assert!(!lamp_on(&sim, lamp::POP_BUMPERS[3]));
        // Start and the flippers are just switches here too.
        let reports = reports_during(&mut sim, |s| s.tap(sw::START_BUTTON));
        assert_eq!(reports.len(), 2);
        assert!(matches!(sim.machine.phase(), Phase::Test(_)));
        sim.run(100);
        assert!(sounds_during(&mut sim, Sim::drain).contains(&Sound::Lane));
    }

    #[test]
    fn coil_test_fires_only_the_chosen_coil_when_start_is_pressed() {
        let mut sim = Sim::new();
        sim.tap(sw::TEST_BUTTON);
        let reports = reports_during(&mut sim, |s| s.tap(sw::TEST_BUTTON));
        assert_eq!(
            reports[1],
            TestReport::Coil {
                id: coil::BALL_SERVE,
                fired: None
            }
        );
        // The flippers choose; they don't flip.
        sim.set(sw::RIGHT_FLIPPER_BUTTON, true);
        assert_eq!(sim.last.coils, 0);
        sim.set(sw::RIGHT_FLIPPER_BUTTON, false);
        sim.tap(sw::RIGHT_FLIPPER_BUTTON);
        let reports = reports_during(&mut sim, |s| s.tap(sw::START_BUTTON));
        assert_eq!(
            reports,
            [TestReport::Coil {
                id: coil::RIGHT_KICKER,
                fired: Some(true)
            }]
        );
        assert_eq!(sim.last.coils, 1 << coil::RIGHT_KICKER.index());
        // Too soon to fire again, and it says so.
        let reports = reports_during(&mut sim, |s| s.tap(sw::START_BUTTON));
        assert_eq!(
            reports,
            [TestReport::Coil {
                id: coil::RIGHT_KICKER,
                fired: Some(false)
            }]
        );
        sim.run(200);
        assert_eq!(sim.last.coils, 0);
        // Back past the first coil wraps round to the last.
        sim.tap(sw::LEFT_FLIPPER_BUTTON);
        sim.tap(sw::LEFT_FLIPPER_BUTTON);
        let reports = reports_during(&mut sim, |s| s.tap(sw::LEFT_FLIPPER_BUTTON));
        assert_eq!(
            reports,
            [TestReport::Coil {
                id: coil::RIGHT_TOP_KICKER,
                fired: None
            }]
        );
    }

    #[test]
    fn coil_test_holds_a_holding_coil_only_briefly() {
        let mut sim = Sim::new();
        sim.tap(sw::TEST_BUTTON);
        sim.tap(sw::TEST_BUTTON);
        while !matches!(
            sim.reports.last(),
            Some(TestReport::Coil { id, .. }) if *id == coil::LEFT_FLIPPER_HOLD
        ) {
            sim.tap(sw::RIGHT_FLIPPER_BUTTON);
        }
        sim.tap(sw::START_BUTTON);
        assert!(sim.coil_on(coil::LEFT_FLIPPER_HOLD));
        sim.run(u32::from(HOLD_TEST_MS));
        assert!(!sim.coil_on(coil::LEFT_FLIPPER_HOLD));
    }

    #[test]
    fn coil_test_lights_the_chosen_parts_lamp() {
        let mut sim = Sim::new();
        sim.tap(sw::TEST_BUTTON);
        sim.tap(sw::TEST_BUTTON);
        while !matches!(
            sim.reports.last(),
            Some(TestReport::Coil { id, .. }) if *id == coil::POP_BUMPERS[0]
        ) {
            sim.tap(sw::RIGHT_FLIPPER_BUTTON);
        }
        assert_eq!(sim.last.lamps, 1 << lamp::POP_BUMPERS[0].index());
    }

    #[test]
    fn lamp_test_steps_through_lamps_and_lights_them_all() {
        let mut sim = Sim::new();
        for _ in 0..4 {
            sim.tap(sw::TEST_BUTTON);
        }
        assert_eq!(sim.last.lamps, 1 << lamp::GAME_OVER.index());
        sim.tap(sw::RIGHT_FLIPPER_BUTTON);
        assert_eq!(sim.last.lamps, 1 << lamp::TILT.index());
        assert_eq!(sim.last.coils, 0);
        assert_eq!(
            reports_during(&mut sim, |s| s.tap(sw::START_BUTTON)),
            [TestReport::AllLamps(true)]
        );
        assert_eq!(sim.last.coils, 0);
        let all: u64 = PARTS
            .iter()
            .filter_map(|p| p.lamp)
            .map(|l| 1 << l.index())
            .sum();
        assert_eq!(sim.last.lamps, all);
        sim.tap(sw::START_BUTTON);
        assert_eq!(sim.last.lamps, 1 << lamp::GAME_OVER.index());
    }

    #[test]
    fn slam_tilt_leaves_test_mode_with_the_coils_off() {
        let mut sim = Sim::new();
        sim.tap(sw::TEST_BUTTON);
        sim.tap(sw::TEST_BUTTON);
        sim.tap(sw::START_BUTTON);
        let mut reports = Vec::new();
        let sounds = sounds_during(&mut sim, |s| {
            reports = reports_during(s, |s| s.tap(sw::SLAM_TILT));
        });
        assert_eq!(reports, [TestReport::Done]);
        assert!(!sounds.contains(&Sound::GameOver), "{sounds:?}");
        assert_eq!(sim.machine.phase(), Phase::Attract);
        assert_eq!(sim.last.coils, 0);
    }

    #[test]
    fn test_mode_stops_the_attract_show() {
        let mut sim = Sim::new();
        sim.run(MUSIC_DELAY_MS + 100);
        assert!(sim.sounds.contains(&Sound::AttractMusic));
        sim.stops.clear();
        sim.tap(sw::TEST_BUTTON);
        assert_eq!(sim.stops, [Sound::AttractMusic]);
        assert_eq!(sim.last.lamps, 0, "only test mode's own lamps");
        // And it doesn't come back while testing.
        sim.sounds.clear();
        sim.run(MUSIC_DELAY_MS * 2);
        assert!(!sim.sounds.contains(&Sound::AttractMusic));
        assert!(matches!(sim.machine.phase(), Phase::Test(_)));
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
    fn a_serve_that_falls_short_is_served_again() {
        let mut sim = Sim::new();
        sim.tap(sw::START_BUTTON);
        // The ball drops back down the ball lift before reaching the lane.
        let sounds = sounds_during(&mut sim, |s| {
            s.tap(sw::OUTHOLE);
            s.run(SERVE_DELAY_MS);
        });
        assert_eq!(sim.machine.ball(), 1);
        assert_eq!(sounds, [Sound::BallServe]);

        // Served properly this time, it drains as usual.
        sim.drain();
        assert_eq!(sim.machine.ball(), 2);
    }

    #[test]
    fn a_ball_that_hits_something_is_in_play_even_without_the_lane_switch() {
        let mut sim = Sim::new();
        sim.tap(sw::START_BUTTON);
        sim.tap(sw::TOP_BUMPER);
        sim.tap(sw::OUTHOLE);
        assert_eq!(sim.machine.ball(), 2);
    }

    #[test]
    fn pressing_the_flippers_doesnt_count_as_the_ball_in_play() {
        let mut sim = Sim::new();
        sim.tap(sw::START_BUTTON);
        sim.tap(sw::LEFT_FLIPPER_BUTTON);
        sim.tap(sw::OUTHOLE);
        assert_eq!(sim.machine.ball(), 1);
    }

    #[test]
    fn a_ball_lift_that_keeps_failing_ends_the_ball() {
        let mut sim = Sim::new();
        sim.tap(sw::START_BUTTON);
        for _ in 1..MAX_SERVES {
            sim.tap(sw::OUTHOLE);
            sim.run(SERVE_DELAY_MS);
            assert_eq!(sim.machine.ball(), 1);
        }
        sim.tap(sw::OUTHOLE);
        assert_eq!(sim.machine.ball(), 2);
    }

    #[test]
    fn game_ends_after_last_ball_drains() {
        let mut sim = Sim::new();
        sim.tap(sw::START_BUTTON);
        for ball in 1..=Settings::DEFAULT.balls_per_game {
            assert_eq!(sim.machine.ball(), ball);
            sim.drain();
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

        sim.drain();
        assert_eq!(sim.machine.phase(), Phase::Playing);
        assert_eq!(sim.machine.ball(), 2);
    }

    #[test]
    fn rubbers_and_lower_pops_advance_the_wheel_and_a_trapdoor_collects_it() {
        let mut sim = Sim::new();
        sim.tap(sw::START_BUTTON);
        assert_eq!(sim.machine.bonus(), 10);

        sim.tap(sw::TOP_REBOUNDS[1]);
        sim.tap(sw::POP_BUMPERS[3]);
        assert_eq!(sim.machine.bonus(), 30);
        assert_eq!(sim.machine.score(), 2);

        sim.drop_through(0);
        assert_eq!(sim.machine.score(), 32);
    }

    #[test]
    fn lanes_and_upper_pops_score_without_advancing_the_wheel() {
        let mut sim = Sim::new();
        sim.tap(sw::START_BUTTON);
        sim.tap(sw::TOP_LANES[0]);
        sim.tap(sw::SIDE_LANES[1]);
        sim.tap(sw::POP_BUMPERS[1]);
        assert_eq!(sim.machine.score(), 21);
        assert_eq!(sim.machine.bonus(), 10);
    }

    #[test]
    fn only_a_lit_advance_bonus_lamp_moves_the_wheel() {
        let mut sim = Sim::new();
        sim.tap(sw::START_BUTTON);
        sim.machine.lamps.set(lamp::POP_BUMPERS[4], LampMode::Off);
        sim.machine
            .lamps
            .set(lamp::TOP_ADVANCE_BONUS[0], LampMode::Off);
        // The unlit lower-right pop bumper and left top rubber don't advance it.
        sim.tap(sw::POP_BUMPERS[4]);
        sim.tap(sw::TOP_REBOUNDS[0]);
        assert_eq!(sim.machine.bonus(), 10);
        // The lit right top rubber still does.
        sim.tap(sw::TOP_REBOUNDS[1]);
        assert_eq!(sim.machine.bonus(), 20);
    }

    #[test]
    fn a_lane_scores_thirty_when_its_lamp_is_lit() {
        let mut sim = Sim::new();
        sim.tap(sw::START_BUTTON);
        sim.machine.lamps.set(lamp::THIRTY_POINTS[3], LampMode::On);
        sim.tap(sw::SIDE_LANES[2]);
        assert_eq!(sim.machine.score(), 10);
        sim.tap(sw::SIDE_LANES[3]);
        assert_eq!(sim.machine.score(), 40);
    }

    #[test]
    fn the_top_bumper_scores_ten() {
        let mut sim = Sim::new();
        sim.tap(sw::START_BUTTON);
        sim.tap(sw::TOP_BUMPER);
        assert!(sim.coil_on(coil::TOP_BUMPER));
        assert_eq!(sim.machine.score(), 10);
    }

    #[test]
    fn the_side_kickers_kick_and_score() {
        let mut sim = Sim::new();
        sim.tap(sw::START_BUTTON);
        sim.tap(sw::LEFT_SIDE_KICKER);
        assert!(sim.coil_on(coil::LEFT_SIDE_KICKER));
        sim.tap(sw::RIGHT_SIDE_KICKER);
        assert!(sim.coil_on(coil::RIGHT_SIDE_KICKER));
        assert_eq!(sim.machine.score(), 2);
    }

    #[test]
    fn the_jack_in_the_box_kickers_kick_and_score() {
        let mut sim = Sim::new();
        sim.tap(sw::START_BUTTON);
        sim.tap(sw::LEFT_TOP_KICKER);
        assert!(sim.coil_on(coil::LEFT_TOP_KICKER));
        sim.tap(sw::RIGHT_TOP_KICKER);
        assert!(sim.coil_on(coil::RIGHT_TOP_KICKER));
        assert_eq!(sim.machine.score(), 2);
    }

    #[test]
    fn a_tilt_turns_off_the_trapdoor_lamps_with_the_flaps() {
        let mut sim = Sim::new();
        sim.tap(sw::START_BUTTON);
        sim.machine.set_trapdoor(1, true);
        sim.tap(sw::TILT);
        assert!(!sim.flap_open(1));
        assert_eq!(sim.last.lamps & (1 << lamp::TRAPDOORS[1].index()), 0);
    }

    #[test]
    fn a_closed_trapdoor_scores_nothing() {
        let mut sim = Sim::new();
        sim.tap(sw::START_BUTTON);
        sim.tap(sw::TRAPDOORS[0]);
        assert_eq!(sim.machine.score(), 0);
    }

    #[test]
    fn a_trapdoor_lamp_is_lit_while_its_flap_is_open() {
        let mut sim = Sim::new();
        sim.tap(sw::START_BUTTON);
        let lit = |sim: &Sim, n: usize| sim.last.lamps & (1 << lamp::TRAPDOORS[n].index()) != 0;
        sim.machine.set_trapdoor(2, true);
        sim.run(1);
        assert!(lit(&sim, 2) && !lit(&sim, 0));
        sim.tap(sw::TRAPDOORS[2]);
        assert!(!lit(&sim, 2));
    }

    #[test]
    fn full_bonus_wheel_lights_special_for_one_replay() {
        let mut sim = Sim::new();
        sim.tap(sw::START_BUTTON);
        for _ in 0..12 {
            sim.tap(sw::TOP_REBOUNDS[0]);
        }
        assert_eq!(sim.machine.bonus(), 100);
        sim.drop_through(0);
        sim.drop_through(1);
        // 12 rubber points + 2 × 100 bonus = 212: no score replay yet.
        assert_eq!(sim.machine.score(), 212);
        assert_eq!(sim.machine.replays(), 1);
    }

    #[test]
    fn replays_are_awarded_at_card_thresholds() {
        let mut sim = Sim::new();
        sim.tap(sw::START_BUTTON);
        for _ in 0..49 {
            sim.tap(sw::SIDE_LANES[0]);
        }
        assert_eq!(sim.machine.replays(), 0);
        // 500 points crosses the first threshold.
        sim.tap(sw::SIDE_LANES[0]);
        assert_eq!(sim.machine.replays(), 1);
        for _ in 0..9 {
            sim.tap(sw::TOP_REBOUNDS[0]);
        }
        assert_eq!(sim.machine.bonus(), 100);
        for n in 0..3 {
            sim.drop_through(n);
        }
        // 509 + 3 × 100 = 809: crosses 600, 700 and 800, plus the special's
        // replay from the centre trap.
        assert_eq!(sim.machine.score(), 809);
        assert_eq!(sim.machine.replays(), 5);
    }

    #[test]
    fn only_the_centre_trap_awards_the_special() {
        let mut sim = Sim::new();
        sim.tap(sw::START_BUTTON);
        for _ in 0..9 {
            sim.tap(sw::TOP_REBOUNDS[0]);
        }
        let special = |sim: &Sim| sim.last.lamps & (1 << lamp::SPECIAL.index()) != 0;
        assert!(special(&sim));
        sim.drop_through(0);
        sim.drop_through(2);
        assert_eq!(sim.machine.replays(), 0);
        assert!(special(&sim), "the special stays lit");
        sim.drop_through(1);
        assert_eq!(sim.machine.replays(), 1);
        assert!(!special(&sim));
    }

    #[test]
    fn a_tilt_loses_the_score_of_the_ball_in_play() {
        let mut sim = Sim::new();
        sim.tap(sw::START_BUTTON);
        sim.tap(sw::SIDE_LANES[0]);
        sim.drain();
        assert_eq!(sim.machine.score(), 10);
        sim.tap(sw::SIDE_LANES[0]);
        sim.tap(sw::TOP_BUMPER);
        assert_eq!(sim.machine.score(), 30);
        sim.tap(sw::TILT);
        assert_eq!(sim.machine.score(), 10, "back to the score at the serve");
        sim.drain();
        assert_eq!(sim.machine.ball(), 3);
        assert_eq!(sim.machine.score(), 10);
    }

    #[test]
    fn a_tilt_keeps_the_other_players_scores() {
        let mut sim = Sim::new();
        sim.tap(sw::START_BUTTON);
        sim.tap(sw::START_BUTTON);
        sim.tap(sw::SIDE_LANES[0]);
        sim.drain();
        sim.tap(sw::SIDE_LANES[0]);
        sim.tap(sw::TILT);
        assert_eq!(sim.machine.score_of(0), 10);
        assert_eq!(sim.machine.score_of(1), 0);
    }

    #[test]
    fn credits_stop_at_thirty() {
        let mut sim = Sim::new();
        for _ in 0..8 {
            sim.tap(sw::START_BUTTON);
            for _ in 0..90 {
                sim.tap(sw::SIDE_LANES[0]);
            }
            sim.tap(sw::SLAM_TILT);
        }
        // Five replays a game for eight games would be 40.
        assert_eq!(sim.machine.credits(), 30);
    }

    #[test]
    fn a_replay_isnt_paid_twice_after_a_tilt() {
        let mut sim = Sim::new();
        sim.tap(sw::START_BUTTON);
        for _ in 0..49 {
            sim.tap(sw::SIDE_LANES[0]);
        }
        sim.drain();
        sim.tap(sw::SIDE_LANES[0]);
        assert_eq!(sim.machine.replays(), 1);
        sim.tap(sw::TILT);
        assert_eq!(sim.machine.score(), 490);
        sim.drain();
        // Back past 500 on the next ball: no second replay for it.
        sim.tap(sw::SIDE_LANES[0]);
        assert_eq!(sim.machine.score(), 500);
        assert_eq!((sim.machine.replays(), sim.machine.credits()), (1, 1));
        // But 600 still pays.
        for _ in 0..10 {
            sim.tap(sw::SIDE_LANES[0]);
        }
        assert_eq!(sim.machine.replays(), 2);
    }

    #[test]
    fn flap_servos_use_the_trimmed_angles() {
        let mut sim = Sim::new();
        let mut trim = FlapTrim::DEFAULT;
        trim.nudge(1, true, -10);
        sim.machine.set_flap_trim(trim);
        sim.tap(sw::START_BUTTON);
        assert_eq!(sim.last.flap_angles, [90, 90, 90]);
        sim.machine.set_trapdoor(1, true);
        sim.run(1);
        assert_eq!(sim.last.flap_angles, [90, 35, 90]);
        assert!(sim.machine.trapdoor_open(1));
    }

    #[test]
    fn the_flaps_stage_moves_each_flap_and_trims_it() {
        let mut sim = Sim::new();
        sim.tap(sw::TEST_BUTTON);
        sim.tap(sw::TEST_BUTTON);
        let reports = reports_during(&mut sim, |s| s.tap(sw::TEST_BUTTON));
        assert_eq!(
            reports,
            [
                TestReport::Stage(TestStage::Flaps),
                TestReport::Flap {
                    n: 0,
                    open: false,
                    angle: 90
                }
            ]
        );
        assert_eq!(sim.last.coils, 0, "coil power is off");
        // Start moves flap 1 to its open position.
        sim.tap(sw::START_BUTTON);
        assert_eq!(sim.last.flap_angles, [45, 90, 90]);
        // The flippers trim that position.
        sim.tap(sw::RIGHT_FLIPPER_BUTTON);
        sim.tap(sw::RIGHT_FLIPPER_BUTTON);
        let reports = reports_during(&mut sim, |s| s.tap(sw::LEFT_FLIPPER_BUTTON));
        assert_eq!(
            reports,
            [TestReport::Flap {
                n: 0,
                open: true,
                angle: 46
            }]
        );
        assert_eq!(sim.last.flap_angles, [46, 90, 90]);
        // Then flap 2 closed, flap 2 open, and on round to flap 1 again.
        sim.tap(sw::START_BUTTON);
        sim.tap(sw::LEFT_FLIPPER_BUTTON);
        assert_eq!(sim.last.flap_angles, [90, 89, 90]);
        for _ in 0..4 {
            sim.tap(sw::START_BUTTON);
        }
        // Back to flap 1 closed; flap 2 keeps its trimmed closed angle.
        assert_eq!(sim.last.flap_angles, [90, 89, 90]);
        // The trims stay after test mode, for the firmware to save.
        sim.tap(sw::TEST_BUTTON);
        sim.tap(sw::TEST_BUTTON);
        assert_eq!(sim.machine.phase(), Phase::Attract);
        assert_eq!(sim.machine.flap_trim().angle(0, true), 46);
        assert_eq!(sim.machine.flap_trim().angle(1, false), 89);
        assert_eq!(sim.last.flap_angles, [90, 89, 90], "all closed");
    }

    #[test]
    fn trapdoor_flap_stays_open_until_a_ball_drops_through() {
        let mut sim = Sim::new();
        sim.tap(sw::START_BUTTON);
        sim.machine.set_trapdoor(1, true);
        sim.run(5_000);
        assert!(sim.flap_open(1));

        sim.tap(sw::TRAPDOORS[1]);
        assert!(!sim.flap_open(1));
        // The ball isn't over until it reaches the outhole.
        assert_eq!(sim.machine.ball(), 1);
        sim.drain();
        assert_eq!(sim.machine.ball(), 2);
    }

    #[test]
    fn trapdoors_close_on_tilt_and_stay_shut() {
        let mut sim = Sim::new();
        sim.tap(sw::START_BUTTON);
        sim.machine.set_trapdoor(0, true);
        sim.tap(sw::TILT);
        assert!(!sim.flap_open(0));
        sim.machine.set_trapdoor(0, true);
        sim.run(5);
        assert!(!sim.flap_open(0));
    }

    #[test]
    fn trapdoors_are_closed_for_each_new_ball() {
        let mut sim = Sim::new();
        sim.tap(sw::START_BUTTON);
        sim.machine.set_trapdoor(2, true);
        sim.drain();
        assert!(!sim.flap_open(2));
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
        sim.drain();
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

        sim.drain();
        assert_eq!((sim.machine.current_player(), sim.machine.ball()), (2, 1));
        sim.tap(sw::SIDE_LANES[0]);
        sim.tap(sw::SIDE_LANES[0]);

        sim.drain();
        assert_eq!((sim.machine.current_player(), sim.machine.ball()), (1, 2));
        assert_eq!(sim.machine.score_of(0), 10);
        assert_eq!(sim.machine.score_of(1), 20);
    }

    #[test]
    fn game_lasts_the_set_number_of_balls_for_every_player() {
        let settings = Settings {
            balls_per_game: 3,
            replays: true,
            bells: false,
            attract_music: false,
        };
        let mut sim = Sim::new();
        sim.machine = Machine::with_settings(settings);
        sim.tap(sw::START_BUTTON);
        sim.tap(sw::START_BUTTON);
        for _ in 0..(3 * 2 - 1) {
            sim.drain();
            assert_eq!(sim.machine.phase(), Phase::Playing);
        }
        sim.drain();
        assert_eq!(sim.machine.phase(), Phase::Attract);
    }

    #[test]
    fn balls_per_game_is_clamped_to_three_to_five() {
        let settings = |balls_per_game| Settings {
            balls_per_game,
            replays: true,
            bells: false,
            attract_music: false,
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
        sim.drain();
        assert_eq!(sim.machine.phase(), Phase::Playing);
        assert_eq!((sim.machine.current_player(), sim.machine.ball()), (2, 1));
    }

    #[test]
    fn novelty_mode_awards_no_replays() {
        let mut sim = Sim::new();
        sim.machine = Machine::with_settings(Settings {
            balls_per_game: 5,
            replays: false,
            bells: false,
            attract_music: false,
        });
        sim.tap(sw::START_BUTTON);
        for _ in 0..50 {
            sim.tap(sw::SIDE_LANES[0]);
        }
        assert_eq!(sim.machine.score(), 500);
        assert_eq!(sim.machine.replays(), 0);
    }

    #[test]
    fn replays_add_credits_that_last_between_games() {
        let mut sim = Sim::new();
        sim.tap(sw::START_BUTTON);
        for _ in 0..50 {
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

        sim.drain();
        assert!(!lit(&sim, lamp::PLAYER_UP[0]) && lit(&sim, lamp::PLAYER_UP[1]));
        sim.drain();
        assert!(lit(&sim, lamp::BALL_IN_PLAY[1]) && !lit(&sim, lamp::BALL_IN_PLAY[0]));

        sim.tap(sw::SLAM_TILT);
        assert!(
            lamp::PLAYER_UP
                .iter()
                .chain(&lamp::BALL_IN_PLAY)
                .all(|&l| !lit(&sim, l))
        );
    }

    /// Every sound started while running `f`.
    fn sounds_during(sim: &mut Sim, f: impl FnOnce(&mut Sim)) -> Vec<Sound> {
        sim.sounds.clear();
        f(sim);
        std::mem::take(&mut sim.sounds)
    }

    #[test]
    fn game_events_make_sounds() {
        let mut sim = Sim::new();
        // Several sounds can play at once: the start-up and the first ball.
        assert_eq!(
            sounds_during(&mut sim, |s| s.tap(sw::START_BUTTON)),
            [Sound::GameStart, Sound::BallServe]
        );
        assert_eq!(
            sounds_during(&mut sim, |s| s.tap(sw::POP_BUMPERS[0])),
            [Sound::PopBumper]
        );
        assert_eq!(
            sounds_during(&mut sim, |s| s.tap(sw::SIDE_LANES[0])),
            [Sound::Lane]
        );
        // A rubber rings the wheel's climbing bell, not the lane bell as well.
        assert_eq!(
            sounds_during(&mut sim, |s| s.tap(sw::TOP_REBOUNDS[0])),
            [Sound::BonusAdvance(1)]
        );
        assert_eq!(
            sounds_during(&mut sim, |s| s.drop_through(1)),
            [Sound::BonusCollect, Sound::Trapdoor]
        );
        assert_eq!(sounds_during(&mut sim, |s| s.tap(sw::TILT)), [Sound::Tilt]);
        assert_eq!(sounds_during(&mut sim, Sim::drain), [Sound::Drain]);
    }

    #[test]
    fn a_full_wheel_falls_back_to_the_lane_bell() {
        let mut sim = Sim::new();
        sim.tap(sw::START_BUTTON);
        for _ in 0..9 {
            sim.tap(sw::TOP_REBOUNDS[0]);
        }
        assert_eq!(
            sounds_during(&mut sim, |s| s.tap(sw::TOP_REBOUNDS[0])),
            [Sound::Lane]
        );
    }

    #[test]
    fn a_burst_of_bumpers_doesnt_stack_chimes() {
        let mut sim = Sim::new();
        sim.tap(sw::START_BUTTON);
        sim.sounds.clear();
        // Two bumpers closing within a few milliseconds.
        sim.switches.set(sw::POP_BUMPERS[0], true);
        sim.run(3);
        sim.switches.set(sw::POP_BUMPERS[1], true);
        sim.run(5);
        let chimes = sim
            .sounds
            .iter()
            .filter(|&&s| s == Sound::PopBumper)
            .count();
        assert_eq!(chimes, 1);
    }

    #[test]
    fn adding_a_player_during_the_march_honks() {
        let mut sim = Sim::new();
        sim.tap(sw::START_BUTTON);
        sim.run(500);
        assert_eq!(
            sounds_during(&mut sim, |s| s.tap(sw::START_BUTTON)),
            [Sound::PlayerAdded]
        );
        assert_eq!(sim.machine.players(), 2);
    }

    #[test]
    fn fitted_bells_ring_for_points_and_replace_the_chimes() {
        let mut sim = Sim::new();
        sim.machine = Machine::with_settings(Settings {
            bells: true,
            ..Settings::DEFAULT
        });
        sim.tap(sw::START_BUTTON);
        sim.run(Sound::GameStart.duration_ms());
        let tens = 1 << coil::BELLS[1].index();
        let sounds = sounds_during(&mut sim, |s| s.tap(sw::SIDE_LANES[0]));
        assert!(!sounds.contains(&Sound::Lane), "{sounds:?}");
        assert!(
            !sounds.iter().any(|s| matches!(s, Sound::BonusAdvance(_))),
            "{sounds:?}"
        );
        // The 10s bell rang once for the 10 points.
        let mut rang = false;
        for _ in 0..200 {
            sim.run(1);
            rang |= sim.last.coils & tens != 0;
        }
        assert!(rang);
    }

    #[test]
    fn bells_are_silent_unless_fitted() {
        let mut sim = Sim::new();
        sim.tap(sw::START_BUTTON);
        let bells: u32 = coil::BELLS.iter().map(|b| 1 << b.index()).sum();
        for _ in 0..5 {
            sim.tap(sw::SIDE_LANES[0]);
            assert_eq!(sim.last.coils & bells, 0);
        }
    }

    #[test]
    fn attract_music_plays_after_a_pause_and_the_lamps_dance() {
        use crate::music_beats::ATTRACT_MUSIC_BEATS_MS;
        let mut sim = Sim::new();
        // Nothing during the pause (the game-over sound has it to itself)...
        assert_eq!(sounds_during(&mut sim, |s| s.run(MUSIC_DELAY_MS - 1)), []);
        // ...then the music.
        let started = sounds_during(&mut sim, |s| s.run(10));
        assert_eq!(started, [Sound::AttractMusic]);
        // A few beats in, the playfield lamps are dancing.
        sim.run(ATTRACT_MUSIC_BEATS_MS[4]);
        let wheel = lamp::BONUS_WHEEL
            .iter()
            .any(|&l| sim.last.lamps & (1 << l.index()) != 0);
        let pops = lamp::POP_BUMPERS
            .iter()
            .any(|&l| sim.last.lamps & (1 << l.index()) != 0);
        assert!(wheel || pops);
        // Game over stays lit throughout.
        assert_ne!(sim.last.lamps & (1 << lamp::GAME_OVER.index()), 0);
    }

    #[test]
    fn starting_a_game_stops_the_music_and_the_show() {
        let mut sim = Sim::new();
        sim.run(MUSIC_DELAY_MS + 1_000);
        sim.switches.set(sw::START_BUTTON, true);
        let mut stopped = false;
        for _ in 0..5 {
            sim.run(1);
            stopped |= sim.last.stop_sounds.contains(Sound::AttractMusic);
        }
        assert!(stopped);
        assert_eq!(sim.machine.phase(), Phase::Playing);
        // Only the game's own lamps: the first bonus position and the pop caps.
        let show_left: Vec<_> = lamp::BONUS_WHEEL[1..]
            .iter()
            .chain(&lamp::PLAYER_UP[1..])
            .chain(&lamp::BALL_IN_PLAY[1..])
            .filter(|&&l| sim.last.lamps & (1 << l.index()) != 0)
            .collect();
        assert!(show_left.is_empty(), "{show_left:?}");
    }

    #[test]
    fn the_music_comes_back_after_a_game() {
        let mut sim = Sim::new();
        sim.machine = Machine::with_settings(Settings {
            balls_per_game: 3,
            ..Settings::DEFAULT
        });
        sim.tap(sw::START_BUTTON);
        for _ in 0..3 {
            sim.drain();
        }
        assert_eq!(sim.machine.phase(), Phase::Attract);
        assert!(
            !sounds_during(&mut sim, |s| s.run(MUSIC_DELAY_MS - 20)).contains(&Sound::AttractMusic)
        );
        assert!(sounds_during(&mut sim, |s| s.run(40)).contains(&Sound::AttractMusic));
    }

    #[test]
    fn attract_music_can_be_turned_off() {
        let mut sim = Sim::new();
        sim.machine = Machine::with_settings(Settings {
            attract_music: false,
            ..Settings::DEFAULT
        });
        assert!(
            !sounds_during(&mut sim, |s| s.run(MUSIC_DELAY_MS * 2)).contains(&Sound::AttractMusic)
        );
    }

    #[test]
    fn attract_music_repeats_from_the_top() {
        let mut sim = Sim::new();
        sim.run(MUSIC_DELAY_MS + 10);
        // Exactly one restart each time the music ends, right on time.
        let again = sounds_during(&mut sim, |s| s.run(ATTRACT_MUSIC_MS));
        assert_eq!(again, [Sound::AttractMusic]);
        let twice = sounds_during(&mut sim, |s| s.run(2 * ATTRACT_MUSIC_MS));
        assert_eq!(twice, [Sound::AttractMusic, Sound::AttractMusic]);
    }

    #[test]
    fn last_drain_plays_game_over() {
        let mut sim = Sim::new();
        sim.machine = Machine::with_settings(Settings {
            balls_per_game: 3,
            replays: true,
            bells: false,
            attract_music: false,
        });
        sim.tap(sw::START_BUTTON);
        sim.run(5_000);
        sim.drain();
        sim.drain();
        sim.run(1_000);
        assert_eq!(sounds_during(&mut sim, Sim::drain), [Sound::GameOver]);
    }

    #[test]
    fn a_replay_knocks() {
        let mut sim = Sim::new();
        sim.tap(sw::START_BUTTON);
        for _ in 0..49 {
            sim.tap(sw::SIDE_LANES[0]);
        }
        sim.run(5_000);
        // 500 points: the knocker, along with the lane's bell.
        assert!(sounds_during(&mut sim, |s| s.tap(sw::SIDE_LANES[0])).contains(&Sound::Replay));
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
