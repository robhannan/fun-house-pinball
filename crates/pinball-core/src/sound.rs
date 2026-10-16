//! Sound effects: which sound to play, and the DFPlayer Mini commands that play
//! it.
//!
//! The rules ask for a [`Sound`] whenever something happens. The DFPlayer only
//! plays one sound at a time, so a [`SoundScheduler`] decides which request
//! actually plays: a new sound starts if nothing is playing, or if it's at least
//! as important as the one that is. That stops a run of pop bumpers from
//! cutting off the game-start calliope, while letting bumpers retrigger each
//! other.
//!
//! The sounds themselves are synthesised in `sim/web/sounds.js`, which the
//! simulator plays directly and `pnpm run render:sounds` renders to WAV files
//! for the DFPlayer's SD card. Tests check that file against [`Sound::ALL`].

use crate::time::Instant;

/// A sound effect. The mix is classic pinball (bells, thumps, the knocker,
/// the tilt buzzer) with circus extras (calliope, drum roll, clown horn, slide
/// whistles, sad trombone).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sound {
    /// A pop bumper or the top centre bumper: a low thump.
    PopBumper,
    /// A kicking rubber: a sharp thwack.
    Kicker,
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
    /// Tilt: the buzzer.
    Tilt,
    /// A game started: a calliope circus march.
    GameStart,
    /// A player joined: a clown horn.
    PlayerAdded,
    /// A ball was served: a slide whistle going up.
    BallServe,
    /// A ball drained: a slide whistle going down.
    Drain,
    /// A ball dropped through a trapdoor: a spring "boing".
    Trapdoor,
    /// The game ended: a sad trombone.
    GameOver,
}

impl Sound {
    /// Every sound, including each bonus-wheel position.
    pub const ALL: [Self; 23] = [
        Self::PopBumper,
        Self::Kicker,
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
    ];

    /// The sound's file number on the SD card (`01/NNN.wav`) and in
    /// `sim/web/sounds.js`.
    #[must_use]
    pub const fn track(self) -> u8 {
        match self {
            Self::PopBumper => 1,
            Self::Kicker => 2,
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
            Self::BonusAdvance(n) => 20 + clamp_step(n),
        }
    }

    /// Higher wins when sounds compete for the player.
    #[must_use]
    pub const fn priority(self) -> u8 {
        match self {
            Self::PopBumper | Self::Kicker => 1,
            Self::Lane | Self::BonusAdvance(_) => 2,
            Self::ThirtyPoints => 3,
            Self::PlayerAdded | Self::BallServe => 4,
            Self::Drain | Self::Trapdoor => 5,
            Self::BonusCollect | Self::Special => 6,
            Self::Replay => 7,
            Self::Tilt => 8,
            Self::GameStart | Self::GameOver => 9,
        }
    }

