# Shopping list

Everything needed to build the controller described in the
[wiring guide](wiring-guide.md), with quantities for the current playfield map
(`crates/pinball-core/src/playfield.rs`).

## Estimated cost

| | Estimate |
|---|---|
| **Everything** | **£870–1,535** |
| Without tools (if you already have them) | £792–1,369 |
| Electronics only (sections 1–8, no pinball parts or tools) | £504–832 |

These are rough **estimates in GBP, as of October 2026**, to help with budgeting.

- **Checked against current listings (October 2026):**
  - Mean Well GST280A48-C6P coil supply: £79–83 ex VAT at RS and Rapid
  - IRL540N MOSFETs: £1.33 each ex VAT at RS for 10 or more
  - AE-26-1200 kicker coil: £15.84 at Pinball Heaven
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
  - **Power supplies:** they connect to mains. Every supply in this list is an
    enclosed desktop adaptor with a mains lead, so nothing has bare mains
    terminals.
  - **MOSFETs:** they switch the coils.
  - **Fuses:** they're what stops a fault becoming a fire.

  Those links go to [RS UK](https://uk.rs-online.com) and
  [Rapid](https://www.rapidonline.com), which sell to individuals. **Mouser and
  Farnell don't sell some of these parts to UK consumers.** Mouser restricts
  some power supplies, the Mean Well LRS range among them, to business
  customers, and Farnell sends UK consumers to its CPC shop. Check that a
  supply is sold to you before choosing a different one.
- **Pinball-specific mechanical parts** (coils, flipper assemblies, rubbers,
  playfield switches, bulbs) are best bought from pinball specialists, who know
  what fits a 1950s Williams. Examples are [Pinball Heaven](https://www.pinball.co.uk)
  in the UK, and [Pinball Life](https://www.pinballlife.com) and
  [Marco Specialties](https://www.marcospecialties.com) in the US.
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
| 🛡️ 9 V adaptor: Mean Well GST25A09-P1J (9 V 2.55 A) | 1 | Powers the Arduino in the machine. Its 5.5 × 2.1 mm centre-positive plug fits the Arduino's barrel jack. C14 inlet, so it takes a mains lead (section 7). The GST18A09-P1J (9 V 2 A) also works. | [RS](https://uk.rs-online.com/web/c/?searchTerm=GST25A09-P1J) | 26–30 |

**Section estimate: £48–73**

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

17 coil outputs are used: ball serve, 6 kickers, 6 pop bumpers and 2 × 2
flipper windings, plus 3 for the optional bells. The trapdoor flaps use servos
instead (section 6c).

| Item | Qty | Notes | Link | Est. £ |
|---|---|---|---|---|
| 🛡️ IRL540N logic-level N-channel MOSFET | 25 | Must be **logic level** (the "L" in IRL). Rated 100 V, suitable for coil supplies up to about 50 V. | [RS](https://uk.rs-online.com/web/c/?searchTerm=IRL540NPBF) | 30–40 |
| 1N4004 diodes | 100 | Flyback diode, one per coil winding, unless the coil already has one | [AliExpress](https://www.aliexpress.com/w/wholesale-1n4004-diode.html) | 1–3 |
| 2-pin 5.08 mm PCB screw terminals | 40 | Coil wires to the driver board | [AliExpress](https://www.aliexpress.com/w/wholesale-5.08mm-pcb-screw-terminal-2-pin.html) | 3–6 |
| 220 Ω and 10 kΩ resistors | — | In the resistor kit from section 2 | — | — |

**Section estimate: £34–49**

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
| 🛡️ 5 V adaptor: Mean Well GST60A05-P1J (5 V 6 A) | 1 | Powers the WAV Trigger separately from the Arduino. The same part as the servo supply (section 6c). | [RS](https://uk.rs-online.com/web/c/?searchTerm=GST60A05-P1J) | 16–20 |
| 1 kΩ resistor | — | In the resistor kit from section 2 | — | — |

**Section estimate: £71–100**

## 6c. Trapdoor flaps

The trapdoors are rebuilt with a hobby servo turning each flap; see the
[wiring guide](wiring-guide.md#7f-trapdoor-flaps-servos).

| Item | Qty | Notes | Link | Est. £ |
|---|---|---|---|---|
| MG90S metal-gear micro servo | 4 | One per flap, plus a spare. Metal gears survive a ball landing on the flap. | [AliExpress](https://www.aliexpress.com/w/wholesale-mg90s-servo.html) | 8–14 |
| 🛡️ 5 V adaptor: Mean Well GST60A05-P1J (5 V 6 A) | 1 | For the servos only; not the Arduino's 5 V or the sound supply. Have the electrician fit it with the others (wiring guide section 4). | [RS](https://uk.rs-online.com/web/c/?searchTerm=GST60A05-P1J) | 16–20 |
| Inline fuse holder and 3 A fuse | 1 | For the servo rail's + lead | [AliExpress](https://www.aliexpress.com/w/wholesale-inline-fuse-holder-5x20.html) | 1–3 |
| 1000 µF 10 V electrolytic capacitor | 1 | Across the servo supply, near the servos | [AliExpress](https://www.aliexpress.com/w/wholesale-1000uf-10v-capacitor.html) | 1–2 |
| Servo extension leads, 30–50 cm | 3 | From under the playfield to the Arduino | [AliExpress](https://www.aliexpress.com/w/wholesale-servo-extension-cable.html) | 2–4 |
| Lever microswitches (KW11 type) | 5 | One in each trapdoor's catch box and one in the ball lift (the outhole switch), plus a spare | [AliExpress](https://www.aliexpress.com/w/wholesale-kw11-micro-switch-lever.html) | 2–4 |
| 2 mm steel rod | 1 length | The flaps' hinge pins | [AliExpress](https://www.aliexpress.com/w/wholesale-2mm-steel-rod.html) | 2–4 |
| 3D-printed ball return, PETG | about 25 parts | The trapdoors' frame, flaps, links, cranks, servo hangers and collector, and the return run, outhole funnel, serve run, ball lift and its exit, from a printing service, library or makerspace. See [3D-printed ball return](3d-printing.md). | [JLCPCB](https://jlcpcb.com/3d-printing) · [Craftcloud](https://craftcloud3d.com) | 40–90 |
| 🛡️ Wood screws, 3.5 × 9.5 mm (No. 6 × 3/8") | 1 pack | The frames, into the playfield's underside. **No longer than 9.5 mm**, or they come through the artwork. | [AliExpress](https://www.aliexpress.com/w/wholesale-wood-screw-3.5x9.5.html) | 2–4 |
| Self-tapping screw assortment, M2 and M3 | 1 box | The collector to the frame (M3 × 6 mm), the ball-lift exit's halves together (M3 × 16 mm), the cranks to the servo horns, and the links' pivots (M2 × 10 mm) | [AliExpress](https://www.aliexpress.com/w/wholesale-self-tapping-screw-assortment-m2-m3.html) | 3–5 |
| M3 × 8 mm grub screws (set screws) | 1 small pack | Holds the ball-lift cup on the coil's plunger, flush with the cup's side; comes with a hex key | [AliExpress](https://www.aliexpress.com/w/wholesale-m3-grub-screw.html) | 1–3 |

**Section estimate: £78–153**

## 7. Power

Choose the coil voltage **with your coil supplier before ordering a supply**.
The original 1950s coils were wound for AC. 48 V DC is a common choice for new
coils, and the IRL540N handles it. The first-round coils in section 10 are
modern Williams/Bally coils from the 50 V DC family, so this list uses 48 V.

Every supply here is an **enclosed desktop adaptor** with an IEC C14 inlet:
each takes an ordinary mains lead, and the leads plug into a multi-socket that
the electrician wires to the machine's switched inlet (wiring guide section 4).
Nothing has bare mains terminals. Open-frame supplies such as the Mean Well LRS
range are cheaper, but Mouser won't sell them to UK consumers, and they need
their mains terminals wired and covered.

The coil supply's 280 W (5.84 A) is less than a kick draws for a moment: one
kicker draws about 10 A for its 20 ms pulse, and two flippers about 15 A for
40 ms. Check on the bench (wiring guide section 9, step 7) that the supply
doesn't cut out when both flippers kick at once (*verify*). If it does, add a
large capacitor across the coil supply, or use the GST360A48-C6P (360 W,
7.5 A), which is the same size but harder to find in stock.

| Item | Qty | Notes | Link | Est. £ |
|---|---|---|---|---|
| 🛡️ Coil supply: Mean Well GST280A48-C6P (48 V 5.84 A, 280 W) | 1 | Only if your coils are specified for 48–50 V DC, like the 50 V family in section 10 (slightly weaker at 48 V). Its output plug is a 6-pin Molex Mini-Fit Jr: pins 1–3 are +V, 4–6 are −V (Mean Well's GST280A datasheet). Cut it off and wire the leads to the coil fuse and terminal block. | [RS](https://uk.rs-online.com/web/p/ac-dc-adapters/1176163) · [Rapid](https://www.rapidonline.com/mean-well-gst280a48-c6p-bench-psu-48v-5-84a-280-32w-erp-compliant-05-0693) | 80–100 |
| 🛡️ Lamp supply: Mean Well GST60A07-P1J (7.5 V 6 A) | 1 | For 6.3 V bulbs: 7.5 V less the ULN2803's ~1 V drop. Check the total lamp current once the sockets are counted. | [RS](https://uk.rs-online.com/web/p/desktop-power-supplies/8808394) | 22–28 |
| IEC C13 mains leads with UK plugs, 1 m | 5 | One per adaptor. The adaptors often come without one. | [RS](https://uk.rs-online.com/web/c/?searchTerm=C13%20UK%20mains%20lead) | 10–20 |
| Enclosed multi-socket, 6-way, UK | 1 | The adaptors plug into it. **The electrician wires it to the switched inlet.** | [RS](https://uk.rs-online.com/web/c/?searchTerm=6%20way%20extension%20lead) | 8–15 |
| 5.5 × 2.1 mm barrel socket to screw terminal adaptors | 3 | For the lamp, sound and servo supplies' plugs, so their leads can go to fuses and screw terminals. Low voltage, so a marketplace part is fine. | [AliExpress](https://www.aliexpress.com/w/wholesale-dc-barrel-jack-screw-terminal-female.html) | 2–4 |
| 🛡️ Slow-blow fuses, 5 × 20 mm, assorted | 1 set | One per supply rail; see the wiring guide for sizes. These glass fuses are rated for 250 V AC; check their DC rating covers 48 V before using one on the coil rail (*verify*). | [Rapid](https://www.rapidonline.com/eska-122-800-slow-blow-micro-fuses-5-x-20mm-pack-of-100-54-5747) | 30–45 |
| 🛡️ Panel-mount 5 × 20 mm fuse holders | 4 | E.g. Bulgin FX0454 (10 A) | [Rapid](https://www.rapidonline.com/search?query=FX0454) | 4–21 |
| Terminal block / bus bar | 2 | One for the common ground point, one for coil +V distribution | [AliExpress](https://www.aliexpress.com/w/wholesale-terminal-block-bus-bar.html) | 4–8 |
| 🛡️ DC-rated coil power switch | 1 | Cuts the coil supply only. Must be rated for **DC** at your coil voltage; many cheap switches are AC-only. E.g. RS PRO 119-705 (DPST, 10 A at 60 V DC). | [RS](https://uk.rs-online.com/web/p/rocker-switches/0119705) | 5–7 |
| 🛡️ IEC C14 mains inlet with fuse and switch | 1 | E.g. Bulgin BZV01/Z0000/01. Check it switches both live and neutral (*verify*). **Have an electrician fit and check this, and the earth bonding.** | [Rapid](https://www.rapidonline.com/bulgin-bzv01-z0000-01-fused-and-switched-vertical-inlet-23-3224) | 11–14 |

**Section estimate: £176–262**

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
| Digital calipers, 150 mm | 1 | For measuring the playfield for the 3D-printed parts | [AliExpress](https://www.aliexpress.com/w/wholesale-digital-caliper-150mm.html) | 10–15 |

**Section estimate: £78–166**

## 10. From a pinball specialist

Buy these in two rounds. The first round is enough to bench-test the coil
drivers (wiring guide section 9, steps 6–7) and to measure the parts the
3D-printed ball return is built around. Order the second round once the
playfield has been surveyed and the first flipper has been test-fitted under
it. For that order, give the supplier the game (1956 Williams *Fun House*), your
coil supply voltage, and photos or measurements of the old parts.

Shops sell the kickers as **slingshot** coils and assemblies; they rarely use
the word "kicker".

### First round: bench testing

These are modern (1980s–90s) Williams/Bally parts from the 50 V DC family,
which is why the coil supply is 48 V (section 7). The coil figures below are
from the suppliers' listings and memory: *verify* them with the multimeter
before wiring.

| Item | Qty | Notes | Est. £ |
|---|---|---|---|
| Left flipper assembly with an FL-11722 coil | 1 | Williams/Bally WPC era. The FL-11722 (green, medium-low strength) is dual-wound with 3 lugs: about 6.2 Ω power, 160 Ω hold, diodes fitted. Leave its end-of-stroke switch unwired; the controller times the flip. Whether it fits under the 1956 playfield is unknown: compare its plate and shaft position with the old mechanism. From [Pinball Heaven](https://www.pinball.co.uk/spares-accessories/major-playfield-mechanisms/flipper-parts/left-flipper-assembly-fl-11722-coil/). | 40–80 |
| Kicker (slingshot) coil, AE-26-1200 | 1 | The first coil to bring up (wiring guide section 9, step 6). Williams' standard slingshot and pop bumper coil: 26 AWG, 1,200 turns, roughly 10 A at 48 V. Avoid the stronger AE-23/24/25 coils and the weak AE-27/30 coils. | 15–17 |
| Coil sleeve and plunger for the AE-26-1200 | 1 each | A bare coil can't kick. Ask the supplier for the ones that match a slingshot. | 5–10 |
| Flipper bat with shaft, and a rubber ring | 1 | Only if the assembly doesn't include them | 8–15 |
| Pinballs, 1-1/16" (27 mm) steel | 3–5 | For testing the flipper and test-fitting the printed ball return. Measure an original ball too (*verify* that it's the same size). | 5–10 |
| Ball-serve (ball lift) coil assembly | 1 | Kicks each ball up the printed ball lift into the shooter lane. Buy it early if you can: measure its plunger and mounting screws for `measurements.scad` before ordering prints. | 25–60 |
| *Optional:* flipper button with its leaf switch | 1 | The tactile buttons in section 2 do on the bench, but a real one shows how the flip feels | 5–10 |

**First round estimate: £98–192**

### Second round: after the survey

| Item | Qty | Notes | Est. £ |
|---|---|---|---|
| Kicker (slingshot) coils | 5 | DC, at your coil voltage. Six kickers in all; the bench-test coil can be the sixth if it suits the table. | 50–75 |
| Pop bumper coils | 6 | DC, at your coil voltage | 60–90 |
| Right flipper assembly | 1 | To match the left one, once that has been test-fitted. If the left one doesn't fit, choose a pair that does. Ask for the lug layout. | 40–80 |
| *Optional:* bells with coils | 0–3 | Not needed now. For the authentic 1950s scoring sound later (wiring guide section 7e). | varies |
| Flipper rebuild kits, rubber rings, pop bumper caps | As needed | | 40–100 |

**Second round estimate: £190–345**

**Section estimate: £288–537**
