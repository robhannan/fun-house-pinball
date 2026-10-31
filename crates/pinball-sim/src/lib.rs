//! A browser simulator for the Fun House game logic.
//!
//! This wraps the real [`pinball_core::Machine`] with a simulated clock and
//! simulated switches, and exports a tiny numeric API to WebAssembly for the
//! page in `sim/web/`. A simulated ball ([`physics`]) rolls round the table,
//! closing switches and being kicked by the real coil outputs; the player can
//! also trigger any switch by clicking its part of the playfield.

pub mod physics;

use pinball_core::{
    Instant, MAX_PLAYERS, Machine, Outputs, Phase, Settings, Sound, SwitchId, SwitchStates,
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
    /// Switches held or tapped by the player.
    switches: SwitchStates,
    /// The ball, flippers and plunger.
    physics: physics::Physics,
    now: Instant,
    /// Switches closed by [`Simulator::tap`], with when to release each one.
    taps: Vec<(SwitchId, Instant)>,
    outputs: Outputs,
    /// Every coil that was on at any point during the last [`Simulator::advance`].
    coils_seen: u32,
    /// Sounds started but not yet played by the page, oldest first.
    sounds: std::collections::VecDeque<Sound>,
    /// Sounds to stop, oldest first.
    stops: std::collections::VecDeque<Sound>,
    /// The latest test-mode reports as text, newest first, as the firmware
    /// prints them on its serial console.
    test_log: std::collections::VecDeque<String>,
    /// How many test reports there have been, so the page knows to redraw.
    test_reports: u32,
}

