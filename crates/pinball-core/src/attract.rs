//! The attract-mode light show: the playfield and backboard lamps dance to the
//! beats of the attract music ([`crate::music_beats`]). When the music ends it
//! repeats from the top: the show starts its beats again, exactly
//! [`ATTRACT_MUSIC_MS`] after the last start, and tells the caller to restart
//! the music in the same tick, so the two never drift apart.
//!
//! The music is built from 16-beat phrases, so the show changes pattern every
//! phrase: a chase round the bonus wheel, pop bumpers and train; everything
//! flashing; the two sides see-sawing; and the lamps filling up. The first beat
//! of each phrase also flashes the special.

use crate::lamps::{LampId, LampMode, Lamps};
use crate::music_beats::{ATTRACT_MUSIC_BEATS_MS, ATTRACT_MUSIC_MS};
use crate::playfield::lamp;
use crate::time::Instant;

/// Beats in a musical phrase.
const PHRASE: usize = 16;

/// Every lamp the show uses, so they can all be cleared afterwards. Bits in a
/// [`Pattern`] index this list.
const SHOW_LAMPS: [LampId; 28] = [
    lamp::BONUS_WHEEL[0],
    lamp::BONUS_WHEEL[1],
    lamp::BONUS_WHEEL[2],
    lamp::BONUS_WHEEL[3],
    lamp::BONUS_WHEEL[4],
    lamp::BONUS_WHEEL[5],
    lamp::BONUS_WHEEL[6],
    lamp::BONUS_WHEEL[7],
    lamp::BONUS_WHEEL[8],
    lamp::BONUS_WHEEL[9],
    lamp::BONUS_WHEEL_CENTRE,
    lamp::SPECIAL,
    lamp::THIRTY_POINTS[0],
    lamp::THIRTY_POINTS[1],
    lamp::POP_BUMPERS[0],
    lamp::POP_BUMPERS[1],
    lamp::POP_BUMPERS[2],
    lamp::POP_BUMPERS[3],
    lamp::POP_BUMPERS[4],
    lamp::PLAYER_UP[0],
    lamp::PLAYER_UP[1],
    lamp::PLAYER_UP[2],
    lamp::PLAYER_UP[3],
    lamp::BALL_IN_PLAY[0],
    lamp::BALL_IN_PLAY[1],
    lamp::BALL_IN_PLAY[2],
    lamp::BALL_IN_PLAY[3],
    lamp::BALL_IN_PLAY[4],
];

// Indexes into SHOW_LAMPS.
const WHEEL: usize = 0;
const CENTRE: usize = 10;
const SPECIAL: usize = 11;
const THIRTY: usize = 12;
const POPS: usize = 14;
const PLAYERS: usize = 19;
const TRAIN: usize = 23;
const TRAIN_CARS: usize = 5;

/// Which of [`SHOW_LAMPS`] are lit: bit `n` is `SHOW_LAMPS[n]`.
type Pattern = u32;

const fn bit(index: usize) -> Pattern {
    1 << index
}

/// Lamps on the centre line, which belong to neither side in the see-saw: the
/// bonus wheel's 10 (top) and 60 (bottom), the top-centre pop bumper and the
/// middle train car.
const CENTRE_LINE: Pattern = bit(WHEEL) | bit(WHEEL + 5) | bit(POPS + 1) | bit(TRAIN + 2);

/// Lamps on the left of the playfield and backboard, for the see-saw. The
/// bonus wheel runs clockwise from 10 at the top, so 70–100 are on the left.
const LEFT: Pattern = bit(WHEEL + 6)
    | bit(WHEEL + 7)
    | bit(WHEEL + 8)
    | bit(WHEEL + 9)
    | bit(THIRTY)
    | bit(POPS)
    | bit(POPS + 3)
    | bit(PLAYERS)
    | bit(PLAYERS + 2)
    | bit(TRAIN)
    | bit(TRAIN + 1);

