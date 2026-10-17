# 3D-printed parts

The trapdoors' under-playfield parts are 3D-printed, from models in
[`hardware/3d/`](../hardware/3d/). You don't need any 3D-modelling experience:
the models are written as code with every size you might change listed in one
file, so making them fit your table means measuring it and typing numbers in.

| Part | File | How many | What it does |
|---|---|---|---|
| Frame | `trapdoor_frame.scad` | 3 | Slides up into a trapdoor hole and screws on underneath. Holds the flap's hinge pin; the servo hangs from it. |
| Flap | `trapdoor_flap.scad` | 3 | The trapdoor itself: sits flush in the top of the frame and pivots on the pin. |
| Lifter arm | `trapdoor_lifter.scad` | 3 | Screws to the servo's arm. The flap rests on its tip: up holds it closed, down lets it swing open. |
| Chute | `trapdoor_chute.scad` | 3 | Screws under the frame, catches the ball, and rolls it towards the outhole. The microswitch mounts under it. |

`assembly.scad` shows how they all fit together, with the playfield and servo
drawn in; it isn't printed.

| Closed | Open |
|---|---|
| ![a trapdoor cut in half, closed: the lifter arm stands upright under the flap](images/trapdoor-closed.png) | ![the same trapdoor open: the lifter has turned back and the flap hangs down on it](images/trapdoor-open.png) |

*A trapdoor cut in half, with the placeholder sizes: the frame (grey), the
flap (yellow) resting on the lifter arm (red), and the chute (green) with the
microswitch tab underneath. The servo hangs on the right, cut away here.* The wiring guide's
[section 7f](wiring-guide.md#7f-trapdoor-flaps-servos) covers wiring the
servos and switches.

## 1. Install OpenSCAD

[OpenSCAD](https://openscad.org) is free. Use a recent **development snapshot**
from the [downloads page](https://openscad.org/downloads.html#snapshots), not
the old 2021 release; CI checks the models with the latest snapshot too. On a
Mac with Homebrew:

```sh
brew install --cask openscad@snapshot
```

## 2. Measure your table

The sizes in `hardware/3d/measurements.scad` are placeholders. With digital
calipers (see the [shopping list](shopping-list.md#9-tools)), measure:

- **The playfield's thickness** at the trapdoor holes.
- **Each hole's width** (across the table) **and length** (up the table). If
  the three holes differ, use the smallest; the frames are a sliding fit.

Then also note, for the chute (step 4):

- **The space below:** how far the cabinet floor, or anything in the way (old
  brackets, wiring, the outhole mechanism), is below the playfield.
- **The way to the outhole:** which direction, and how far, the ball needs to
  roll from the holes.

Photos of the underside around the "Scores Bonus" box, with a ruler in shot,
help a lot if the parts need changing.

## 3. Type in your measurements

Open `hardware/3d/measurements.scad` in OpenSCAD (or any text editor), change
the numbers, and save. Every part reads this file, so they always fit each
other. Change nothing else in it.

If a measurement can't work, OpenSCAD stops with a message saying why, such as
the hole being too narrow for the ball to fall past the lifter arm.

## 4. Check the parts, and adjust the chute

1. Open `assembly.scad`. Press **F5** for a quick preview, then drag to turn it
   around and scroll to zoom.
2. Open **Window → Customizer** for the options: tick **open** to see the flap
   open, untick **show_playfield** to see underneath, and tick **section** to
   cut through the middle.
3. Open `trapdoor_chute.scad` and set its options in the Customizer:
   - **exit:** which way the ball rolls out, `front` (down the table) or
     `left`. Not right: the servo is there.
   - **drop**, **slope** and **length:** how far the funnel drops, the slope of
     the channel, and how long it is, so it reaches the outhole and clears
     anything underneath.
   - **switch_at:** where the microswitch sits along the channel.

   Click **+** next to the presets box to save your settings, so they're kept.

## 5. Export the files for printing

For each part: open it, press **F6** to render it (this takes a few seconds),
then **File → Export → Export as STL**. Or, from the repository, export them
all at once into `target/3d/`:

```sh
pnpm run export:3d
```

(The command line uses each part's default options; for your own chute
settings, export it from OpenSCAD.)

## 6. Get them printed

You don't need your own printer:

- **A library or makerspace:** many UK libraries and makerspaces have 3D
  printers, and people happy to help with a first print.
- **An online service:** upload the STL files and the parts arrive by post.
  [JLCPCB](https://jlcpcb.com/3d-printing) and
  [Craftcloud](https://craftcloud3d.com) both do this. Expect a few pounds a
  part.
- **Your own printer:** a Bambu Lab A1 mini (about £180–250) is a common
  choice that works well out of the box.

Whichever you use, ask for:

- **Material: PETG**, not PLA. It's tougher, and it won't soften in the warmth
  under a playfield; PLA starts to at about 55 °C.
- **0.2 mm layers, 4 walls (perimeters), 30% infill.**
- **Supports** for the chute only, under its sloping floor. The other parts
  print without.

Each part's file says which way up to print it.

## 7. Put each trapdoor together

1. Push the hinge pin (2 mm steel rod) through one side of the frame, through
   the flap's knuckle, and out of the other side. Trim it flush. If the frame's
   holes are too tight, open them with a 2 mm drill.
2. Slide the frame up into its hole. Screw it on with four 3.5 × 9.5 mm wood
   screws. **No longer than 9.5 mm**, or they'll come through the playfield's
   artwork.
3. Screw the servo to the frame's mount, its shaft pointing into the frame.
4. Screw the lifter arm to the single-arm horn that came with the servo, and
   press the horn onto the shaft pointing roughly up. Test mode sets the exact
   angles (wiring guide section 7f).
5. Screw the microswitch to the tab under the chute, its lever poking up
   through the slot. Screw the chute to the frame with two M3 × 6 mm
   self-tapping screws.

If something doesn't fit, change the measurement and print again: that's the
point of printing them. Small fit problems usually need `fit_clearance`
adjusting.
