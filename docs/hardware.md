# Hardware

Planned wiring for the controller. New to electronics? Start with the
step-by-step [wiring guide](wiring-guide.md), which includes a
[playfield map](playfield.svg). `crates/pinball-core/src/playfield.rs` is the
source of truth for logical assignments and `firmware/src/main.rs` for pins. Keep
them all in step.

## Controller: Arduino Mega 2560

The Mega has 54 digital I/O pins and 8 KiB of SRAM, which is plenty for a 1950s
playfield, and the [avr-hal](https://github.com/Rahix/avr-hal) Rust support for it
is mature.

| Function | Pins | Notes |
|---|---|---|
| Switch matrix columns (strobes) | D22–D29 | Driven low one at a time |
| Switch matrix rows (returns) | D30–D37 | Internal pull-ups, active low |
| Direct switches | D38–D45 | Flipper buttons, start, tilt, slam tilt (active low) |
| Shift-register data / clock | D51 (MOSI), D52 (SCK) | Hardware SPI |
| Shift-register latch | D49 | |
| Shift-register output enable | D48 | **External 10k pull-up to 5V**, so outputs stay off at reset |
| SPI SS (unused, must be output) | D53 | |
| Serial console | D0/D1 (USB) | 57600 baud |
| Sound board (WAV Trigger) | D18 (TX1), D19 (RX1) | 57600 baud; TX1 goes through a 1 kΩ resistor to the board's RX |

## Switches

The original EM wiring had no matrix. In the new harness, each playfield switch
sits in an 8×8 matrix **with a 1N4148 diode in series** so that several switches
can be closed at once without ghosting. Flipper buttons, start and tilts are wired
directly so they never wait on a scan.

## Outputs

A daisy chain of 12 × 74HC595 shift registers:

1. Bytes 0–3 (nearest the Arduino): 32 coil outputs, each feeding a logic-level
   N-channel MOSFET with a flyback diode across the coil.
2. Bytes 4–11: 64 lamp outputs (LED replacements) via ULN2803 or MOSFET arrays.

## Safety

The protections are layered, so no single failure can leave a coil energised:

1. **Software limits:** `pinball-core` clamps every pulse to a per-coil maximum
   and enforces a recycle time (`COIL_KINDS` in `playfield.rs`).
2. **Watchdog:** if the main loop stalls for about 64 ms, the MCU resets.
3. **Fail-safe enable:** a pull-up holds the 595s' `OE` high, so at reset, during
   bootloading and on panic, every output is off.
4. **Hardware:** fuse each coil supply rail, and fit a physical coil-power
   interlock (for example, on the coin door).

The original 1950s coils were wound for AC. Use DC-rated replacement coils that
match your supply voltage.
