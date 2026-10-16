//! Sound effects: which sounds to play, and the WAV Trigger commands that play
//! them.
//!
//! The rules ask for a [`Sound`] whenever something happens. The WAV Trigger
//! (by Robertsonics) mixes up to 14 sounds at once, so in fast play several can
//! overlap. The [`SoundScheduler`] only stops the same sound restarting within
//! [`MIN_RETRIGGER_MS`], so a ball rattling between pop bumpers doesn't stack
//! a dozen copies of the chime. Important sounds are [`Sound::locked`] on the
//! board, so a flurry of chimes can't steal their voices.
//!
//! The sound files are built in `sim/web/` (recordings, with synthesised
//! fallbacks in `sounds.js`); the simulator plays them directly, and
//! `pnpm run render:sounds` writes them to the WAV Trigger's SD card. Tests
//! check `sounds.js` against [`Sound::ALL`].

use crate::time::Instant;

/// A sound effect: mostly the sounds a 1950s machine made (bells, the knocker,
/// relays, the score motor), with circus extras (drum roll, fanfare, slide
/// whistles, sad trombone).
///
/// Coils make their own noise on the real machine, so there are no sounds for
/// kickers, flippers or the pop bumpers' solenoids; the simulator adds those
/// itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sound {
    /// A pop bumper or the top centre bumper scored: a chime.
    PopBumper,
    /// A lane or rollover: a bell.
    Lane,
    /// A lit "30 points" rollover: three rising bells.
    ThirtyPoints,
    /// The bonus wheel moved to position `n` (1 = 20 points ... 9 = 100): a bell
    /// that climbs with the wheel.
    BonusAdvance(u8),
    /// The "Scores Bonus" gate collected the bonus: drum roll and cymbal crash.
    BonusCollect,
    /// "Special when lit" lit up: a calliope fanfare.
    Special,
    /// A replay was won: the knocker.
    Replay,
    /// Tilt: the tilt relay dropping out.
    Tilt,
    /// A game started: the score motor running, reels resetting and a chime.
    GameStart,
    /// A player joined: a relay click and a bell.
    PlayerAdded,
    /// A ball was served: a slide whistle going up.
    BallServe,
    /// A ball drained: a slide whistle going down.
    Drain,
    /// A ball dropped through a trapdoor: the ball falling into the hole.
    Trapdoor,
    /// The game ended: a sad trombone.
    GameOver,
    /// Background music in attract mode: "Entrance of the Gladiators" (Julius
    /// Fučík, 1897), the classic circus march. The lamps dance to its beats.
    AttractMusic,
}

impl Sound {
    /// Every sound, including each bonus-wheel position.
    pub const ALL: [Self; 23] = [
        Self::PopBumper,
        Self::Lane,
        Self::ThirtyPoints,
        Self::BonusAdvance(1),
        Self::BonusAdvance(2),
        Self::BonusAdvance(3),
        Self::BonusAdvance(4),
        Self::BonusAdvance(5),
        Self::BonusAdvance(6),
        Self::BonusAdvance(7),
        Self::BonusAdvance(8),
        Self::BonusAdvance(9),
        Self::BonusCollect,
        Self::Special,
        Self::Replay,
        Self::Tilt,
        Self::GameStart,
        Self::PlayerAdded,
        Self::BallServe,
        Self::Drain,
        Self::Trapdoor,
        Self::GameOver,
        Self::AttractMusic,
    ];

    /// The sound's file number on the SD card (`01/NNN.wav`) and in
    /// `sim/web/sounds.js`.
    #[must_use]
    pub const fn track(self) -> u8 {
        match self {
            Self::PopBumper => 1,
            Self::Lane => 3,
            Self::ThirtyPoints => 4,
            Self::BonusCollect => 5,
            Self::Special => 6,
            Self::Replay => 7,
            Self::Tilt => 8,
            Self::GameStart => 9,
            Self::PlayerAdded => 10,
            Self::BallServe => 11,
            Self::Drain => 12,
            Self::Trapdoor => 13,
            Self::GameOver => 14,
            Self::AttractMusic => 30,
            Self::BonusAdvance(n) => 20 + clamp_step(n),
        }
    }

    /// Whether the WAV Trigger must never steal this sound's voice when all 14
    /// are busy: the big moments, and the sounds that are the only sign that
    /// something happened (a player joining, a tilt).
    #[must_use]
    pub const fn locked(self) -> bool {
        matches!(
            self,
            Self::GameStart
                | Self::GameOver
                | Self::Tilt
                | Self::PlayerAdded
                | Self::Replay
                | Self::BonusCollect
                | Self::Special
                | Self::AttractMusic
        )
    }

