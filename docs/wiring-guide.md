# Wiring guide

A step-by-step guide to wiring the Fun House controller, written for someone
new to electronics. Read it all once before buying parts or picking up a
soldering iron.

- [docs/hardware.md](hardware.md) has the design at a glance.
- `crates/pinball-core/src/playfield.rs` is the source of truth for which part
  goes where. If this guide and that file disagree, the code wins. Please fix
  the guide.

## Playfield map

Every playfield part with its switch matrix position (`C2·R0` = column 2,
row 0), coil number and lamp number. Use it alongside the tables in sections 6
and 7.

![Fun House playfield wiring reference](playfield.svg)

## Backboard map

The backglass lamps (player up, ball in play, game over, tilt), each player's
three score reels, and the credit window. The score display hardware isn't
chosen yet (issue #28).

![Fun House backboard wiring reference](backboard.svg)

---

## 1. Safety first

A pinball machine combines **mains electricity**, **coil supplies strong enough
to burn**, and **coils that get hot enough to scorch wood** if left on.

1. **Mains (230 V / 110 V) is not a learning project.** Buy ready-made, enclosed
   DC power supplies and have a qualified electrician check the mains wiring:
   - the inlet
   - the fuse
   - the power switch
   - the earth bond to the cabinet and any metal parts

   Never open a power supply.
2. **Never wire anything while it's powered.** Unplug the machine from the wall,
   not just at its switch. Then wait a minute for the power supply capacitors to
   drain.
3. **Fuse every supply rail**, as close to the supply as you can (see section 4).
4. **Build and test in stages.** Test with LEDs before any coil is connected, and
   bring up one coil at a time (see section 9).
5. **Keep a hand on the plug** the first time you power anything new. If you smell
   burning, see smoke, or a coil stays on, pull it.
6. **Work in a dry space**, wear safety glasses when soldering or clipping wire,
   and keep the soldering iron in its stand.

---

## 2. A five-minute electronics primer

You only need a handful of ideas.

| Term | What it means here |
|---|---|
| **Voltage (V)** | Electrical "pressure". The Arduino logic runs at **5 V**; coils run at a much higher voltage. |
| **Current (A)** | How much flows. Coils draw several amps for a split second; logic draws a few thousandths of an amp (mA). |
| **Ground (GND)** | The 0 V reference everything is measured against. **All DC supplies must share a common ground** or nothing works. |
| **Circuit** | Current needs a loop: out from the supply, through the part, back to ground. A switch breaks or completes that loop. |
| **Pull-up resistor** | Gently holds a pin at 5 V (reads "high") when nothing is connected. A closed switch to ground pulls it low. The Arduino has these built in. |
| **Active low** | "On" means 0 V. All our switch inputs work this way: pressed or closed reads low. |
| **Diode** | A one-way valve for current. The end with the **painted band is the cathode**: current flows *towards* the band. |
| **MOSFET** | An electrically controlled switch. A tiny 5 V signal from the Arduino side switches a big coil current on and off. |
| **Flyback diode** | A coil kicks back a high-voltage spike when switched off. A diode across the coil soaks it up. Without one, the MOSFET dies. |
| **Shift register (74HC595)** | A chip that turns 3 Arduino wires into 8 outputs. Chain 12 of them for 96 outputs. |

**Using a multimeter:** you'll use it constantly. Learn three settings:

- **Continuity (beep):** checks that a wire or closed switch connects two points.
- **Diode test:** shows which way round a diode is.
- **DC volts:** checks that a supply gives what it should.

---

## 3. Shopping list

The full list, with quantities and links, is in
[shopping-list.md](shopping-list.md). Three rules matter most:

- **MOSFETs must be "logic level"** (fully on with 5 V on the gate) and rated for
  at least twice your coil voltage, e.g. the IRL540N.
- **Choose the coil supply voltage together with your replacement coils.** The
  original 1956 coils were wound for AC and shouldn't be reused on DC.
- **Buy power supplies, MOSFETs and fuses from a reputable distributor**, not a
  marketplace: fakes are common, and these are the safety-critical parts.

---

## 4. Power layout

```
            ┌───────────────────────────────┐
 Mains ───▶ │      Inlet, fuse, switch      │  ◀── have this checked by an electrician
            └──┬─────────┬──────┬───────┬───┘
               │         │      │       │
        ┌──────▼───┐ ┌───▼────┐ ┌▼──────┐ ┌▼────────┐
        │ Coil PSU │ │Lamp PSU│ │ 9 V   │ │ 5 V 2 A │
        │ (DC)     │ │ (DC)   │ │Arduino│ │ sound   │
        └─┬──────┬─┘ └─┬────┬─┘ └─┬───┬─┘ └─┬─────┬─┘
        +V│  fuse│-  +V│fuse│-   +│   │-   +│fuse │-
          ▼      │     ▼    │     ▼   │     ▼     │
       coils     │   lamps  │  Arduino│  DFPlayer │
                 │          │  barrel │   VCC     │
                 ▼          ▼  jack   ▼           ▼
              ══════════ COMMON GROUND POINT ══════════
```

Every supply, the 5 V sound adaptor included, is fed from the machine's
**switched mains inlet**, so the power switch turns everything off at once.
**Have the electrician fit all of them.** Don't splice an adaptor into the mains
wiring yourself. A plug-in adaptor can go in an enclosed multi-socket that the
electrician wires to the inlet.

- **Common ground:** run every DC supply's negative (–) terminal, and the
  Arduino's GND, to **one** ground point, such as a terminal block. Without a
  shared ground the Arduino's 5 V signals mean nothing to the drivers.
- **Fuses:** put one in each positive (+) rail, right at the supply. As a
  starting point:
  - coil rail: a slow-blow fuse rated for the supply, e.g. 5 A
  - lamp rail: about 25% above total lamp current
  - sound rail (5 V): 2 A
- **Arduino power:** use the barrel jack (7–12 V; 9 V is ideal). Its 5 V pin can
  power the 74HC595 chips, which draw very little, but **nothing else**.
- **Sound power:** the DFPlayer Mini has its own 5 V supply (section 7d). Only
  that supply's negative (–) goes to the common ground point; never join its
  +5 V to the Arduino's 5V pin.
- **Coil power off switch:** fit an easy-to-reach switch or interlock that cuts
  the coil supply only. You can then test switches and lamps with coils dead.

---

## 5. Wiring the Arduino

Fit the screw-terminal shield and wire to its terminals. Pin names are printed
on the board (`22`, `23`, ... and `GND`, `5V`).

| Arduino pin | Goes to | Section |
|---|---|---|
| 22–29 | Switch matrix **columns** 0–7 (D22 = column 0) | 6 |
| 30–37 | Switch matrix **rows** 0–7 (D30 = row 0) | 6 |
| 38–45 | Direct switches 0–7 (D38 = direct 0) | 6 |
| 51 (MOSI) | 74HC595 #1 pin 14 (data in) | 7 |
| 52 (SCK) | Every 74HC595 pin 11 (shift clock) | 7 |
| 49 | Every 74HC595 pin 12 (latch) | 7 |
| 48 | Every 74HC595 pin 13 (output enable) + 10 kΩ pull-up to 5 V | 7 |
| 5V | Every 74HC595 pin 16 and pin 10 | 7 |
| GND | Common ground point, every 74HC595 pin 8, every ULN2803 pin 9 | 4, 7 |
| 18 (TX1) | DFPlayer Mini RX (pin 2), **through a 1 kΩ resistor** | 7d |
| 19 (RX1) | DFPlayer Mini TX (pin 3); optional, for future status messages | 7d |
| USB | Your computer, for flashing and the serial console | 9 |

Leave pins 50 and 53 unconnected; the SPI hardware uses them.

---

## 6. Switches

### Direct switches (cabinet buttons and tilts)

These are the simplest. Wire **one side of the switch to its Arduino pin** and the
**other side to GND**. No diode or resistor is needed: the Arduino's internal
pull-up does the rest.

```
 Arduino pin 38 ───────o  o─────── GND
                     (switch)
```

| Direct input | Arduino pin | Switch |
|---|---|---|
| 0 | 38 | Left flipper button |
| 1 | 39 | Right flipper button |
| 2 | 40 | Start button |
| 3 | 41 | Tilt (plumb bob) |
| 4 | 42 | Slam tilt |
| 5–7 | 43–45 | Spare |

### The switch matrix (playfield switches)

There are more playfield switches than pins, so they're arranged in a grid.
The firmware pulls one **column** wire low at a time and checks which **row**
wires go low with it. Each switch sits where its column and row cross.

**Every matrix switch needs its own 1N4148 diode.** Without diodes, two closed
switches can make a third, open one look closed ("ghosting").

Wire each matrix switch like this:

```
 Row wire ──────o  o──────▶|────── Column wire
 (D30–D37)    (switch)   diode     (D22–D29)
                    anode   band (cathode)
                         towards the column
```

**Diode direction matters:**

- The **band goes towards the column wire**.
- Fit the diode right at the switch, soldered to one of its lugs.
- Check every diode with the multimeter's diode test before closing up. It should
  read about 0.6 V one way and nothing the other way.

A practical way to wire it:

- **Columns:** run one wire per column around the playfield, daisy-chained to
  every switch in that column (to the diode's band end).
- **Rows:** do the same for each row (to the switch's other lug).
- **Labels:** label both ends of every wire, e.g. `C2` or `R4`.

Current assignments, from `playfield.rs`:

| Switch | Column (pin) | Row (pin) |
|---|---|---|
| Outhole | 0 (D22) | 0 (D30) |
| Shooter lane | 0 (D22) | 1 (D31) |
| Left kicker | 1 (D23) | 0 (D30) |
| Right kicker | 1 (D23) | 1 (D31) |
| Pop bumper: top left | 2 (D24) | 0 (D30) |
| Pop bumper: top centre | 2 (D24) | 1 (D31) |
| Pop bumper: top right | 2 (D24) | 2 (D32) |
| Pop bumper: lower left | 2 (D24) | 3 (D33) |
| Pop bumper: lower right | 2 (D24) | 4 (D34) |
| Top lanes 1–4 (left to right) | 3 (D25) | 0–3 (D30–D33) |
| Top centre red bumper | 3 (D25) | 4 (D34) |
| Side "10 points" lanes 1–4 (left to right) | 4 (D26) | 0–3 (D30–D33) |
| "Scores Bonus" gate | 4 (D26) | 4 (D34) |
| Trapdoors 1–3 under the gate (left to right) | 4 (D26) | 5–7 (D35–D37) |
| "30 points" rollover: left | 5 (D27) | 0 (D30) |
| "30 points" rollover: right | 5 (D27) | 1 (D31) |

Columns 6–7 and any unused positions are free for new switches. Add them to
`playfield.rs` and this table together.

**Testing a matrix switch with power off:** set the multimeter to diode test.
Put the red probe on the row wire and the black probe on the column wire. With
the switch closed it should read about 0.6 V; with it open, nothing.

---

## 7. Outputs: shift registers, coils and lamps

### 7a. The 74HC595 chain

Twelve 74HC595 chips are chained so that 3 Arduino wires drive 96 outputs.

- **Chips 1–4** (nearest the Arduino) drive coils.
- **Chips 5–12** drive lamps.

The 74HC595 pinout (notch at the top, pin 1 top-left, counting down the left
side and back up the right side):

```
            ┌───U───┐
       Q1  1│       │16 VCC (5 V)
       Q2  2│       │15 Q0
       Q3  3│       │14 DS (data in)
       Q4  4│  595  │13 OE (output enable, active low)
       Q5  5│       │12 ST_CP (latch)
       Q6  6│       │11 SH_CP (shift clock)
       Q7  7│       │10 MR (reset, active low)
      GND  8│       │ 9 Q7' (data out to next chip)
            └───────┘
```

Wire **every** chip the same way, except for data:

| 595 pin | Connect to |
|---|---|
| 16 (VCC) | 5 V, plus a **100 nF capacitor from pin 16 to pin 8**, as close to the chip as possible |
| 8 (GND) | Ground |
| 10 (MR) | 5 V (never reset) |
| 11 (SH_CP) | Arduino D52 (all chips share it) |
| 12 (ST_CP) | Arduino D49 (all chips share it) |
| 13 (OE) | Arduino D48 (all chips share it) **and a 10 kΩ resistor from this line to 5 V** |
| 14 (DS) | Chip 1: Arduino D51. Chip *N*: pin 9 of chip *N–1* |
| 9 (Q7') | Pin 14 of the next chip (unconnected on chip 12) |

**The 10 kΩ pull-up on OE is a safety feature, so don't skip it.** It holds every
output **off** whenever the Arduino isn't actively driving pin 48: at power-on,
while flashing, and during a reset. The firmware only enables outputs after
clearing them.

**Which output is which:** output Qn on chip *k* drives:

- **Chips 1–4:** coil number `8 × (k − 1) + n`
- **Chips 5–12:** lamp number `8 × (k − 5) + n`

So Q0 on chip 1 is coil 0, and Q3 on chip 6 is lamp 11.

### 7b. Coil drivers

One MOSFET circuit per coil output (or per winding, for flippers):

```
                           Coil +V (fused)
                               │
                        ┌──────┤
                 band ─▶│      █ coil
           1N4004 diode ▲      █
          (band to +V)  │      █
                        └──────┤
                               │
 595 Qn ──[220 Ω]──┬──────── Gate ┐
                   │              │ MOSFET (e.g. IRL540N)
                [10 kΩ]  Drain ───┘  (drain = coil side)
                   │     Source ──┐
                  GND             │
                                 GND (common ground)
```

In words:

1. **Coil supply:** coil supply + (through its fuse) goes to one coil lug.
2. **Coil to MOSFET:** the other coil lug goes to the MOSFET's **drain**.
3. **MOSFET to ground:** the MOSFET's **source** goes to ground. Use thick 18 AWG
   wire back to the common ground.
4. **Gate:** the 595 output goes through a **220 Ω** resistor to the MOSFET's
   **gate**.
5. **Gate pull-down:** a **10 kΩ** resistor from the gate to ground keeps the
   MOSFET off if a wire comes loose.
6. **Flyback diode:** fit a **1N4004 directly across the coil lugs**, with the
   **band towards the + supply side**. Many replacement coils already have one;
   check before adding another. If a diode is in backwards, the coil shorts the
   supply and the fuse blows.

For the IRL540N (TO-220 package), facing the printed side with the legs down,
the legs are **Gate, Drain, Source** from left to right. Check the datasheet for
any other part.

**Flippers** use two windings: a strong **power** winding for the flip and a weak
**hold** winding to keep the flipper up. A dual-wound flipper coil usually has
three lugs. Wire:

- the **common lug** to + supply
- each of the other two lugs to **its own MOSFET**, with a flyback diode across
  each winding

Confirm which lug is which with the coil's datasheet or supplier. The power
winding has the *lower* resistance; measure it with the multimeter's Ω setting,
power off.

**Trapdoor flaps** (the three trapdoors under the "Scores Bonus" gate) use the
same single-coil MOSFET circuit. Unlike a kicker, a flap can be **held open for
a long time**, so:

- Each flap coil must be rated for **continuous duty** at your coil supply
  voltage. A normal kicker coil held on will overheat and burn out.
- Check that the flap is closed when its coil has no power, so that it fails
  safe. The firmware assumes energised = open (*verify* on the real mechanism).
- After it has been held open for a few minutes, check by touch that the coil
  is no more than warm.

| Coil | Number | Chip | Output (595 pin) |
|---|---|---|---|
| Ball serve | 0 | 1 | Q0 (15) |
| Left kicker | 1 | 1 | Q1 (1) |
| Right kicker | 2 | 1 | Q2 (2) |
| Pop bumpers 1–5 (top left, top centre, top right, lower left, lower right) | 3–7 | 1 | Q3–Q7 (3–7) |
| Left flipper power | 8 | 2 | Q0 (15) |
| Left flipper hold | 9 | 2 | Q1 (1) |
| Right flipper power | 10 | 2 | Q2 (2) |
| Right flipper hold | 11 | 2 | Q3 (3) |
| Trapdoor flaps 1–3 (left to right) | 12–14 | 2 | Q4–Q6 (4–6) |

### 7c. Lamp drivers

Each ULN2803A drives 8 lamps. It connects a lamp's negative side to ground when
its input is high.

```
            ┌───U───┐
      IN1  1│       │18 OUT1
      IN2  2│       │17 OUT2
       ...  │ ULN   │ ...
      IN8  8│ 2803A │11 OUT8
      GND  9│       │10 COM
            └───────┘
```

- **INx (pins 1–8):** Q0–Q7 of the matching 595 (Q0 → IN1, Q1 → IN2, and so on)
- **GND (pin 9):** ground
- **COM (pin 10):** lamp supply +
- **OUTx (pins 18 down to 11):** the lamp's negative side. OUT1 is pin 18 and
  OUT8 is pin 11.
- **Lamp's other side:** lamp supply + (fused)

The ULN2803A drops about 1 V, so 6.3 V LED bulbs may look slightly dim. If they
do, raise the lamp supply a little, staying within the bulbs' rated voltage.
Each chip handles up to 500 mA per lamp but less in total, which is plenty for
LEDs.

| Lamp | Number | Chip | Output |
|---|---|---|---|
| Game over | 0 | 5 | Q0 |
| Tilt | 1 | 5 | Q1 |
| Special when lit | 2 | 5 | Q2 |
| Bonus wheel 10–80 | 8–15 | 6 | Q0–Q7 |
| Bonus wheel 90, 100 | 16–17 | 7 | Q0–Q1 |
| Bonus wheel centre | 18 | 7 | Q2 |
| "30 points" left, right | 20–21 | 7 | Q4–Q5 |
| Pop bumper caps 1–5 | 24–28 | 8 | Q0–Q4 |
| Backboard: player up 1–4 | 32–35 | 9 | Q0–Q3 |
| Backboard: ball in play 1–5 (four train cars, then the dragon's head *) | 40–44 | 10 | Q0–Q4 |

### 7d. Sound (DFPlayer Mini)

Sound comes from a **DFPlayer Mini**, a small module that plays numbered WAV or
MP3 files from a microSD card and drives a speaker directly. The Arduino tells it
which file to play over a serial connection.

The DFPlayer Mini pinout (pins 1–8 down the left, 9–16 up the right, with the
SD card slot at the top):

```
            ┌──[ SD ]──┐
      VCC  1│          │16 BUSY
       RX  2│          │15 USB-
       TX  3│ DFPlayer │14 USB+
    DAC_R  4│   Mini   │13 ADKEY2
    DAC_L  5│          │12 ADKEY1
     SPK1  6│          │11 IO2
      GND  7│          │10 GND
     SPK2  8│          │ 9 IO1
            └──────────┘
```

| DFPlayer pin | Connect to |
|---|---|
| 1 (VCC) | **A separate 5 V supply** (a 5 V 2 A adaptor), not the Arduino's 5V pin: the module can draw several hundred mA on loud sounds |
| 7 (GND) | The common ground point |
| 2 (RX) | Arduino **D18 (TX1)**, through a **1 kΩ resistor** (protects the module's 3.3 V input, and cuts hiss) |
| 3 (TX) | Arduino D19 (RX1); optional |
| 6 (SPK1), 8 (SPK2) | The speaker, 3 W at 4–8 Ω. **Don't connect either speaker pin to ground.** |

Everything else stays unconnected.

**The SD card:**

1. Run `pnpm run render:sounds`. This writes every sound to
   `target/sd-card/01/` as `001.wav`, `002.wav` and so on.
2. Format a microSD card (8–32 GB) as **FAT32**.
3. Copy the whole `01` folder to the card's root, so the files are at
   `/01/001.wav`.
4. Put the card in the DFPlayer.

Sounds are synthesised in `sim/web/sounds.js`, the same file the simulator
plays, so what you hear in the simulator is what the machine plays. After
changing a sound, re-render and re-copy the card.

The firmware waits 2 seconds after power-on for the module to start, then sets
the volume (`DFPLAYER_VOLUME` in `crates/pinball-core/src/sound.rs`, 0–30).

---

## 8. Good wiring habits

- **Sizes:** use **18 AWG** for coil power and coil ground returns, and **22 AWG**
  for switches, logic and lamps.
- **Colours:** use one colour per job and stick to it, e.g. red for coil +,
  black for ground, one colour for columns and another for rows.
- **Labels:** label both ends of every wire.
- **Logic vs coil wiring:** keep the Arduino and 595 wiring away from coil power
  runs, and cross them at right angles where you can.
- **Strain relief:** tie wires down with cable ties near each connection so that
  vibration can't work them loose.
- **Insulation:** heat-shrink every solder joint. Don't leave bare metal.
- **Order of work:** wire one assembly, test it, and only then move on.

---

## 9. Bring-up: the order to do things in

Each step only adds one new thing, so when something goes wrong you know where
to look. **Keep the coil supply switched off until step 6.**

1. **Arduino alone, over USB.** Flash the firmware with `cd firmware && cargo run --release`.
   The console should print `Fun House controller ready`.
2. **Direct switches.** Wire the start and flipper buttons (section 6).
   Check each with the multimeter: continuity to ground when pressed.
3. **One shift register and test LEDs.** Build chip 1 only, on a breadboard. Put
   an LED with a 1 kΩ resistor from Q0, Q1 and Q2 to ground:
   - Press start: Q0 (ball serve) should give a very short blink.
   - Q1 and Q2 should stay dark.

   Then build chip 2 the same way, with test LEDs on Q0–Q3:
   - Hold the left flipper button: Q0 (power) blinks and Q1 (hold) stays lit
     until you let go.

   Nothing should light at power-on.
4. **The rest of the chain.** Add the other chips one at a time, checking that
   the test LEDs still behave and that nothing lights at power-on.
5. **Matrix switches.** Wire and diode-test each one with power off (section 6).
6. **The first coil.** Connect **one** MOSFET circuit and **one** coil, ideally a
   kicker. Switch on the coil supply with a hand near the plug. Close the kicker
   switch by hand: you should get one short, sharp kick. If the coil buzzes or
   stays on, cut the power and recheck the flyback diode and MOSFET wiring.
7. **The remaining coils, one by one**, then the lamps.
8. **Sound.** Wire the DFPlayer Mini and its speaker, and prepare the SD card
   (section 7d). Power on and wait 2 seconds, then press start: you should hear
   the calliope march.

---

## 10. Troubleshooting

| Symptom | Likely cause |
|---|---|
| Nothing works, but the Arduino's power LED is on | Grounds not joined: check the common ground point |
| All outputs on or random at power-on | OE pull-up missing, or D48 not wired |
| Outputs shifted along by one (the wrong lamp or coil fires) | Data chain wiring (pin 9 → pin 14) wrong or loose, or a long noisy clock wire |
| A switch reads closed when it isn't | Diode backwards, missing, or shorted; or a pinched wire touching another |
| Pressing one switch also shows another one | Missing diode (ghosting) |
| Coil buzzes, gets hot, or the fuse blows | Flyback diode backwards, MOSFET shorted (replace it), or a wiring short |
| Coil fires weakly | Supply voltage too low for the coil, or thin wiring/bad crimps |
| MOSFET gets hot | Not a logic-level part, or gate resistor/pull-down wired wrong |
| Board resets when a coil fires | Coil ground return too thin, or shared with logic wiring; give coils their own thick ground run |
| LED bulbs dim | Lamp supply too low after the ULN2803's ~1 V drop |
| No sound | SD card not FAT32; folder not named `01`; files not named `001.wav` etc.; DFPlayer RX wired to D1 (TX0) instead of D18 (TX1) |
| Sound hisses or crackles | 1 kΩ resistor missing on the RX line, or the DFPlayer powered from the Arduino's 5V pin |

---

## Glossary

- **AWG**: American Wire Gauge. Smaller numbers mean thicker wire.
- **Cathode / anode**: a diode's two ends. Current flows from anode to cathode
  (towards the band).
- **DIP socket**: a socket the chip plugs into, so a dead chip can be swapped
  without soldering.
- **EM**: electromechanical. The original machine ran on relays and switches,
  with no computer.
- **Logic level**: a part that works fully with 5 V control signals.
- **Slow-blow fuse**: a fuse that tolerates short surges (like a coil firing) but
  blows on a sustained overload.
