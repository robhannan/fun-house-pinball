# Shopping list

Everything needed to build the controller described in the
[wiring guide](wiring-guide.md), with quantities for the current playfield map
(`crates/pinball-core/src/playfield.rs`).

## Estimated cost

| | Estimate |
|---|---|
| **Everything** | **£580–1,065** |
| Without tools (if you already have them) | £510–915 |
| Electronics only (sections 1–8, no pinball parts or tools) | £270–485 |

These are rough **estimates in GBP, as of October 2026**, to help with budgeting.

- **Checked against current listings (converted from USD and EUR):**
  - Mean Well coil supply: about $33–46
  - IRL540N MOSFETs: about $1.60 each
  - flipper coils: about €13–16
- **Everything else** is a typical marketplace price range, not a quote.
- **Not included:** delivery, import fees and an electrician's time for the mains
  wiring.

AliExpress prices move a lot with sales and exchange rates, so check before
ordering.

## How to use this list

- **AliExpress links are searches, not specific listings.** Listings change and
  disappear all the time, so each link opens a search for the right part. Pick a
  seller with:
  - plenty of orders and a rating of 95% or more
  - recent reviews with photos
  - delivery to your country

  Allow 1–3 weeks for delivery.
- **Some parts should come from a reputable distributor instead** (marked
  🛡️), because fakes on marketplaces are common and a fake here is a safety
  risk:
  - **Power supplies:** they connect to mains.
  - **MOSFETs:** they switch the coils.
  - **Fuses:** they're what stops a fault becoming a fire.

  Those links go to [Mouser UK](https://www.mouser.co.uk); RS, Farnell or
  DigiKey are equally good.
- **Pinball-specific mechanical parts** (coils, flipper assemblies, rubbers,
  playfield switches, bulbs) are best bought from pinball specialists, who know
  what fits a 1950s Williams. Examples are [Pinball Life](https://www.pinballlife.com)
  and [Marco Specialties](https://www.marcospecialties.com).
- **Quantities include spares.** Cheap parts break and get lost, and a second
  order means another 2-week wait.

Count the playfield before ordering; parts marked *verify* in the code may change
the numbers.

---

## 1. Controller

| Item | Qty | Notes | Link | Est. £ |
|---|---|---|---|---|
| Arduino Mega 2560 R3 | 2 | One to build with, one spare. Clones with a CH340 USB chip work fine but may need the [CH340 driver](https://www.wch-ic.com/downloads/CH341SER_MAC_ZIP.html) on older macOS. | [AliExpress](https://www.aliexpress.com/w/wholesale-arduino-mega-2560-r3.html) | 16–30 |
| Mega screw-terminal shield | 1 | Lets you screw wires to the Arduino instead of soldering to it | [AliExpress](https://www.aliexpress.com/w/wholesale-mega-2560-screw-terminal-shield.html) | 5–10 |
| USB A-to-B cable | 1 | Often included with the board | [AliExpress](https://www.aliexpress.com/w/wholesale-usb-a-to-b-cable-arduino.html) | 1–3 |
| 9 V 1 A DC adaptor, 5.5 × 2.1 mm plug | 1 | Powers the Arduino in the machine. Get one with your country's plug. | [AliExpress](https://www.aliexpress.com/w/wholesale-9v-1a-power-adapter-5.5x2.1.html) | 3–6 |

**Section estimate: £25–49**

## 2. Bench testing (before anything goes in the machine)

| Item | Qty | Notes | Link | Est. £ |
|---|---|---|---|---|
| Breadboard and jumper wire kit | 1 | For building the first shift register and testing | [AliExpress](https://www.aliexpress.com/w/wholesale-breadboard-jumper-wire-kit.html) | 4–8 |
| 5 mm LED assortment | 1 pack | Test LEDs on outputs before any coil is connected | [AliExpress](https://www.aliexpress.com/w/wholesale-5mm-led-assortment-kit.html) | 2–4 |
| Metal-film resistor assortment (¼ W, 1%) | 1 kit | Covers the 220 Ω, 1 kΩ and 10 kΩ used throughout | [AliExpress](https://www.aliexpress.com/w/wholesale-metal-film-resistor-kit.html) | 3–6 |
| Tactile push buttons | 1 pack | Stand in for flipper and start buttons on the bench | [AliExpress](https://www.aliexpress.com/w/wholesale-tactile-push-button-switch-kit.html) | 2–3 |

**Section estimate: £11–21**

## 3. Switch matrix

The map has 25 matrix switches, including the three trapdoors in the
"Scores Bonus" box, plus 6 direct switches.

| Item | Qty | Notes | Link | Est. £ |
|---|---|---|---|---|
| 1N4148 signal diodes | 100 | One per matrix switch, plus plenty spare | [AliExpress](https://www.aliexpress.com/w/wholesale-1n4148-diode.html) | 1–2 |
| Playfield switches (leaf, rollover, pop bumper skirt) | As needed | Reuse the originals where they're good; buy matching replacements from a pinball specialist | Pinball specialist | varies |
| Flipper and start button leaf switches | As needed | As above | Pinball specialist | varies |
| Momentary push button, panel mount | 1 | The test button, inside the coin door | [AliExpress](https://www.aliexpress.com/w/wholesale-12mm-momentary-push-button.html) | 1–2 |

**Section estimate: £2–4, plus switches**

## 4. Output chain (74HC595 shift registers)

| Item | Qty | Notes | Link | Est. £ |
|---|---|---|---|---|
| 74HC595 shift register, DIP-16 | 20 | 12 used | [AliExpress](https://www.aliexpress.com/w/wholesale-74hc595-dip16.html) | 2–5 |
| DIP-16 IC sockets | 20 | So a dead chip can be swapped without soldering | [AliExpress](https://www.aliexpress.com/w/wholesale-dip16-ic-socket.html) | 1–3 |
| 100 nF ceramic capacitors ("104") | 100 | One per chip, plus spares | [AliExpress](https://www.aliexpress.com/w/wholesale-100nf-ceramic-capacitor-104.html) | 1–3 |
| Stripboard / prototype PCB | 10 | Mount the chips and drivers on these | [AliExpress](https://www.aliexpress.com/w/wholesale-stripboard-prototype-pcb.html) | 4–8 |
| 2.54 mm pin headers (male and female) | 1 pack each | Board-to-board connections | [AliExpress](https://www.aliexpress.com/w/wholesale-2.54mm-pin-header.html) | 3–6 |
| JST-XH connector kit with crimper | 1 | Unpluggable connections between boards and the playfield harness | [AliExpress](https://www.aliexpress.com/w/wholesale-jst-xh-connector-kit-crimper.html) | 8–15 |

**Section estimate: £19–40**

## 5. Coil drivers

20 coil outputs are used: ball serve, 6 kickers, 6 pop bumpers, 2 × 2 flipper
windings and 3 trapdoor flaps, plus 3 for the optional bells.

| Item | Qty | Notes | Link | Est. £ |
|---|---|---|---|---|
| 🛡️ IRL540N logic-level N-channel MOSFET | 25 | Must be **logic level** (the "L" in IRL). Rated 100 V, suitable for coil supplies up to about 50 V. | [Mouser](https://www.mouser.co.uk/c/?q=IRL540N) | 25–38 |
| 1N4004 diodes | 100 | Flyback diode, one per coil winding, unless the coil already has one | [AliExpress](https://www.aliexpress.com/w/wholesale-1n4004-diode.html) | 1–3 |
| 2-pin 5.08 mm PCB screw terminals | 40 | Coil wires to the driver board | [AliExpress](https://www.aliexpress.com/w/wholesale-5.08mm-pcb-screw-terminal-2-pin.html) | 3–6 |
| 220 Ω and 10 kΩ resistors | — | In the resistor kit from section 2 | — | — |

**Section estimate: £29–47**

**Easier alternative:** ready-made multi-channel MOSFET driver boards that take
5 V logic inputs replace the MOSFETs, resistors and screw terminals above. Check
that they're rated for your coil voltage and current, and that they include
flyback diodes or allow them to be fitted. Expect roughly £5–15 per 8-channel board.
[AliExpress](https://www.aliexpress.com/w/wholesale-mosfet-driver-module-8-channel.html)

## 6. Lamp drivers and lamps

38 lamp outputs are used, on chips 5–10: 27 on the playfield and 11 on the backboard.

| Item | Qty | Notes | Link | Est. £ |
|---|---|---|---|---|
| ULN2803A Darlington driver, DIP-18 | 8 | 6 used | [AliExpress](https://www.aliexpress.com/w/wholesale-uln2803a-dip18.html) | 3–6 |
| DIP-18 IC sockets | 8 | | [AliExpress](https://www.aliexpress.com/w/wholesale-dip18-ic-socket.html) | 1–2 |
| LED replacement bulbs, #44/#47 bayonet (BA9S), 6.3 V | 50 | Count the sockets on the playfield and backglass first. Warm white looks closest to the original incandescents. | [AliExpress](https://www.aliexpress.com/w/wholesale-ba9s-6.3v-led-pinball.html) | 12–25 |

**Section estimate: £16–33**

## 6b. Sound

A WAV Trigger plays the sound effects from a microSD card, mixing up to 14 at
once; see the [wiring guide](wiring-guide.md#7d-sound-wav-trigger).

| Item | Qty | Notes | Link | Est. £ |
|---|---|---|---|---|
| WAV Trigger (Robertsonics / SparkFun WIG-13660) | 1 | Polyphonic WAV player with a small on-board amplifier. About $61 at SparkFun; check stock, as some listings show backorders. | [SparkFun](https://www.sparkfun.com/wav-trigger.html) · [Pimoroni](https://shop.pimoroni.com/products/wav-trigger) | 50–70 |
| microSD card, up to 32 GB | 1 | Formatted FAT32 with 32 KB allocation. Branded cards are more reliable; fakes are common. | [AliExpress](https://www.aliexpress.com/w/wholesale-microsd-card-16gb.html) | 3–6 |
| Speaker, 4–8 Ω, 50–77 mm | 1 | On the WAV Trigger's speaker header | [AliExpress](https://www.aliexpress.com/w/wholesale-3w-4ohm-speaker.html) | 2–4 |
| 5 V 2 A DC adaptor | 1 | Powers the WAV Trigger separately from the Arduino | [AliExpress](https://www.aliexpress.com/w/wholesale-5v-2a-power-adapter.html) | 3–6 |
| 1 kΩ resistor | — | In the resistor kit from section 2 | — | — |

**Section estimate: £58–86**

## 7. Power

Choose the coil voltage **with your coil supplier before ordering a supply**.
The original 1950s coils were wound for AC. 48 V DC is a common choice for new
coils, and the IRL540N handles it.

| Item | Qty | Notes | Link | Est. £ |
|---|---|---|---|---|
| 🛡️ Coil supply: Mean Well LRS-350-48 (48 V, 350 W) | 1 | Only if your coils are specified for 48 V DC | [Mouser](https://www.mouser.co.uk/c/?q=LRS-350-48) | 35–50 |
| 🛡️ Lamp supply: Mean Well, matched to the bulbs | 1 | For 6.3 V bulbs, a supply adjustable to 6.3–7.5 V allows for the ULN2803's ~1 V drop | [Mouser](https://www.mouser.co.uk/c/?q=mean%20well%20LRS) | 12–20 |
| 🛡️ Slow-blow fuses, 5 × 20 mm, assorted | 1 set | One per supply rail; see the wiring guide for sizes | [Mouser](https://www.mouser.co.uk/c/?q=5x20%20slow%20blow%20fuse) | 5–10 |
| Panel-mount 5 × 20 mm fuse holders | 4 | | [AliExpress](https://www.aliexpress.com/w/wholesale-5x20mm-panel-fuse-holder.html) | 3–6 |
| Terminal block / bus bar | 2 | One for the common ground point, one for coil +V distribution | [AliExpress](https://www.aliexpress.com/w/wholesale-terminal-block-bus-bar.html) | 4–8 |
| DC-rated coil power switch | 1 | Cuts the coil supply only. Must be rated for **DC** at your coil voltage; many cheap switches are AC-only. | [AliExpress](https://www.aliexpress.com/w/wholesale-dc-rocker-switch-60v.html) | 3–8 |
| IEC mains inlet with fuse and switch | 1 | **Have an electrician fit and check this, and the earth bonding** | [AliExpress](https://www.aliexpress.com/w/wholesale-iec-c14-inlet-fuse-switch.html) | 3–6 |

**Section estimate: £65–108**

## 8. Wire, connectors and finishing

| Item | Qty | Notes | Link | Est. £ |
|---|---|---|---|---|
| 22 AWG stranded hook-up wire, 6+ colours | 1 set | Switches, logic and lamps | [AliExpress](https://www.aliexpress.com/w/wholesale-22awg-stranded-hook-up-wire-kit.html) | 10–18 |
| 18 AWG silicone wire, red and black | 10 m each | Coil power and coil ground returns | [AliExpress](https://www.aliexpress.com/w/wholesale-18awg-silicone-wire.html) | 10–20 |
| Insulated crimp terminal kit (spade, ring) | 1 | | [AliExpress](https://www.aliexpress.com/w/wholesale-insulated-crimp-terminal-kit.html) | 6–12 |
| Ratcheting crimper for insulated terminals | 1 | | [AliExpress](https://www.aliexpress.com/w/wholesale-ratchet-crimping-tool-insulated-terminals.html) | 12–25 |
| Heat-shrink tubing assortment | 1 | | [AliExpress](https://www.aliexpress.com/w/wholesale-heat-shrink-tubing-assortment.html) | 4–8 |
| Cable ties and adhesive mounts | 1 pack each | | [AliExpress](https://www.aliexpress.com/w/wholesale-cable-tie-mounts.html) | 4–8 |
| Wire marker labels | 1 pack | Label both ends of every wire | [AliExpress](https://www.aliexpress.com/w/wholesale-wire-marker-labels.html) | 3–6 |

**Section estimate: £49–97**

## 9. Tools

| Item | Qty | Notes | Link | Est. £ |
|---|---|---|---|---|
| Digital multimeter with continuity and diode test | 1 | A UNI-T is a good budget choice | [AliExpress](https://www.aliexpress.com/w/wholesale-uni-t-multimeter.html) | 15–35 |
| Temperature-controlled soldering station | 1 | | [AliExpress](https://www.aliexpress.com/w/wholesale-temperature-controlled-soldering-station.html) | 25–60 |
| Solder, 0.8 mm rosin core | 1 reel | | [AliExpress](https://www.aliexpress.com/w/wholesale-0.8mm-rosin-core-solder.html) | 5–10 |
| Flux pen and desoldering braid | 1 each | | [AliExpress](https://www.aliexpress.com/w/wholesale-flux-pen-desoldering-braid.html) | 4–8 |
| Automatic wire strippers | 1 | | [AliExpress](https://www.aliexpress.com/w/wholesale-automatic-wire-stripper.html) | 8–15 |
| Flush cutters | 1 | | [AliExpress](https://www.aliexpress.com/w/wholesale-flush-cutters.html) | 3–6 |
| Helping hands with magnifier | 1 | | [AliExpress](https://www.aliexpress.com/w/wholesale-helping-hands-soldering.html) | 6–12 |
| Safety glasses | 1 | | [AliExpress](https://www.aliexpress.com/w/wholesale-safety-glasses.html) | 2–5 |

**Section estimate: £68–151**

## 10. From a pinball specialist

Order these once the playfield has been surveyed. Give the supplier the game
(1956 Williams *Fun House*), your coil supply voltage, and photos or measurements
of the old parts.

| Item | Qty | Notes | Est. £ |
|---|---|---|---|
| Kicker coils | 6 | DC, at your coil voltage | 60–90 |
| Pop bumper coils | 6 | DC, at your coil voltage | 60–90 |
| Ball serve coil and mechanism | 1 | Only if replacing the manual ball lift | 25–60 |
| Dual-wound flipper coils | 2 | DC, at your coil voltage; ask for the lug layout | 22–30 |
| Trapdoor flap coils | 3 | **Continuous-duty**, DC, at your coil voltage | 30–60 |
| *Optional:* bells with coils | 0–3 | Not needed now. For the authentic 1950s scoring sound later (wiring guide section 7e). | varies |
| Flipper rebuild kits, rubber rings, pop bumper caps | As needed | | 40–100 |

**Section estimate: £237–430**
