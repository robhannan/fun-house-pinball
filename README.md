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
There's no ball physics: click a part to roll a ball over its switch. The
backboard from `docs/backboard.svg` sits above the playfield, with each
player's score reels, the credit window, and the player-up and ball-in-play
lamps. Up to 4 players, and the operator settings (3, 4 or 5 balls; replay or
novelty) can be changed on the page.

It plays the machine's sounds too: mostly what a 1950s machine sounded like
(bells, relays, the score motor starting a game, the knocker), with circus
extras (drum roll, fanfare, slide whistles, sad trombone). They're cut from
free CC0 recordings, many from real electromechanical pinball machines
(credits in [docs/sound-credits.md](docs/sound-credits.md)). The simulator
also plays the noise each coil makes (kickers, pop bumpers, flippers), which
the real machine makes by itself.

In attract mode, "Entrance of the Gladiators" (Julius
Fučík's 1897 circus march, played by the United States Marine Band) plays in
the background, repeating from the top, and the playfield and backboard lamps
dance to its beats. It can be turned off in the operator settings. Synthesised
versions are kept in `sim/web/sounds.js` as a fallback, and the page can switch
between the two. The **Sound test** panel plays each one, including the
optional real bells.

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

### Sound on the machine

The machine plays the same sounds through a WAV Trigger, which mixes up to 14
at once. Render them to WAV files for its SD card with:

```sh
pnpm run render:sounds   # writes target/sd-card/001_Pop-bumper.wav, ...
```

Copy the files to the root of a FAT32 microSD card. To change which
recordings are used, edit `sim/samples/sources.json` and run
`pnpm run import:samples` (needs `ffmpeg`). Wiring is in
[docs/wiring-guide.md](docs/wiring-guide.md#7d-sound-wav-trigger).

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

### Test mode

For bringing the machine up one part at a time. Press the **test button**
(inside the coin door) in attract mode, then press it again to move on:

1. **Switches:** close any switch to hear a bell, light that part's lamp, and
   see which switch it was.
2. **Coils:** the flipper buttons choose a coil, and Start fires it once at its
   normal pulse length. Nothing else can fire.
3. **Lamps:** the flipper buttons light the lamps one at a time, and Start
   lights them all.

A fourth press (or the slam tilt) goes back to attract mode. Each step prints a
line on the USB console that `cargo run` opens, such as
`switch C2 R3 closed: Pop bumper 4 (lower left)`. The simulator has the same
test button and shows the same lines.

`firmware/rust-toolchain.toml` pins the nightly that
[avr-hal](https://github.com/Rahix/avr-hal) currently supports. When bumping
avr-hal, bump the toolchain to match its `rust-toolchain.toml`.

## Next steps

- [ ] Check the features in `crates/pinball-core/src/playfield.rs` (taken from a
      photo) against the real playfield; counts marked *verify* are guesses
- [ ] Confirm the scoring and bonus-wheel rules against the original schematic
- [ ] Choose driver boards and confirm coil voltages and pulse times
- [ ] Decide on a score display (backglass lamps, reels or LED digits)
- [x] Sound: recorded effects, mixed by a WAV Trigger (wire and test on the machine)