    /// The sound with SD-card track number `track`, if there is one.
    #[must_use]
    pub fn from_track(track: u8) -> Option<Self> {
        Self::ALL.into_iter().find(|sound| sound.track() == track)
    }

    /// How long the sound lasts: the length of its recording. `sim/web/sounds.js`
    /// must match (a test checks).
    #[must_use]
    pub const fn duration_ms(self) -> u32 {
        match self {
            Self::Replay => 400,
            Self::BonusAdvance(_) => 550,
            Self::PopBumper | Self::Tilt => 600,
            Self::PlayerAdded => 700,
            Self::Trapdoor => 750,
            Self::Lane | Self::BallServe => 800,
            Self::ThirtyPoints => 1_100,
            Self::Drain => 1_450,
            Self::Special => 2_000,
            Self::BonusCollect => 2_400,
            Self::GameStart => 2_600,
            Self::GameOver => 4_300,
            Self::AttractMusic => crate::music_beats::ATTRACT_MUSIC_MS,
        }
    }
}

const fn clamp_step(n: u8) -> u8 {
    if n < 1 {
        1
    } else if n > 9 {
        9
    } else {
        n
    }
}

/// A set of sounds, such as those to start in one tick.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Sounds(u32);

impl Sounds {
    /// No sounds.
    pub const NONE: Self = Self(0);

    /// Adds `sound`.
    pub fn insert(&mut self, sound: Sound) {
        self.0 |= 1 << sound.track();
    }

    /// Whether `sound` is in the set.
    #[must_use]
    pub const fn contains(self, sound: Sound) -> bool {
        self.0 & (1 << sound.track()) != 0
    }

    /// Whether the set is empty.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// The sounds in the set, in [`Sound::ALL`] order.
    pub fn iter(self) -> impl Iterator<Item = Sound> {
        Sound::ALL
            .into_iter()
            .filter(move |&sound| self.contains(sound))
    }
}

/// The shortest gap before the same sound can start again.
pub const MIN_RETRIGGER_MS: u32 = 30;

/// Collects the sounds asked for in a tick, and drops quick repeats.
#[derive(Clone, Copy, Debug, Default)]
pub struct SoundScheduler {
    requested: Sounds,
    stopping: Sounds,
    /// When each track last started, by track number.
    last_started: [Option<Instant>; 32],
}

impl SoundScheduler {
    /// A scheduler with nothing requested.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            requested: Sounds::NONE,
            stopping: Sounds::NONE,
            last_started: [None; 32],
        }
    }

    /// Asks for `sound` to start.
    pub fn request(&mut self, sound: Sound) {
        self.requested.insert(sound);
    }

    /// Asks for `sound` to stop, if it's playing.
    pub fn stop(&mut self, sound: Sound) {
        self.stopping.insert(sound);
    }

    /// The sounds to stop now: everything asked to stop since the last call.
    pub fn take_stops(&mut self) -> Sounds {
        core::mem::take(&mut self.stopping)
    }

    /// The sounds to start now: everything requested since the last call,
    /// except a sound that already started less than [`MIN_RETRIGGER_MS`] ago.
    pub fn next(&mut self, now: Instant) -> Sounds {
        let mut start = Sounds::NONE;
        for sound in self.requested.iter() {
            let last = &mut self.last_started[usize::from(sound.track())];
            if last.is_some_and(|at| !now.reached(at.plus(MIN_RETRIGGER_MS))) {
                continue;
            }
            *last = Some(now);
            start.insert(sound);
        }
        self.requested = Sounds::NONE;
        start
    }
}

/// The WAV Trigger's output gain at power-on, in dB (-70 to +10; 0 = unity).
pub const WAV_TRIGGER_GAIN_DB: i16 = 0;

/// The WAV Trigger command that plays `track` mixed with whatever is playing
/// (`PLAY_POLY`), locked against voice stealing if `lock`.
#[must_use]
pub const fn wav_trigger_play(track: u16, lock: bool) -> [u8; 9] {
    let [low, high] = track.to_le_bytes();
    [0xF0, 0xAA, 0x09, 0x0D, 0x01, low, high, lock as u8, 0x55]
}

/// The WAV Trigger command that sets the output gain in dB (clamped to -70 to
/// +10).
#[must_use]
pub const fn wav_trigger_gain(db: i16) -> [u8; 7] {
    let db = if db < -70 {
        -70
    } else if db > 10 {
        10
    } else {
        db
    };
    let [low, high] = db.to_le_bytes();
    [0xF0, 0xAA, 0x07, 0x05, low, high, 0x55]
}