const ALL: Pattern = (1 << SHOW_LAMPS.len()) - 1;

/// The lamps lit on beat `beat` of the music.
const fn pattern(beat: usize) -> Pattern {
    let step = beat % PHRASE;
    let mut lit = match (beat / PHRASE) % 4 {
        // Chase: two lamps opposite each other round the wheel, the pop
        // bumpers in turn, the train running along, the player numbers.
        0 => {
            bit(WHEEL + beat % 10)
                | bit(WHEEL + (beat + 5) % 10)
                | bit(POPS + beat % 5)
                | bit(TRAIN + beat % TRAIN_CARS)
                | bit(PLAYERS + beat % 4)
                | bit(THIRTY + beat % 2)
        }
        // Flash: everything on with the beat, off in between.
        1 => {
            if beat % 2 == 0 {
                ALL & !bit(SPECIAL)
            } else {
                0
            }
        }
        // See-saw: the left side, then the right.
        2 => {
            if beat % 2 == 0 {
                LEFT
            } else {
                ALL & !LEFT & !CENTRE_LINE & !bit(SPECIAL) & !bit(CENTRE)
            }
        }
        // Fill: the wheel, pops and train fill up through the phrase.
        _ => {
            let mut lit = 0;
            let mut n = 0;
            while n <= step * 10 / PHRASE {
                lit |= bit(WHEEL + n);
                n += 1;
            }
            let mut n = 0;
            while n <= step * 5 / PHRASE {
                lit |= bit(POPS + n);
                n += 1;
            }
            let mut n = 0;
            while n <= step * TRAIN_CARS / PHRASE {
                lit |= bit(TRAIN + n);
                n += 1;
            }
            lit
        }
    };
    // Accent the first beat of every phrase.
    if step == 0 {
        lit |= bit(SPECIAL) | bit(CENTRE);
    }
    lit
}

/// Runs the light show in time with the music.
#[derive(Clone, Copy, Debug, Default)]
pub struct AttractShow {
    /// When the music started; `None` when the show isn't running.
    started: Option<Instant>,
    /// The next beat to show.
    next_beat: usize,
}