    /// How long the sound lasts. `sim/web/sounds.js` must match (a test checks).
    #[must_use]
    pub const fn duration_ms(self) -> u32 {
        match self {
            Self::PopBumper | Self::Kicker => 150,
            Self::Replay | Self::BonusAdvance(_) => 300,
            Self::Lane => 400,
            Self::PlayerAdded => 500,
            Self::ThirtyPoints => 600,
            Self::BallServe | Self::Trapdoor => 700,
            Self::Drain => 900,
            Self::Special | Self::Tilt => 1_200,
            Self::BonusCollect => 1_600,
            Self::GameOver => 2_600,
            Self::GameStart => 4_000,
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

/// Picks which requested sound plays, one at a time.
#[derive(Clone, Copy, Debug, Default)]
pub struct SoundScheduler {
    requested: Option<Sound>,
    playing: Option<(Sound, Instant)>,
}

impl SoundScheduler {
    /// A scheduler with nothing playing.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            requested: None,
            playing: None,
        }
    }

    /// Asks for `sound`. If several are asked for before the next
    /// [`SoundScheduler::next`], the most important wins (the latest, on a tie).
    pub fn request(&mut self, sound: Sound) {
        if self
            .requested
            .is_none_or(|current| sound.priority() >= current.priority())
        {
            self.requested = Some(sound);
        }
    }

    /// The sound to start now, if any. A request is dropped if a more important
    /// sound is still playing.
    pub fn next(&mut self, now: Instant) -> Option<Sound> {
        let sound = self.requested.take()?;
        if let Some((playing, ends)) = self.playing {
            if !now.reached(ends) && sound.priority() < playing.priority() {
                return None;
            }
        }
        self.playing = Some((sound, now.plus(sound.duration_ms())));
        Some(sound)
    }
}

/// The DFPlayer Mini's volume setting at power-on (0–30).
pub const DFPLAYER_VOLUME: u8 = 20;

/// The DFPlayer Mini command that plays `track` from folder `01` on the SD card
/// (file `01/NNN.wav` or `01/NNN.mp3`, where `NNN` is the zero-padded track).
#[must_use]
pub const fn dfplayer_play(track: u8) -> [u8; 10] {
    dfplayer_command(0x0F, 0x01, track)
}

/// The DFPlayer Mini command that sets the volume (clamped to 0–30).
#[must_use]
pub const fn dfplayer_volume(volume: u8) -> [u8; 10] {
    let volume = if volume > 30 { 30 } else { volume };
    dfplayer_command(0x06, 0x00, volume)
}

/// A 10-byte DFPlayer frame: start, version, length, command, no feedback,
/// two parameter bytes, a checksum (the negated sum of bytes 1–6) and the end
/// marker.
const fn dfplayer_command(command: u8, high: u8, low: u8) -> [u8; 10] {
    let sum = 0xFF_u16 + 0x06 + command as u16 + high as u16 + low as u16;
    let checksum = 0_u16.wrapping_sub(sum).to_be_bytes();
    [
        0x7E,
        0xFF,
        0x06,
        command,
        0x00,
        high,
        low,
        checksum[0],
        checksum[1],
        0xEF,
    ]
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    const fn at(ms: u32) -> Instant {
        Instant::from_millis(ms)
    }

    #[test]
    fn checksum_matches_the_dfplayer_datasheet_example() {
        // "Play track 1" (command 0x03) from the DFPlayer Mini datasheet.
        assert_eq!(
            dfplayer_command(0x03, 0x00, 0x01),
            [0x7E, 0xFF, 0x06, 0x03, 0x00, 0x00, 0x01, 0xFE, 0xF7, 0xEF]
        );
    }

    #[test]
    fn play_and_volume_commands() {
        assert_eq!(
            dfplayer_play(9),
            [0x7E, 0xFF, 0x06, 0x0F, 0x00, 0x01, 0x09, 0xFE, 0xE2, 0xEF]
        );
        assert_eq!(dfplayer_volume(99)[6], 30);
    }

    #[test]
    fn tracks_are_unique() {
        let tracks: HashSet<u8> = Sound::ALL.iter().map(|s| s.track()).collect();
        assert_eq!(tracks.len(), Sound::ALL.len());
    }

    #[test]
    fn a_more_important_sound_is_not_cut_off() {
        let mut s = SoundScheduler::new();
        s.request(Sound::GameStart);
        assert_eq!(s.next(at(0)), Some(Sound::GameStart));

        s.request(Sound::PopBumper);
        assert_eq!(s.next(at(100)), None);

        // Once it has finished, anything can play.
        s.request(Sound::PopBumper);
        assert_eq!(s.next(at(4_000)), Some(Sound::PopBumper));
    }

    #[test]
    fn equal_or_higher_priority_interrupts() {
        let mut s = SoundScheduler::new();
        s.request(Sound::PopBumper);
        s.next(at(0));
        s.request(Sound::Kicker);
        assert_eq!(s.next(at(10)), Some(Sound::Kicker));
        s.request(Sound::Tilt);
        assert_eq!(s.next(at(20)), Some(Sound::Tilt));
    }

    #[test]
    fn the_most_important_request_in_a_tick_wins() {
        let mut s = SoundScheduler::new();
        s.request(Sound::Replay);
        s.request(Sound::Lane);
        assert_eq!(s.next(at(0)), Some(Sound::Replay));
        assert_eq!(s.next(at(1)), None);
    }
}
