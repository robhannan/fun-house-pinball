# fun-house-pinball

A Rust controller for a restored **1956 Williams *Fun House*** ([IPDB #967](https://www.ipdb.org/machine.cgi?id=967)).
The original electromechanical (EM) game is being rebuilt with new parts and
driven by an Arduino Mega 2560 instead of relays and stepper units.

> ⚠️ **Safety:** the playfield runs at coil voltage, and a coil left energised
> will burn out within seconds. Read [docs/hardware.md](docs/hardware.md) before
> wiring anything up.

## Layout

```
.
├── crates/pinball-core   # no_std game logic: switches, coils, lamps, rules (host-testable)
├── firmware              # Arduino Mega 2560 binary: I/O + main loop (AVR, nightly)
└── docs/hardware.md      # wiring, pin map, driver boards
```

The design is a **functional core with an imperative shell**. Every millisecond
the firmware scans the switches, hands them to `pinball_core::Machine::tick`,
and shifts the returned coil and lamp bitmaps out to the driver boards. All the
rules and safety limits live in `pinball-core`, so you can unit-test them on
your laptop without an Arduino.

## Getting started

### Game logic (any machine, stable Rust)

```sh
cargo test           # run the game-logic tests
cargo clippy --all-targets
```

### Firmware (Arduino Mega 2560)

You need the AVR GCC toolchain, `avrdude`, and `ravedude` (the flashing runner):

```sh
# macOS
xcode-select --install
brew tap osx-cross/avr
brew install avr-gcc avrdude

# Debian/Ubuntu
sudo apt install gcc-avr avr-libc avrdude

cargo +stable install --locked ravedude
```

Then plug in the Mega and run:

```sh
cd firmware
cargo run --release   # builds, flashes via ravedude, opens a 57600 baud console
```

If more than one serial port is present, set `RAVEDUDE_PORT=/dev/cu.usbmodemXXXX`.

`firmware/rust-toolchain.toml` pins the nightly that
[avr-hal](https://github.com/Rahix/avr-hal) currently supports. When bumping
avr-hal, bump the toolchain to match its `rust-toolchain.toml`.

## Next steps

- [ ] Check the features in `crates/pinball-core/src/playfield.rs` (taken from a
      photo) against the real playfield; counts marked *verify* are guesses
- [ ] Confirm the scoring and bonus-wheel rules against the original schematic
- [ ] Multi-player support (the instruction card prices per player)
- [ ] Choose driver boards and confirm coil voltages and pulse times
- [ ] Decide on a score display (backglass lamps, reels or LED digits)
- [ ] Add sound
