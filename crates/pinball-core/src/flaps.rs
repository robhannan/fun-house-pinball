//! The three trapdoor flaps in the "Scores Bonus" box, each turned by a hobby
//! servo (an MG90S or similar) rather than a coil.
//!
//! The rules only decide whether each flap is open or closed. Each flap's
//! closed and open angles are a [`FlapTrim`], set in test mode's flaps stage so
//! a flap can be made to sit exactly flush, and saved by the firmware in the
//! Arduino's EEPROM.

/// How many flaps there are, left to right.
pub const FLAP_COUNT: usize = 3;
/// The largest servo angle, in degrees.
pub const MAX_ANGLE: u8 = 180;

/// Each flap's closed and open servo angles, in degrees.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FlapTrim {
    /// Where each flap sits flush with the playfield.
    pub closed: [u8; FLAP_COUNT],
    /// Where each flap hangs open, making a ramp into its hole.
    pub open: [u8; FLAP_COUNT],
}

impl Default for FlapTrim {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl FlapTrim {
    /// A starting point before anything is trimmed: closed at the middle of
    /// the servo's travel, open 45° from there.
    pub const DEFAULT: Self = Self {
        closed: [90; FLAP_COUNT],
        open: [45; FLAP_COUNT],
    };

    /// Bytes in the stored form: a marker, then the closed and open angles.
    pub const BYTES: usize = 1 + 2 * FLAP_COUNT;
    /// Marks stored trims, so blank EEPROM (all `0xFF`) reads as the default.
    const MARKER: u8 = 0xF1;

    /// The angle for flap `n` (0–2) open or closed. Out-of-range `n` gives 90°.
    #[must_use]
    pub fn angle(&self, n: usize, open: bool) -> u8 {
        let angles = if open { &self.open } else { &self.closed };
        angles.get(n).copied().unwrap_or(90)
    }

    /// Moves flap `n`'s open or closed angle by `degrees`, within 0–180°.
    pub fn nudge(&mut self, n: usize, open: bool, degrees: i8) {
        let angles = if open {
            &mut self.open
        } else {
            &mut self.closed
        };
        if let Some(angle) = angles.get_mut(n) {
            let moved = i16::from(*angle) + i16::from(degrees);
            *angle = u8::try_from(moved.clamp(0, i16::from(MAX_ANGLE))).unwrap_or(90);
        }
    }

    /// The stored form, for the EEPROM.
    #[must_use]
    pub fn to_bytes(self) -> [u8; Self::BYTES] {
        let mut bytes = [Self::MARKER; Self::BYTES];
        bytes[1..=FLAP_COUNT].copy_from_slice(&self.closed);
        bytes[1 + FLAP_COUNT..].copy_from_slice(&self.open);
        bytes
    }

    /// Reads the stored form. Anything that isn't a valid saved trim (blank
    /// EEPROM, or a corrupt angle) gives [`FlapTrim::DEFAULT`].
    #[must_use]
    pub fn from_bytes(bytes: &[u8; Self::BYTES]) -> Self {
        if bytes[0] != Self::MARKER || bytes[1..].iter().any(|&a| a > MAX_ANGLE) {
            return Self::DEFAULT;
        }
        let mut trim = Self::DEFAULT;
        trim.closed.copy_from_slice(&bytes[1..=FLAP_COUNT]);
        trim.open.copy_from_slice(&bytes[1 + FLAP_COUNT..]);
        trim
    }
}

/// The servo pulse width for `angle`, in microseconds: 500 µs at 0° to
/// 2500 µs at 180°, the usual range for MG90S and SG90 servos. Repeated every
/// 20 ms.
#[must_use]
pub const fn pulse_us(angle: u8) -> u16 {
    let angle = if angle > MAX_ANGLE { MAX_ANGLE } else { angle };
    500 + (angle as u16 * 100) / 9
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pulses_span_the_servo_range() {
        assert_eq!(pulse_us(0), 500);
        assert_eq!(pulse_us(90), 1_500);
        assert_eq!(pulse_us(180), 2_500);
        assert_eq!(pulse_us(255), 2_500, "clamped");
    }

    #[test]
    fn trims_survive_the_eeprom() {
        let mut trim = FlapTrim::DEFAULT;
        trim.nudge(1, false, 7);
        trim.nudge(2, true, -20);
        assert_eq!(FlapTrim::from_bytes(&trim.to_bytes()), trim);
        assert_eq!(trim.angle(1, false), 97);
        assert_eq!(trim.angle(2, true), 25);
    }

    #[test]
    fn blank_or_corrupt_eeprom_reads_as_the_default() {
        assert_eq!(
            FlapTrim::from_bytes(&[0xFF; FlapTrim::BYTES]),
            FlapTrim::DEFAULT
        );
        let mut bytes = FlapTrim::DEFAULT.to_bytes();
        bytes[3] = 200;
        assert_eq!(FlapTrim::from_bytes(&bytes), FlapTrim::DEFAULT);
    }

    #[test]
    fn nudging_stays_within_the_servo_range() {
        let mut trim = FlapTrim::DEFAULT;
        trim.nudge(0, true, -100);
        assert_eq!(trim.angle(0, true), 0);
        for _ in 0..3 {
            trim.nudge(0, false, 100);
        }
        assert_eq!(trim.angle(0, false), MAX_ANGLE);
    }
}
