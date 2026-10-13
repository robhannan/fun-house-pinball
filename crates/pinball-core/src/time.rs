//! Monotonic millisecond time, supplied by the firmware's timer interrupt.

/// A point in time, in milliseconds since power-on.
///
/// The underlying counter wraps after ~49.7 days; all comparisons here are
/// wrap-safe as long as the two instants are less than ~24 days apart.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Instant(u32);

impl Instant {
    /// Creates an instant from a raw millisecond count.
    #[must_use]
    pub const fn from_millis(ms: u32) -> Self {
        Self(ms)
    }

    /// The raw millisecond count.
    #[must_use]
    pub const fn as_millis(self) -> u32 {
        self.0
    }

    /// Milliseconds elapsed since `earlier`.
    #[must_use]
    pub const fn since(self, earlier: Self) -> u32 {
        self.0.wrapping_sub(earlier.0)
    }

    /// The instant `ms` milliseconds after `self`.
    #[must_use]
    pub const fn plus(self, ms: u32) -> Self {
        Self(self.0.wrapping_add(ms))
    }

    /// Whether `self` is at or after `deadline`.
    #[must_use]
    pub const fn reached(self, deadline: Self) -> bool {
        self.0.wrapping_sub(deadline.0) < (1 << 31)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deadlines_survive_counter_wrap() {
        let before_wrap = Instant::from_millis(u32::MAX - 5);
        let deadline = before_wrap.plus(10);
        assert!(!before_wrap.reached(deadline));
        assert!(!Instant::from_millis(3).reached(deadline));
        assert!(Instant::from_millis(4).reached(deadline));
        assert_eq!(Instant::from_millis(4).since(before_wrap), 10);
    }
}