impl AttractShow {
    /// A show that isn't running.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            started: None,
            next_beat: 0,
        }
    }

    /// Whether the show is running.
    #[must_use]
    pub const fn is_running(&self) -> bool {
        self.started.is_some()
    }

    /// Starts the show; the music must start in the same tick.
    pub fn start(&mut self, now: Instant) {
        self.started = Some(now);
        self.next_beat = 0;
    }

    /// Stops the show and turns its lamps off.
    pub fn stop(&mut self, lamps: &mut Lamps) {
        self.started = None;
        for id in SHOW_LAMPS {
            lamps.set(id, LampMode::Off);
        }
    }

    /// Shows any beats that have arrived. Returns `true` when a loop of the
    /// music has ended and it must start again now.
    pub fn tick(&mut self, now: Instant, lamps: &mut Lamps) -> bool {
        let Some(mut started) = self.started else {
            return false;
        };
        let looped = now.reached(started.plus(ATTRACT_MUSIC_MS));
        if looped {
            // From the loop's exact end, not `now`, so the timing never drifts.
            started = started.plus(ATTRACT_MUSIC_MS);
            self.started = Some(started);
            self.next_beat = 0;
        }
        let mut latest = None;
        while self.next_beat < ATTRACT_MUSIC_BEATS_MS.len()
            && now.reached(started.plus(ATTRACT_MUSIC_BEATS_MS[self.next_beat]))
        {
            latest = Some(self.next_beat);
            self.next_beat += 1;
        }
        if let Some(beat) = latest {
            let lit = pattern(beat);
            for (n, id) in SHOW_LAMPS.into_iter().enumerate() {
                let on = lit & bit(n) != 0;
                lamps.set(id, if on { LampMode::On } else { LampMode::Off });
            }
        }
        looped
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const fn at(ms: u32) -> Instant {
        Instant::from_millis(ms)
    }

    fn lit(lamps: &Lamps, now: Instant) -> u64 {
        lamps.frame(now)
    }

    #[test]
    fn beats_are_in_order_and_inside_the_music() {
        assert!(ATTRACT_MUSIC_BEATS_MS.windows(2).all(|w| w[0] < w[1]));
        assert!(
            ATTRACT_MUSIC_BEATS_MS
                .last()
                .is_some_and(|&b| b < ATTRACT_MUSIC_MS)
        );
        // A sensible tempo: 60–200 beats per minute.
        let first = ATTRACT_MUSIC_BEATS_MS[0];
        let last = *ATTRACT_MUSIC_BEATS_MS.last().unwrap();
        let mean = (last - first) / (ATTRACT_MUSIC_BEATS_MS.len() as u32 - 1);
        assert!((300..=1_000).contains(&mean), "mean beat {mean} ms");
    }

    #[test]
    fn the_lamps_change_on_the_beat() {
        let mut lamps = Lamps::new();
        let mut show = AttractShow::new();
        show.start(at(0));
        let first = ATTRACT_MUSIC_BEATS_MS[0];
        if first > 0 {
            show.tick(at(first - 1), &mut lamps);
            assert_eq!(
                lit(&lamps, at(first - 1)),
                0,
                "nothing before the first beat"
            );
        }
        show.tick(at(first), &mut lamps);
        let on_first = lit(&lamps, at(first));
        assert_ne!(on_first, 0);
        let second = ATTRACT_MUSIC_BEATS_MS[1];
        show.tick(at(second - 1), &mut lamps);
        assert_eq!(
            lit(&lamps, at(second - 1)),
            on_first,
            "no change between beats"
        );
        show.tick(at(second), &mut lamps);
        assert_ne!(lit(&lamps, at(second)), on_first, "a new pattern each beat");
    }

    #[test]
    fn every_pattern_lights_something_and_uses_only_show_lamps() {
        for beat in 0..ATTRACT_MUSIC_BEATS_MS.len() {
            let p = pattern(beat);
            assert_eq!(p & !ALL, 0, "beat {beat}");
            // The flash phrase goes dark on its off-beats; everything else lights up.
            if (beat / PHRASE) % 4 != 1 || beat % 2 == 0 {
                assert_ne!(p, 0, "beat {beat}");
            }
        }
    }

    #[test]
    fn the_see_saw_sides_are_mirror_images() {
        let right = ALL & !LEFT & !CENTRE_LINE & !bit(SPECIAL) & !bit(CENTRE);
        assert_eq!(LEFT & CENTRE_LINE, 0);
        assert_eq!(LEFT.count_ones(), right.count_ones());
    }

    #[test]
    fn the_show_loops_with_the_music() {
        let mut lamps = Lamps::new();
        let mut show = AttractShow::new();
        show.start(at(0));
        assert!(!show.tick(at(ATTRACT_MUSIC_BEATS_MS[3]), &mut lamps));
        // Even a tick a few ms late restarts from the loop's exact end.
        assert!(show.tick(at(ATTRACT_MUSIC_MS + 3), &mut lamps));
        assert!(show.is_running());
        assert!(!show.tick(at(ATTRACT_MUSIC_MS + 4), &mut lamps));
        assert!(show.tick(at(2 * ATTRACT_MUSIC_MS), &mut lamps));
    }

    #[test]
    fn stopping_clears_the_lamps() {
        let mut lamps = Lamps::new();
        let mut show = AttractShow::new();
        show.start(at(0));
        show.tick(at(ATTRACT_MUSIC_BEATS_MS[0]), &mut lamps);
        show.stop(&mut lamps);
        assert!(!show.is_running());
        assert_eq!(lit(&lamps, at(0)), 0);
    }
}