/// The WAV Trigger command that turns its on-board speaker amplifier on or off.
#[must_use]
pub const fn wav_trigger_amp(on: bool) -> [u8; 6] {
    [0xF0, 0xAA, 0x06, 0x09, on as u8, 0x55]
}

/// The WAV Trigger command that stops `track`.
#[must_use]
pub const fn wav_trigger_stop(track: u16) -> [u8; 9] {
    let [low, high] = track.to_le_bytes();
    [0xF0, 0xAA, 0x09, 0x0D, 0x04, low, high, 0x00, 0x55]
}

/// The WAV Trigger command that stops every track.
#[must_use]
pub const fn wav_trigger_stop_all() -> [u8; 5] {
    [0xF0, 0xAA, 0x05, 0x04, 0x55]
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    const fn at(ms: u32) -> Instant {
        Instant::from_millis(ms)
    }

    // Examples from the Robertsonics WAV Trigger User Guide (2023-01-14).

    #[test]
    fn play_matches_the_user_guide_example() {
        // "Plays track 7 polyphonically with voice locked".
        assert_eq!(
            wav_trigger_play(7, true),
            [0xF0, 0xAA, 0x09, 0x0D, 0x01, 0x07, 0x00, 0x01, 0x55]
        );
        assert_eq!(wav_trigger_play(300, false)[5..8], [0x2C, 0x01, 0x00]);
    }

    #[test]
    fn gain_matches_the_user_guide_example() {
        // "Sets output gain to -10dB".
        assert_eq!(
            wav_trigger_gain(-10),
            [0xF0, 0xAA, 0x07, 0x05, 0xF6, 0xFF, 0x55]
        );
        assert_eq!(wav_trigger_gain(99)[4], 10);
    }

    #[test]
    fn stop_uses_the_stop_track_code() {
        // CONTROL_TRACK with code STOP (0x04), per the user guide.
        assert_eq!(
            wav_trigger_stop(30),
            [0xF0, 0xAA, 0x09, 0x0D, 0x04, 0x1E, 0x00, 0x00, 0x55]
        );
    }

    #[test]
    fn stops_are_collected_once() {
        let mut s = SoundScheduler::new();
        s.stop(Sound::AttractMusic);
        assert!(s.take_stops().contains(Sound::AttractMusic));
        assert!(s.take_stops().is_empty());
    }

    #[test]
    fn amp_and_stop_all_match_the_user_guide_examples() {
        assert_eq!(wav_trigger_amp(true), [0xF0, 0xAA, 0x06, 0x09, 0x01, 0x55]);
        assert_eq!(wav_trigger_stop_all(), [0xF0, 0xAA, 0x05, 0x04, 0x55]);
    }

    #[test]
    fn tracks_are_unique_and_fit_the_set() {
        let tracks: HashSet<u8> = Sound::ALL.iter().map(|s| s.track()).collect();
        assert_eq!(tracks.len(), Sound::ALL.len());
        assert!(tracks.iter().all(|&t| t < 32));
        for sound in Sound::ALL {
            assert_eq!(Sound::from_track(sound.track()), Some(sound));
        }
    }

    #[test]
    fn several_sounds_can_start_at_once() {
        let mut s = SoundScheduler::new();
        s.request(Sound::GameStart);
        s.request(Sound::BallServe);
        s.request(Sound::PopBumper);
        let started: Vec<Sound> = s.next(at(0)).iter().collect();
        assert_eq!(
            started,
            [Sound::PopBumper, Sound::GameStart, Sound::BallServe]
        );
        assert!(s.next(at(1)).is_empty());
    }

    #[test]
    fn the_same_sound_cannot_restart_too_quickly() {
        let mut s = SoundScheduler::new();
        s.request(Sound::PopBumper);
        assert!(s.next(at(0)).contains(Sound::PopBumper));
        s.request(Sound::PopBumper);
        s.request(Sound::Lane);
        let started = s.next(at(10));
        assert!(!started.contains(Sound::PopBumper));
        assert!(started.contains(Sound::Lane));
        s.request(Sound::PopBumper);
        assert!(s.next(at(MIN_RETRIGGER_MS)).contains(Sound::PopBumper));
    }

    #[test]
    fn the_big_moments_are_locked() {
        assert!(Sound::GameStart.locked() && Sound::Tilt.locked() && Sound::PlayerAdded.locked());
        assert!(!Sound::PopBumper.locked() && !Sound::Lane.locked());
    }
}