/// Test-mode lines kept for the page.
pub const TEST_LOG_LINES: usize = 8;

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
            physics: physics::Physics::default(),
            now: Instant::from_millis(0),
            taps: Vec::new(),
            outputs: Outputs::default(),
            coils_seen: 0,
            sounds: std::collections::VecDeque::new(),
            stops: std::collections::VecDeque::new(),
            test_log: std::collections::VecDeque::new(),
            test_reports: 0,
        }
    }

    /// The latest test-mode report lines, newest first.
    pub fn test_log(&self) -> impl Iterator<Item = &str> {
        self.test_log.iter().map(String::as_str)
    }

    /// How many test-mode reports there have been.
    #[must_use]
    pub const fn test_reports(&self) -> u32 {
        self.test_reports
    }

    /// The ball, flippers and plunger.
    #[must_use]
    pub const fn physics(&self) -> &physics::Physics {
        &self.physics
    }

    /// Pulls the plunger back (`true`) or lets it go, launching the ball if
    /// it's in the shooter lane.
    pub fn plunger(&mut self, pulled: bool) {
        self.physics.plunger(pulled);
    }

    /// Bumps the table.
    pub fn nudge(&mut self) {
        self.physics.nudge();
    }

    /// Puts the ball somewhere on the playfield, for tests.
    pub fn place_ball(&mut self, pos: physics::Vec2, vel: physics::Vec2) {
        self.physics.place(pos, vel);
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

    /// The oldest sound to stop that the page hasn't taken yet.
    pub fn take_stop(&mut self) -> Option<Sound> {
        self.stops.pop_front()
    }

    /// The oldest sound started but not yet taken by the page.
    pub fn take_sound(&mut self) -> Option<Sound> {
        self.sounds.pop_front()
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
            // The ball moves with last tick's coils, then the machine sees
            // both the player's switches and the ball's.
            let open_flaps = (0..3)
                .filter(|&n| self.machine.trapdoor_open(n))
                .fold(0, |bits, n| bits | (1 << n));
            let ball = self.physics.tick(self.outputs.coils, open_flaps);
            let mut switches = self.switches;
            for index in 0..pinball_core::switches::SWITCH_COUNT as u32 {
                if let Some(id) = switch_from_index(index)
                    && ball.is_closed(id)
                {
                    switches.set(id, true);
                }
            }
            self.outputs = self.machine.tick(now, &switches);
            self.coils_seen |= self.outputs.coils;
            self.sounds.extend(self.outputs.sounds.iter());
            self.stops.extend(self.outputs.stop_sounds.iter());
            for report in self.outputs.reports.iter() {
                let line = pinball_core::test_mode::describe(report);
                self.test_log.push_front(line.as_str().to_owned());
                self.test_log.truncate(TEST_LOG_LINES);
                self.test_reports = self.test_reports.wrapping_add(1);
            }
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

    /// Power-cycles the machine with `balls_per_game` (clamped to 3–5),
    /// replays on (`replay_mode != 0`) or novelty mode, real bells fitted
    /// (`bells_fitted != 0`) or not, and attract music on (`music != 0`) or off.
    #[unsafe(no_mangle)]
    pub extern "C" fn sim_reset_with(
        balls_per_game: u32,
        replay_mode: u32,
        bells_fitted: u32,
        music: u32,
    ) {
        let settings = Settings {
            balls_per_game: balls_per_game.min(5) as u8,
            replays: replay_mode != 0,
            bells: bells_fitted != 0,
            attract_music: music != 0,
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

    /// 1 if trapdoor `n` (0–2) is open.
    #[unsafe(no_mangle)]
    pub extern "C" fn sim_trapdoor_open(n: u32) -> u32 {
        with(|sim| u32::from(sim.machine().trapdoor_open(n as usize)))
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

    /// The next sound to play: its track number in `sim/web/sounds.js`, plus
    /// 256 if it's locked against voice stealing (as on the WAV Trigger); 0 when
    /// there are none left.
    #[unsafe(no_mangle)]
    pub extern "C" fn sim_take_sound() -> u32 {
        with(|sim| {
            sim.take_sound().map_or(0, |sound| {
                u32::from(sound.track()) | if sound.locked() { 256 } else { 0 }
            })
        })
    }

    /// The next sound to stop, as its track number; 0 when there are none left.
    #[unsafe(no_mangle)]
    pub extern "C" fn sim_take_stop() -> u32 {
        with(|sim| sim.take_stop().map_or(0, |sound| u32::from(sound.track())))
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
            Phase::Test(_) => 3,
        })
    }

    /// The ball's radius, in millimetres (playfield-map units).
    #[unsafe(no_mangle)]
    pub extern "C" fn sim_ball_radius() -> f32 {
        super::physics::BALL_RADIUS
    }

    /// The ball's position across the table, in millimetres (playfield-map units).
    #[unsafe(no_mangle)]
    pub extern "C" fn sim_ball_x() -> f32 {
        with(|sim| sim.physics().ball().x)
    }

    /// The ball's position down the table, in millimetres (playfield-map units).
    #[unsafe(no_mangle)]
    pub extern "C" fn sim_ball_y() -> f32 {
        with(|sim| sim.physics().ball().y)
    }

    /// 1 if the ball is on top of the playfield (or in the outhole), 0 while
    /// it rolls underneath from a trapdoor.
    #[unsafe(no_mangle)]
    pub extern "C" fn sim_ball_shown() -> u32 {
        with(|sim| {
            u32::from(!matches!(
                sim.physics().state(),
                super::physics::BallState::Underneath { .. }
            ))
        })
    }

    /// How far flipper `n` (0 = left, 1 = right) has turned from rest, in
    /// radians (clockwise positive, as SVG rotates).
    #[unsafe(no_mangle)]
    pub extern "C" fn sim_flipper_turn(n: u32) -> f32 {
        with(|sim| sim.physics().flipper_turn(n as usize))
    }

    /// Pulls the plunger back (`pulled != 0`) or lets it go.
    #[unsafe(no_mangle)]
    pub extern "C" fn sim_plunger(pulled: u32) {
        with(|sim| sim.plunger(pulled != 0));
    }

    /// How far the plunger is pulled back, 0 to 1.
    #[unsafe(no_mangle)]
    pub extern "C" fn sim_plunger_pull() -> f32 {
        with(|sim| sim.physics().plunger_pull())
    }

    /// 1 when the ball is waiting in the shooter lane.
    #[unsafe(no_mangle)]
    pub extern "C" fn sim_ready_to_plunge() -> u32 {
        with(|sim| u32::from(sim.physics().ready_to_plunge()))
    }

    /// Bumps the table.
    #[unsafe(no_mangle)]
    pub extern "C" fn sim_nudge() {
        with(Simulator::nudge);
    }

    /// How many test-mode reports there have been; redraw the log when it
    /// changes.
    #[unsafe(no_mangle)]
    pub extern "C" fn sim_test_reports() -> u32 {
        with(|sim| sim.test_reports())
    }

    /// Length in bytes of test-mode log line `n` (0 = newest), or 0.
    #[unsafe(no_mangle)]
    pub extern "C" fn sim_test_line_len(n: u32) -> u32 {
        with(|sim| {
            sim.test_log()
                .nth(n as usize)
                .map_or(0, |line| line.len() as u32)
        })
    }

    /// Byte `i` of test-mode log line `n` (0 = newest), or 0. The lines are
    /// plain ASCII.
    #[unsafe(no_mangle)]
    pub extern "C" fn sim_test_line_byte(n: u32, i: u32) -> u32 {
        with(|sim| {
            sim.test_log()
                .nth(n as usize)
                .and_then(|line| line.as_bytes().get(i as usize))
                .map_or(0, |&b| u32::from(b))
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
    fn test_mode_reports_reach_the_page_newest_first() {
        let mut sim = Simulator::new();
        sim.tap(pinball_core::playfield::sw::TEST_BUTTON);
        sim.advance(100);
        sim.tap(pinball_core::playfield::sw::SIDE_LANES[0]);
        sim.advance(100);
        let lines: Vec<_> = sim.test_log().collect();
        assert_eq!(
            lines,
            [
                "switch C4 R0 opened: Side lane 1",
                "switch C4 R0 closed: Side lane 1",
                "TEST 1/4 switches: close any switch. Test button: next",
            ]
        );
        assert_eq!(sim.test_reports(), 3);
    }

    // Ball physics, driven through the whole simulator so the real rules fire
    // the coils.

    use super::physics::{BALL_RADIUS, BallState, Vec2};

    fn run(sim: &mut Simulator, ms: u32) {
        for _ in 0..ms / 10 {
            sim.advance(10);
        }
    }

    /// Starts a game and waits for the ball to reach the shooter lane.
    fn served() -> Simulator {
        let mut sim = Simulator::new();
        run(&mut sim, 200);
        sim.tap(sw::START_BUTTON);
        run(&mut sim, 300);
        sim
    }

    fn plunge(sim: &mut Simulator, pull_ms: u32) {
        sim.plunger(true);
        run(sim, pull_ms);
        sim.plunger(false);
    }

    // These tests find parts on the playfield map rather than hard-coding
    // where they are, so they keep working as parts are measured and moved.

    /// The playfield map's first tag containing `marker`, such as
    /// `data-switch="C2R0"`.
    fn map_tag(marker: &str) -> &'static str {
        SVG.split('<')
            .find(|tag| tag.contains(marker))
            .unwrap_or_else(|| panic!("no {marker} on the playfield map"))
    }

    /// A number attribute of the map's first tag containing `marker`.
    fn map_number(marker: &str, name: &str) -> f32 {
        let tag = map_tag(marker);
        let start = tag
            .find(&format!(" {name}=\""))
            .unwrap_or_else(|| panic!("{marker} has no {name}"))
            + name.len()
            + 3;
        tag[start..]
            .split('"')
            .next()
            .and_then(|value| value.parse().ok())
            .unwrap_or_else(|| panic!("{marker}'s {name} isn't a number"))
    }

    /// The corners of the path of the map's first tag containing `marker`.
    fn path_corners(marker: &str) -> Vec<Vec2> {
        let tag = map_tag(marker);
        let d = tag
            .split(" d=\"")
            .nth(1)
            .and_then(|rest| rest.split('"').next());
        physics::path_ends(d.unwrap_or_else(|| panic!("{marker} has no path")))
    }

    /// A flipper's pivot and tip: the middles of its base and tip ends, as
    /// the physics finds them.
    fn flipper_ends(side: &str) -> (Vec2, Vec2) {
        let p = path_corners(&format!("data-flipper=\"{side}\""));
        let middle = |a: Vec2, b: Vec2| Vec2::new(f32::midpoint(a.x, b.x), f32::midpoint(a.y, b.y));
        (middle(p[0], p[3]), middle(p[1], p[2]))
    }

    fn left_flipper_pivot() -> Vec2 {
        flipper_ends("left").0
    }

    /// Whether the ball is still on the playfield board.
    fn on_board(p: Vec2) -> bool {
        (0.0..=map_number("data-board=\"\"", "width")).contains(&p.x)
            && (0.0..=map_number("data-board=\"\"", "height")).contains(&p.y)
    }

    #[test]
    fn starting_a_game_serves_the_ball_to_the_shooter_lane() {
        let mut sim = Simulator::new();
        run(&mut sim, 100);
        assert_eq!(sim.physics().state(), BallState::InOuthole);
        sim.tap(sw::START_BUTTON);
        run(&mut sim, 300);
        assert!(
            sim.physics().ready_to_plunge(),
            "{:?}",
            sim.physics().ball()
        );
    }

    #[test]
    fn a_full_plunge_sends_the_ball_round_the_top_and_onto_the_playfield() {
        let mut sim = served();
        plunge(&mut sim, 900);
        // The gate is at the top of the shooter lane's wall.
        let lane_wall = map_number("data-gate=\"\"", "x1");
        let mut highest = f32::MAX;
        let mut reached_playfield = false;
        for _ in 0..300 {
            run(&mut sim, 10);
            let ball = sim.physics().ball();
            highest = highest.min(ball.y);
            reached_playfield |= ball.x < lane_wall - BALL_RADIUS && ball.y < 600.0;
        }
        assert!(highest < 120.0, "only reached y = {highest}");
        assert!(reached_playfield);
    }

    #[test]
    fn a_feeble_plunge_rolls_back_to_the_plunger() {
        let mut sim = served();
        plunge(&mut sim, 0);
        run(&mut sim, 4_000);
        assert!(
            sim.physics().ready_to_plunge(),
            "{:?}",
            sim.physics().ball()
        );
    }

    #[test]
    fn an_unflipped_ball_always_drains_and_the_next_one_is_served() {
        for pull_ms in [300, 450, 600, 750, 900] {
            let mut sim = served();
            plunge(&mut sim, pull_ms);
            let mut ms = 0;
            while sim.machine().ball() == 1 && ms < 120_000 {
                run(&mut sim, 10);
                ms += 10;
                let p = sim.physics().ball();
                assert!(on_board(p), "pull {pull_ms}: ball escaped to {p:?}");
            }
            assert_eq!(
                sim.machine().ball(),
                2,
                "pull {pull_ms}: stuck at {:?}",
                sim.physics().ball()
            );
            run(&mut sim, 1_500);
            assert!(
                sim.physics().ready_to_plunge(),
                "pull {pull_ms}: next ball not served"
            );
        }
    }

    #[test]
    fn a_ball_never_escapes_or_gets_stuck_during_random_play() {
        for seed in [12_345, 777, 2_024] {
            random_play(seed, 12);
        }
    }

    /// Plays `balls` balls with random plunges and flips, failing if the ball
    /// ever leaves the table or gets trapped.
    fn random_play(seed: u32, balls: u32) {
        // A small deterministic random number generator (an LCG).
        let mut seed = seed;
        let mut random = move |n: u32| {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            (seed >> 8) % n
        };
        let mut sim = served();
        // Where the ball has been hanging around, and for how long. A ball
        // trapped somewhere (even one a kicker keeps bouncing) stays near one
        // spot.
        let mut anchor = sim.physics().ball();
        let mut near_ms = 0;
        for _ in 0..balls {
            // Plunge whatever ball is waiting, then flip at random.
            plunge(&mut sim, 300 + random(600));
            for _ in 0..3_000 {
                if random(40) == 0 {
                    let button =
                        [sw::LEFT_FLIPPER_BUTTON, sw::RIGHT_FLIPPER_BUTTON][random(2) as usize];
                    sim.tap(button);
                }
                run(&mut sim, 10);
                let p = sim.physics().ball();
                assert!(on_board(p), "seed {seed}: ball escaped to {p:?}");
                let waiting =
                    sim.physics().state() != BallState::Rolling || sim.physics().ready_to_plunge();
                let (dx, dy) = (p.x - anchor.x, p.y - anchor.y);
                if waiting || dx * dx + dy * dy > 40.0 * 40.0 {
                    anchor = p;
                    near_ms = 0;
                } else {
                    near_ms += 10;
                    assert!(near_ms < 5_000, "seed {seed}: ball trapped near {anchor:?}");
                }
                if sim.machine().phase() == pinball_core::Phase::Attract {
                    sim.tap(sw::START_BUTTON);
                }
            }
        }
    }

    /// A longer search for traps: `cargo test -p pinball-sim --release -- --ignored`.
    #[test]
    #[ignore = "slow; run when changing the table geometry"]
    fn many_random_games() {
        for seed in 1..=40 {
            random_play(seed, 15);
        }
    }

    #[test]
    fn ending_a_ball_by_hand_brings_the_ball_back_to_the_plunger() {
        let mut sim = served();
        plunge(&mut sim, 900);
        run(&mut sim, 1_000);
        assert!(!sim.physics().ready_to_plunge());
        sim.tap(sw::OUTHOLE);
        run(&mut sim, 1_500);
        assert_eq!(sim.machine().ball(), 2);
        assert!(
            sim.physics().ready_to_plunge(),
            "{:?}",
            sim.physics().ball()
        );
    }

    #[test]
    fn a_pop_bumper_scores_and_kicks_the_ball_away() {
        let mut sim = served();
        // Roll the ball gently into pop 1 from above.
        let pop = "data-switch=\"C2R0\"";
        let top = map_number(pop, "cy") - map_number(pop, "r") - BALL_RADIUS;
        sim.place_ball(
            Vec2::new(map_number(pop, "cx"), top - 25.0),
            Vec2::new(0.0, 150.0),
        );
        let mut fastest: f32 = 0.0;
        for _ in 0..40 {
            run(&mut sim, 10);
            let v = sim.physics().velocity();
            fastest = fastest.max((v.x * v.x + v.y * v.y).sqrt());
        }
        assert!(sim.machine().score() >= 1);
        assert!(fastest > 1_000.0, "fastest {fastest}");
    }

    fn just_above_the_middle_trapdoor() -> Vec2 {
        let door = "data-trapdoor=\"1\"";
        Vec2::new(
            map_number(door, "x") + map_number(door, "width") / 2.0,
            map_number(door, "y") - 6.0,
        )
    }

    #[test]
    fn an_open_trapdoor_swallows_the_ball_and_scores_the_bonus() {
        let mut sim = served();
        sim.set_trapdoor(1, true);
        run(&mut sim, 20);
        sim.place_ball(just_above_the_middle_trapdoor(), Vec2::new(0.0, 100.0));
        run(&mut sim, 100);
        assert!(matches!(
            sim.physics().state(),
            BallState::Underneath { .. }
        ));
        assert_eq!(sim.machine().score(), 10, "the bonus");
        run(&mut sim, 2_000);
        assert_eq!(sim.machine().ball(), 2, "it rolled to the outhole");
    }

    #[test]
    fn a_closed_trapdoor_lets_the_ball_roll_over() {
        let mut sim = served();
        sim.place_ball(just_above_the_middle_trapdoor(), Vec2::new(0.0, 100.0));
        run(&mut sim, 300);
        assert_eq!(sim.physics().state(), BallState::Rolling);
        let door = "data-trapdoor=\"1\"";
        let bottom = map_number(door, "y") + map_number(door, "height");
        assert!(sim.physics().ball().y > bottom - 6.0);
    }

    #[test]
    fn a_held_flipper_cradles_the_ball() {
        let mut sim = served();
        sim.set_switch(sw::LEFT_FLIPPER_BUTTON, true);
        run(&mut sim, 100);
        // Drop the ball onto the raised left flipper, by its pivot.
        let pivot = left_flipper_pivot();
        sim.place_ball(pivot + Vec2::new(-5.0, -45.0), Vec2::default());
        run(&mut sim, 3_000);
        assert_eq!(sim.physics().state(), BallState::Rolling);
        // Resting by the pivot, not thrown somewhere else on the table.
        let ball = sim.physics().ball();
        let (dx, dy) = (ball.x - pivot.x, ball.y - pivot.y);
        assert!(dx * dx + dy * dy < 40.0 * 40.0, "{ball:?}");
        // Let go and it rolls off and drains.
        sim.set_switch(sw::LEFT_FLIPPER_BUTTON, false);
        run(&mut sim, 3_000);
        assert_eq!(sim.machine().ball(), 2);
    }

    #[test]
    fn flippers_rest_parallel_to_the_apron() {
        // On the table the flippers lie parallel to the apron's edge at rest,
        // with just room for a ball between. The outline starts at the left
        // wall and closes from the cutout back to it, so its first and last
        // corners are the apron's left edge; the right edge runs from the
        // lane wall to the cutout, its third- and second-last corners.
        let outline = path_corners("data-wall=\"\"");
        let n = outline.len();
        let len = |v: Vec2| v.x.hypot(v.y);
        let angle = |v: Vec2| v.y.atan2(v.x).to_degrees();
        for (side, start, end) in [
            ("left", outline[0], outline[n - 1]),
            ("right", outline[n - 3], outline[n - 2]),
        ] {
            let (pivot, tip) = flipper_ends(side);
            // Both run from the wall end, so compare their slopes.
            let (edge, bat) = (end - start, tip - pivot);
            let slope = |v: Vec2| angle(Vec2::new(v.x.abs(), v.y));
            let apart = slope(edge) - slope(bat);
            assert!(apart.abs() < 2.0, "{side}: {apart}° off parallel");
            // The gap under the base: room for a 27 mm ball, not much more.
            let corner = path_corners(&format!("data-flipper=\"{side}\""))[0];
            let gap = ((pivot - start).x * edge.y - (pivot - start).y * edge.x).abs() / len(edge)
                - len(corner - pivot);
            assert!(
                (BALL_RADIUS * 2.0..45.0).contains(&gap),
                "{side}: gap {gap}"
            );
        }
    }

    #[test]
    fn a_fast_ball_cant_get_stuck_on_the_left_jack_in_the_box() {
        // A fast ball along the kicker's top could once squeeze under the
        // roller into a pocket by the arch, and bounce there for ever.
        let mut sim = served();
        sim.place_ball(Vec2::new(95.0, 173.0), Vec2::new(-3_500.0, 0.0));
        run(&mut sim, 6_000);
        let p = sim.physics().ball();
        let (dx, dy) = (p.x - 55.0, p.y - 170.0);
        assert!(
            dx * dx + dy * dy > 40.0 * 40.0 || sim.physics().state() != BallState::Rolling,
            "{p:?}"
        );
    }

    #[test]
    fn a_flipper_hits_the_ball_up_the_table() {
        let mut sim = served();
        let pivot = left_flipper_pivot();
        sim.place_ball(pivot + Vec2::new(42.5, -3.5), Vec2::default());
        run(&mut sim, 30);
        sim.set_switch(sw::LEFT_FLIPPER_BUTTON, true);
        let mut highest = f32::MAX;
        for _ in 0..100 {
            run(&mut sim, 10);
            highest = highest.min(sim.physics().ball().y);
        }
        // At least a third of the way up the table. The ball leaves early in
        // the swing, so how far it goes depends on the flipper's speed, which
        // is assumed.
        assert!(highest < pivot.y - 350.0, "only reached y = {highest}");
    }

    #[test]
    fn a_tilt_drops_the_flippers() {
        let mut sim = served();
        sim.set_switch(sw::LEFT_FLIPPER_BUTTON, true);
        run(&mut sim, 100);
        assert!(sim.physics().flipper_turn(0) < -0.5);
        sim.tap(sw::TILT);
        run(&mut sim, 200);
        assert!(sim.physics().flipper_turn(0).abs() < 0.01);
    }

    #[test]
    fn trapdoors_can_be_opened_by_hand() {
        let mut sim = Simulator::new();
        sim.tap(sw::START_BUTTON);
        sim.advance(100);
        sim.set_trapdoor(1, true);
        sim.set_trapdoor(7, true); // ignored
        sim.advance(10);
        let open: Vec<_> = (0..3).map(|n| sim.machine().trapdoor_open(n)).collect();
        assert_eq!(open, [false, true, false]);
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
    const SOUNDS_JS: &str = include_str!("../../../sim/web/sounds.js");

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

    /// The 3D-printed ball return's measurements of the table.
    const MEASUREMENTS: &str = include_str!("../../../hardware/3d/measurements.scad");

    /// A number from `hardware/3d/measurements.scad`, by name.
    fn measurement(name: &str) -> f32 {
        MEASUREMENTS
            .lines()
            .find_map(|line| {
                line.strip_prefix(name)?
                    .trim_start()
                    .strip_prefix('=')?
                    .trim()
                    .trim_end_matches(';')
                    .parse()
                    .ok()
            })
            .unwrap_or_else(|| panic!("measurements.scad has no {name}"))
    }

    #[test]
    fn the_3d_measurements_match_the_playfield_map() {
        // The printed parts and the map must agree on the table. The map's y
        // runs from the board's back edge, measurements.scad's from its front.
        let near = |scad: f32, map: f32, what: &str| {
            assert!(
                (scad - map).abs() <= 0.5,
                "{what}: measurements.scad says {scad}, the map {map}"
            );
        };
        let board = "data-board=\"\"";
        let length = map_number(board, "height");
        near(
            measurement("playfield_width"),
            map_number(board, "width"),
            "board width",
        );
        near(measurement("playfield_length"), length, "board length");
        let door = |n: u8, name: &str| map_number(&format!("data-trapdoor=\"{n}\""), name);
        let centre_x = |n: u8| door(n, "x") + door(n, "width") / 2.0;
        near(
            measurement("trapdoor_x"),
            centre_x(1),
            "middle trapdoor across",
        );
        near(
            measurement("trapdoor_y"),
            length - (door(1, "y") + door(1, "height") / 2.0),
            "middle trapdoor up the table",
        );
        near(
            measurement("trapdoor_spacing"),
            centre_x(1) - centre_x(0),
            "trapdoor spacing",
        );
        near(
            measurement("trapdoor_spacing"),
            centre_x(2) - centre_x(1),
            "trapdoor spacing",
        );
        near(
            measurement("hole_width"),
            door(1, "width"),
            "trapdoor hole width",
        );
        near(
            measurement("hole_length"),
            door(1, "height"),
            "trapdoor hole length",
        );
        let outhole = "data-switch=\"C0R0\"";
        near(
            measurement("outhole_x"),
            map_number(outhole, "cx"),
            "outhole across",
        );
        near(
            measurement("outhole_y"),
            length - map_number(outhole, "cy"),
            "outhole up the table",
        );
        near(
            measurement("outhole_diameter"),
            2.0 * map_number(outhole, "r"),
            "outhole diameter",
        );
    }

    #[test]
    fn measured_points_can_line_up_a_photo() {
        // The trace page (sim/web/trace/) lines a photo up with the map by the
        // centres of the parts marked data-measured: at least four, each a
        // circle or rectangle with its own label.
        let mut labels = BTreeSet::new();
        for tag in SVG.split('<').filter(|t| t.contains(" data-measured=\"")) {
            let name = tag.split_whitespace().next();
            assert!(
                matches!(name, Some("circle" | "rect")),
                "data-measured on a {name:?}: the page needs a centre"
            );
            let label = tag
                .split(" data-measured=\"")
                .nth(1)
                .and_then(|rest| rest.split('"').next())
                .unwrap_or_default();
            assert!(!label.is_empty(), "a measured point without a label");
            assert!(labels.insert(label), "two points labelled {label}");
        }
        assert!(labels.len() >= 4, "only {} measured points", labels.len());
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
            bells: false,
            attract_music: false,
        });
        assert_eq!(sim.machine().settings().balls_per_game, 3);
        assert!(!sim.machine().settings().replays);
    }

    #[test]
    fn every_sound_is_synthesised_with_the_right_duration() {
        for sound in Sound::ALL {
            let entry = format!("\n  {}: {{ name: \"", sound.track());
            let start = SOUNDS_JS
                .find(&entry)
                .unwrap_or_else(|| panic!("{sound:?}: no track {} in sounds.js", sound.track()));
            let line = SOUNDS_JS[start + 1..].lines().next().unwrap_or_default();
            let duration = line
                .split("durationMs: ")
                .nth(1)
                .and_then(|rest| rest.split(',').next())
                .and_then(|ms| ms.trim().parse::<u32>().ok());
            assert_eq!(duration, Some(sound.duration_ms()), "{sound:?}: {line}");
        }
        let entries = SOUNDS_JS.matches(": { name: \"").count();
        assert_eq!(
            entries,
            Sound::ALL.len(),
            "sounds.js has tracks the core doesn't use"
        );
    }

    #[test]
    fn every_recorded_sound_exists_and_matches_its_track() {
        let web = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../sim/web");
        for sound in Sound::ALL {
            let entry = format!("\n  {}: {{ name: \"", sound.track());
            let start = SOUNDS_JS.find(&entry).expect("checked above");
            let line = SOUNDS_JS[start + 1..].lines().next().unwrap_or_default();
            let Some(sample) = line
                .split("sample: \"")
                .nth(1)
                .and_then(|rest| rest.split('"').next())
            else {
                continue;
            };
            let stem = format!("samples/{:03}.", sound.track());
            assert!(
                sample == format!("{stem}wav") || sample == format!("{stem}mp3"),
                "{sound:?}: {sample} should be {stem}wav or {stem}mp3"
            );
            assert!(
                web.join(sample).is_file(),
                "{sound:?}: {sample} is missing (pnpm run import:samples)"
            );
        }
    }

    #[test]
    fn started_sounds_are_queued_for_the_page() {
        let mut sim = Simulator::new();
        sim.tap(sw::START_BUTTON);
        sim.advance(100);
        assert_eq!(sim.take_sound(), Some(Sound::GameStart));
        assert_eq!(sim.take_sound(), Some(Sound::BallServe));
        assert_eq!(sim.take_sound(), None);
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
