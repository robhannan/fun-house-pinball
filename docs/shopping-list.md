# Shopping list

Everything needed to build the controller described in the
[wiring guide](wiring-guide.md), with quantities for the current playfield map
(`crates/pinball-core/src/playfield.rs`).

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

| Item | Qty | Notes | Link |
|---|---|---|---|
| Arduino Mega 2560 R3 | 2 | One to build with, one spare. Clones with a CH340 USB chip work fine but may need the [CH340 driver](https://www.wch-ic.com/downloads/CH341SER_MAC_ZIP.html) on older macOS. | [AliExpress](https://www.aliexpress.com/w/wholesale-arduino-mega-2560-r3.html) |
| Mega screw-terminal shield | 1 | Lets you screw wires to the Arduino instead of soldering to it | [AliExpress](https://www.aliexpress.com/w/wholesale-mega-2560-screw-terminal-shield.html) |
| USB A-to-B cable | 1 | Often included with the board | [AliExpress](https://www.aliexpress.com/w/wholesale-usb-a-to-b-cable-arduino.html) |
| 9 V 1 A DC adaptor, 5.5 × 2.1 mm plug | 1 | Powers the Arduino in the machine. Get one with your country's plug. | [AliExpress](https://www.aliexpress.com/w/wholesale-9v-1a-power-adapter-5.5x2.1.html) |

## 2. Bench testing (before anything goes in the machine)

| Item | Qty | Notes | Link |
|---|---|---|---|
| Breadboard and jumper wire kit | 1 | For building the first shift register and testing | [AliExpress](https://www.aliexpress.com/w/wholesale-breadboard-jumper-wire-kit.html) |
| 5 mm LED assortment | 1 pack | Test LEDs on outputs before any coil is connected | [AliExpress](https://www.aliexpress.com/w/wholesale-5mm-led-assortment-kit.html) |
| Metal-film resistor assortment (¼ W, 1%) | 1 kit | Covers the 220 Ω, 1 kΩ and 10 kΩ used throughout | [AliExpress](https://www.aliexpress.com/w/wholesale-metal-film-resistor-kit.html) |
| Tactile push buttons | 1 pack | Stand in for flipper and start buttons on the bench | [AliExpress](https://www.aliexpress.com/w/wholesale-tactile-push-button-switch-kit.html) |

## 3. Switch matrix

The map has 21 matrix switches, 24 with the three trapdoors under the
"Scores Bonus" gate, plus 5 direct switches.

| Item | Qty | Notes | Link |
|---|---|---|---|
| 1N4148 signal diodes | 100 | One per matrix switch, plus plenty spare | [AliExpress](https://www.aliexpress.com/w/wholesale-1n4148-diode.html) |
| Playfield switches (leaf, rollover, pop bumper skirt) | As needed | Reuse the originals where they're good; buy matching replacements from a pinball specialist | Pinball specialist |
| Flipper and start button leaf switches | As needed | As above | Pinball specialist |

## 4. Output chain (74HC595 shift registers)

| Item | Qty | Notes | Link |
|---|---|---|---|
| 74HC595 shift register, DIP-16 | 20 | 12 used | [AliExpress](https://www.aliexpress.com/w/wholesale-74hc595-dip16.html) |
| DIP-16 IC sockets | 20 | So a dead chip can be swapped without soldering | [AliExpress](https://www.aliexpress.com/w/wholesale-dip16-ic-socket.html) |
| 100 nF ceramic capacitors ("104") | 100 | One per chip, plus spares | [AliExpress](https://www.aliexpress.com/w/wholesale-100nf-ceramic-capacitor-104.html) |
| Stripboard / prototype PCB | 10 | Mount the chips and drivers on these | [AliExpress](https://www.aliexpress.com/w/wholesale-stripboard-prototype-pcb.html) |
| 2.54 mm pin headers (male and female) | 1 pack each | Board-to-board connections | [AliExpress](https://www.aliexpress.com/w/wholesale-2.54mm-pin-header.html) |
| JST-XH connector kit with crimper | 1 | Unpluggable connections between boards and the playfield harness | [AliExpress](https://www.aliexpress.com/w/wholesale-jst-xh-connector-kit-crimper.html) |

## 5. Coil drivers

15 coil outputs are used: ball serve, 2 kickers, 5 pop bumpers, 2 × 2 flipper
windings and 3 trapdoor flaps.

| Item | Qty | Notes | Link |
|---|---|---|---|
| 🛡️ IRL540N logic-level N-channel MOSFET | 20 | Must be **logic level** (the "L" in IRL). Rated 100 V, suitable for coil supplies up to about 50 V. | [Mouser](https://www.mouser.co.uk/c/?q=IRL540N) |
| 1N4004 diodes | 100 | Flyback diode, one per coil winding, unless the coil already has one | [AliExpress](https://www.aliexpress.com/w/wholesale-1n4004-diode.html) |
| 2-pin 5.08 mm PCB screw terminals | 40 | Coil wires to the driver board | [AliExpress](https://www.aliexpress.com/w/wholesale-5.08mm-pcb-screw-terminal-2-pin.html) |
| 220 Ω and 10 kΩ resistors | — | In the resistor kit from section 2 | — |

**Easier alternative:** ready-made multi-channel MOSFET driver boards that take
5 V logic inputs replace the MOSFETs, resistors and screw terminals above. Check
that they're rated for your coil voltage and current, and that they include
flyback diodes or allow them to be fitted.
[AliExpress](https://www.aliexpress.com/w/wholesale-mosfet-driver-module-8-channel.html)

## 6. Lamp drivers and lamps

21 lamp outputs are used, on chips 5–8.

| Item | Qty | Notes | Link |
|---|---|---|---|
| ULN2803A Darlington driver, DIP-18 | 8 | 4 used | [AliExpress](https://www.aliexpress.com/w/wholesale-uln2803a-dip18.html) |
| DIP-18 IC sockets | 8 | | [AliExpress](https://www.aliexpress.com/w/wholesale-dip18-ic-socket.html) |
| LED replacement bulbs, #44/#47 bayonet (BA9S), 6.3 V | 40 | Count the sockets on the playfield and backglass first. Warm white looks closest to the original incandescents. | [AliExpress](https://www.aliexpress.com/w/wholesale-ba9s-6.3v-led-pinball.html) |

## 7. Power

Choose the coil voltage **with your coil supplier before ordering a supply**.
The original 1950s coils were wound for AC. 48 V DC is a common choice for new
coils, and the IRL540N handles it.

| Item | Qty | Notes | Link |
|---|---|---|---|
| 🛡️ Coil supply: Mean Well LRS-350-48 (48 V, 350 W) | 1 | Only if your coils are specified for 48 V DC | [Mouser](https://www.mouser.co.uk/c/?q=LRS-350-48) |
| 🛡️ Lamp supply: Mean Well, matched to the bulbs | 1 | For 6.3 V bulbs, a supply adjustable to 6.3–7.5 V allows for the ULN2803's ~1 V drop | [Mouser](https://www.mouser.co.uk/c/?q=mean%20well%20LRS) |
| 🛡️ Slow-blow fuses, 5 × 20 mm, assorted | 1 set | One per supply rail; see the wiring guide for sizes | [Mouser](https://www.mouser.co.uk/c/?q=5x20%20slow%20blow%20fuse) |
| Panel-mount 5 × 20 mm fuse holders | 4 | | [AliExpress](https://www.aliexpress.com/w/wholesale-5x20mm-panel-fuse-holder.html) |
| Terminal block / bus bar | 2 | One for the common ground point, one for coil +V distribution | [AliExpress](https://www.aliexpress.com/w/wholesale-terminal-block-bus-bar.html) |
| DC-rated coil power switch | 1 | Cuts the coil supply only. Must be rated for **DC** at your coil voltage; many cheap switches are AC-only. | [AliExpress](https://www.aliexpress.com/w/wholesale-dc-rocker-switch-60v.html) |
| IEC mains inlet with fuse and switch | 1 | **Have an electrician fit and check this, and the earth bonding** | [AliExpress](https://www.aliexpress.com/w/wholesale-iec-c14-inlet-fuse-switch.html) |

## 8. Wire, connectors and finishing

| Item | Qty | Notes | Link |
|---|---|---|---|
| 22 AWG stranded hook-up wire, 6+ colours | 1 set | Switches, logic and lamps | [AliExpress](https://www.aliexpress.com/w/wholesale-22awg-stranded-hook-up-wire-kit.html) |
| 18 AWG silicone wire, red and black | 10 m each | Coil power and coil ground returns | [AliExpress](https://www.aliexpress.com/w/wholesale-18awg-silicone-wire.html) |
| Insulated crimp terminal kit (spade, ring) | 1 | | [AliExpress](https://www.aliexpress.com/w/wholesale-insulated-crimp-terminal-kit.html) |
| Ratcheting crimper for insulated terminals | 1 | | [AliExpress](https://www.aliexpress.com/w/wholesale-ratchet-crimping-tool-insulated-terminals.html) |
| Heat-shrink tubing assortment | 1 | | [AliExpress](https://www.aliexpress.com/w/wholesale-heat-shrink-tubing-assortment.html) |
| Cable ties and adhesive mounts | 1 pack each | | [AliExpress](https://www.aliexpress.com/w/wholesale-cable-tie-mounts.html) |
| Wire marker labels | 1 pack | Label both ends of every wire | [AliExpress](https://www.aliexpress.com/w/wholesale-wire-marker-labels.html) |

## 9. Tools

| Item | Qty | Notes | Link |
|---|---|---|---|
| Digital multimeter with continuity and diode test | 1 | A UNI-T is a good budget choice | [AliExpress](https://www.aliexpress.com/w/wholesale-uni-t-multimeter.html) |
| Temperature-controlled soldering station | 1 | | [AliExpress](https://www.aliexpress.com/w/wholesale-temperature-controlled-soldering-station.html) |
| Solder, 0.8 mm rosin core | 1 reel | | [AliExpress](https://www.aliexpress.com/w/wholesale-0.8mm-rosin-core-solder.html) |
| Flux pen and desoldering braid | 1 each | | [AliExpress](https://www.aliexpress.com/w/wholesale-flux-pen-desoldering-braid.html) |
| Automatic wire strippers | 1 | | [AliExpress](https://www.aliexpress.com/w/wholesale-automatic-wire-stripper.html) |
| Flush cutters | 1 | | [AliExpress](https://www.aliexpress.com/w/wholesale-flush-cutters.html) |
| Helping hands with magnifier | 1 | | [AliExpress](https://www.aliexpress.com/w/wholesale-helping-hands-soldering.html) |
| Safety glasses | 1 | | [AliExpress](https://www.aliexpress.com/w/wholesale-safety-glasses.html) |

## 10. From a pinball specialist

Order these once the playfield has been surveyed. Give the supplier the game
(1956 Williams *Fun House*), your coil supply voltage, and photos or measurements
of the old parts.

| Item | Qty | Notes |
|---|---|---|
| Kicker coils | 2 | DC, at your coil voltage |
| Pop bumper coils | 5 | DC, at your coil voltage |
| Ball serve coil and mechanism | 1 | Only if replacing the manual ball lift |
| Dual-wound flipper coils | 2 | DC, at your coil voltage; ask for the lug layout |
| Trapdoor flap coils | 3 | **Continuous-duty**, DC, at your coil voltage |
| Flipper rebuild kits, rubber rings, pop bumper caps | As needed | |
