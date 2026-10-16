//! Hardware-independent game logic for the 1956 Williams *Fun House* (IPDB #967)
//! rebuild.
//!
//! This crate follows a "functional core, imperative shell" design:
//!
//! * The firmware (the shell) owns all I/O. Roughly every millisecond it scans
//!   the switch matrix into a [`SwitchStates`], calls [`Machine::tick`], and
//!   shifts the returned [`Outputs`] out to the coil and lamp drivers.
//! * Everything in here is pure logic with no hardware access, so it builds on
//!   any target and is unit-tested on the host with a plain `cargo test`.
//!
//! The crate is `no_std` and allocation-free so it fits on an `ATmega2560`
//! (8 KiB of SRAM).

#![cfg_attr(not(test), no_std)]

pub mod bells;
pub mod coils;
pub mod game;
pub mod lamps;
pub mod playfield;
pub mod sound;
pub mod switches;
pub mod time;

pub use coils::{CoilId, CoilKind, Coils};
pub use game::{MAX_CREDITS, MAX_PLAYERS, Machine, Outputs, Phase, Settings};
pub use lamps::{LampId, LampMode, Lamps};
pub use sound::{Sound, SoundScheduler};
pub use switches::{Debouncer, SwitchEvent, SwitchId, SwitchStates};
pub use time::Instant;
