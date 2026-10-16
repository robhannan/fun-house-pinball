# fun-house-pinball

A Rust controller for a restored **1956 Williams *Fun House*** ([IPDB #967](https://www.ipdb.org/machine.cgi?id=967)).
The original electromechanical (EM) game is being rebuilt with new parts and
driven by an Arduino Mega 2560 instead of relays and stepper units.

> ⚠️ **Safety:** the playfield runs at coil voltage, and a coil left energised
> will burn out within seconds. Read the [wiring guide](docs/wiring-guide.md)
> before wiring anything up.

## Layout

```
.
├── crates/pinball-core   # no_std game logic: switches, coils, lamps, rules (host-testable)
├── crates/pinball-sim    # the game logic compiled to WebAssembly for the simulator
├── sim/                  # browser simulator page and build script
├── firmware              # Arduino Mega 2560 binary: I/O + main loop (AVR, nightly)
└── docs
    ├── hardware.md       # design at a glance: pin map, driver boards
    ├── wiring-guide.md   # step-by-step wiring for beginners
    ├── shopping-list.md  # parts, quantities and where to buy
    └── playfield.svg     # playfield map with switch/coil/lamp numbers
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

### Simulator (play it in a browser)

Try the rules without any hardware. The simulator runs the real `pinball-core`
logic compiled to WebAssembly, on the playfield map from `docs/playfield.svg`.
There's no ball physics: click a part to roll a ball over its switch.

```sh
pnpm install           # once, installs zx for the build script
pnpm run build:sim
python3 -m http.server -d sim/web 8000   # then open http://localhost:8000
```

Controls:

- <kbd>Enter</kbd>: start
- <kbd>Z</kbd> / <kbd>M</kbd>: hold the left / right flipper
- <kbd>T</kbd>: tilt
- <kbd>D</kbd>: drain

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
