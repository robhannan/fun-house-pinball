//! Switch identification, raw switch snapshots and debouncing.
//!
//! The controller reads an 8×8 switch matrix (64 switches) plus 8 directly
//! wired cabinet inputs (flipper buttons, start, tilt, ...).

/// Number of switches the controller can read: 8×8 matrix + 8 direct inputs.
pub const SWITCH_COUNT: usize = 64 + 8;

const BYTES: usize = SWITCH_COUNT.div_ceil(8);

/// Identifies one switch input.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SwitchId(u8);

impl SwitchId {
    /// A switch in the matrix at `column` (strobe line) and `row` (return line).
    ///
    /// # Panics
    ///
    /// If out of range. Used in a `const`, this is a compile-time error.
    #[must_use]
    pub const fn matrix(column: u8, row: u8) -> Self {
        assert!(column < 8 && row < 8, "matrix position out of range");
        Self(column * 8 + row)
    }

    /// One of the 8 directly wired (non-matrix) inputs.
    ///
    /// # Panics
    ///
    /// If out of range. Used in a `const`, this is a compile-time error.
    #[must_use]
    pub const fn direct(input: u8) -> Self {
        assert!(input < 8, "direct input out of range");
        Self(64 + input)
    }

    /// Dense index in `0..SWITCH_COUNT`.
    #[must_use]
    pub const fn index(self) -> usize {
        self.0 as usize
    }
}

/// A snapshot of every switch: one bit per switch, set when closed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SwitchStates([u8; BYTES]);

impl SwitchStates {
    /// All switches open.
    #[must_use]
    pub const fn new() -> Self {
        Self([0; BYTES])
    }

    /// Whether `id` is closed.
    #[must_use]
    pub const fn is_closed(&self, id: SwitchId) -> bool {
        self.0[id.index() / 8] & (1 << (id.index() % 8)) != 0
    }

    /// Sets a single switch.
    pub fn set(&mut self, id: SwitchId, closed: bool) {
        let mask = 1 << (id.index() % 8);
        if closed {
            self.0[id.index() / 8] |= mask;
        } else {
            self.0[id.index() / 8] &= !mask;
        }
    }

    /// Sets a whole matrix column at once; bit `n` of `rows` is row `n`.
    ///
    /// This matches how the firmware scans: strobe one column, read 8 rows.
    pub fn set_column(&mut self, column: u8, rows: u8) {
        self.0[usize::from(column)] = rows;
    }

    /// Sets all 8 direct inputs at once; bit `n` is direct input `n`.
    pub fn set_direct(&mut self, inputs: u8) {
        self.0[8] = inputs;
    }
}

/// A debounced change of switch state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SwitchEvent {
    /// The switch closed (ball arrived, button pressed, ...).
    Closed(SwitchId),
    /// The switch opened.
    Opened(SwitchId),
}

/// Filters contact bounce: a switch must read the same new state for
/// `threshold` consecutive scans before the change is reported.
#[derive(Clone, Debug)]
pub struct Debouncer {
    stable: SwitchStates,
    counts: [u8; SWITCH_COUNT],
    threshold: u8,
}

impl Debouncer {
    /// Creates a debouncer. With a 1 ms scan, a `threshold` of 2–3 suits
    /// leaf switches and buttons; optos may want more.
    #[must_use]
    pub const fn new(threshold: u8) -> Self {
        Self {
            stable: SwitchStates::new(),
            counts: [0; SWITCH_COUNT],
            threshold,
        }
    }

    /// The current debounced state of every switch.
    #[must_use]
    pub const fn stable(&self) -> &SwitchStates {
        &self.stable
    }

    /// Feeds one raw scan and returns the switches whose debounced state changed.
    pub fn update(&mut self, raw: &SwitchStates) -> SwitchEvents {
        let mut events = SwitchEvents::new();
        for index in 0..SWITCH_COUNT {
            let id = SwitchId(index as u8);
            let closed = raw.is_closed(id);
            if closed == self.stable.is_closed(id) {
                self.counts[index] = 0;
                continue;
            }
            self.counts[index] = self.counts[index].saturating_add(1);
            if self.counts[index] >= self.threshold {
                self.counts[index] = 0;
                self.stable.set(id, closed);
                events.push(if closed {
                    SwitchEvent::Closed(id)
                } else {
                    SwitchEvent::Opened(id)
                });
            }
        }
        events
    }
}

/// The events produced by one [`Debouncer::update`]. Each switch can change at
/// most once per scan, so this can never overflow.
#[derive(Clone, Debug)]
pub struct SwitchEvents {
    buf: [SwitchEvent; SWITCH_COUNT],
    len: u8,
    next: u8,
}

impl SwitchEvents {
    const fn new() -> Self {
        Self {
            buf: [SwitchEvent::Opened(SwitchId(0)); SWITCH_COUNT],
            len: 0,
            next: 0,
        }
    }

    fn push(&mut self, event: SwitchEvent) {
        self.buf[usize::from(self.len)] = event;
        self.len += 1;
    }
}

impl Iterator for SwitchEvents {
    type Item = SwitchEvent;

    fn next(&mut self) -> Option<SwitchEvent> {
        (self.next < self.len).then(|| {
            self.next += 1;
            self.buf[usize::from(self.next - 1)]
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SW: SwitchId = SwitchId::matrix(2, 5);

    #[test]
    fn change_is_reported_only_after_threshold_scans() {
        let mut debouncer = Debouncer::new(3);
        let mut raw = SwitchStates::new();
        raw.set(SW, true);

        assert_eq!(debouncer.update(&raw).count(), 0);
        assert_eq!(debouncer.update(&raw).count(), 0);
        assert_eq!(
            debouncer.update(&raw).collect::<Vec<_>>(),
            [SwitchEvent::Closed(SW)]
        );
        assert!(debouncer.stable().is_closed(SW));
    }

    #[test]
    fn bounce_resets_the_count() {
        let mut debouncer = Debouncer::new(2);
        let closed = {
            let mut s = SwitchStates::new();
            s.set(SW, true);
            s
        };
        let open = SwitchStates::new();

        for _ in 0..10 {
            assert_eq!(debouncer.update(&closed).count(), 0);
            assert_eq!(debouncer.update(&open).count(), 0);
        }
        assert!(!debouncer.stable().is_closed(SW));
    }

    #[test]
    fn column_and_direct_bits_map_to_ids() {
        let mut s = SwitchStates::new();
        s.set_column(2, 1 << 5);
        s.set_direct(1 << 3);
        assert!(s.is_closed(SW));
        assert!(s.is_closed(SwitchId::direct(3)));
        assert!(!s.is_closed(SwitchId::matrix(5, 2)));
    }
}
