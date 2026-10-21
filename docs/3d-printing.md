# 3D-printed ball return

The ball return is 3D-printed: everything under the playfield that gets a ball
from the three trapdoors and the outhole back up into the shooter lane, and the
small hood on top that turns it into the lane. The models are in
[`hardware/3d/`](../hardware/3d/). You don't need any 3D-modelling experience:
they're written as code, with every measurement of your table in one file.
Making them fit means measuring the table and typing the numbers in.

![the whole ball return under the playfield: the trapdoors at the back, the return run down to the outhole funnel, and the serve run across to the ball lift](images/ball-return.png)

*The whole return, seen from above with the playfield hidden, with the
placeholder measurements: the trapdoors (top right), the return run down the
table, the outhole funnel (left), and the serve run across to the ball lift,
with its exit on top.*

## How it works

1. **The trapdoors.** Each flap is hinged along its back edge. When the game
   opens it, its servo swings it down through a link and crank until it hangs
   straight down, clearing the hole. Closed, the servo holds it flush.
2. **The catch boxes.** A ball dropping through lands in that trapdoor's own
   box, rolls forward over that trapdoor's microswitch, and drops into the
   collector trough across the front.
3. **The return run.** The trough feeds a channel that hangs under the
   playfield and runs down the table to the outhole. It stays level with the
   playfield: the playfield's own slope keeps the ball rolling.
4. **The outhole funnel.** Drained balls drop into it from above, trapdoor
   balls arrive from the return run, and both roll out along the serve run.
5. **The ball lift.** The serve run brings every ball to a tube under a hole
   beside the shooter lane. The ball waits on the ball-serve coil's plunger,
   holding the outhole switch closed. When the game serves a ball, the coil
   kicks it up the tube.
6. **The ball-lift exit.** On top of the playfield, over the hole (hidden under
   the apron), the ball hits a hood whose roof slopes up towards the lane. That
   turns it sideways, and it rolls down a short ramp, through the opening in the
   shooter lane's wall, and down the lane to the plunger. The ramp only slopes
   down into the lane, so a ball that rolls back down the lane after a weak
   plunge goes past the opening to the plunger again.

![the ball lift cut through the middle: the serve run arrives at the bottom of the tube, where the cup waits on the coil's plunger; the tube rises through the playfield into the hood on top, beside the shooter lane's wall](images/ball-lift.png)

*The ball lift, cut through the middle: the serve run (left) arrives at the
bottom of the tube, where the cup (red) waits on the coil's plunger. The tube
rises through the playfield into the hood, beside the shooter lane's wall
(right).*

| Closed | Open |
|---|---|
| ![a trapdoor's drive, cut away: the link runs from the rib under the closed flap down to the crank on the servo](images/trapdoor-closed.png) | ![the same drive with the trapdoor open: the crank has turned and the rib has swung back as the flap hangs down](images/trapdoor-open.png) |

*The right-hand trapdoor's drive, cut away beside it: the rib under the flap
(top left), the link, and the crank on the servo. Opening, the servo turns the
crank about 80 degrees and the flap swings down to hang straight.*

## The parts

| Part | File | Print | What it does |
|---|---|---|---|
| Trapdoor frame | `trapdoor_frame.scad` | 1 | Slides up into all three trapdoor holes and screws on underneath. Holds the flaps' hinge pins. |
| Trapdoor flap | `trapdoor_flap.scad` | 3 | Sits flush in the frame and pivots on its pin. The rib underneath joins the link. |
| Trapdoor link | `trapdoor_link.scad` | 3 | Joins a flap's rib to its crank. |
| Trapdoor crank | `trapdoor_crank.scad` | 3 | Screws to the servo's horn and turns the link. |
| Servo hanger | `trapdoor_servo_hanger.scad` | 3 | Screws under the playfield behind a trapdoor and holds its servo. |
| Trapdoor collector | `trapdoor_collector.scad` | 1 | The three catch boxes, with a microswitch mount each, and the trough that feeds the return run. |
| Return run | `return_run_segment.scad` | one of each segment | The channel down the table to the outhole, in segments that fit a printer. Each hangs from the playfield on two posts. |
| Outhole funnel | `outhole_funnel.scad` | 1 | Catches drained balls and the return run, and feeds the serve run. |
| Serve run | `serve_run_segment.scad` | one of each segment | The channel across to the ball lift, sloping down. |
| Ball lift | `ball_lift.scad` | 1 | The tube up to the hole beside the shooter lane, with mounts for the ball-serve coil and the outhole switch. |
| Ball-lift exit | `ball_lift_exit.scad` | 1 | Screws on top of the playfield over the hole, and turns the ball through the lane's wall into the shooter lane. |
| Ball-lift cup | `ball_lift_cup.scad` | 1 | Pushes onto the coil's plunger and cradles the waiting ball. |

`assembly.scad` shows everything in place; it isn't printed. The files in
`hardware/3d/checks/` check that nothing collides: see step 4.

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
calipers and a tape measure (see the [shopping list](shopping-list.md#9-tools)),
measure, on the underside of the playfield:

- **The playfield's thickness** at the trapdoor holes.
- **Each trapdoor hole's width** (across the table) **and length** (up the
  table), and the distance between the holes' centres. If the holes differ,
  use the smallest.
- **Where things are**, measured from the playfield's **bottom-left corner**
  (nearest the player's left hand): the centre of the middle trapdoor hole,
  the centre of the outhole and its diameter, and the centre of the hole beside
  the shooter lane that the ball is lifted through, and its diameter.
- **The shooter lane**, on top of the playfield: where its wall (the one with
  the opening the ball rolls in through) is and how thick it is, the opening's
  width and height, the lane's width, where the plunger's tip is at rest, and
  how much room there is above the hole, under the apron. The model assumes the
  opening is straight across from the hole: say if it isn't.
- **The ball-serve coil**: its plunger's diameter, and the distance between its
  mounting screws.

Also check what's already under the playfield along the route: the return run
goes straight from the trapdoors to the outhole, and the serve run straight
across to the shooter lane. `pnpm run export:3d` prints how far below the
playfield the deepest part reaches (about 105 mm with the placeholders), and
the ball-serve coil hangs below that. Note anything in the way (coils, wiring,
old brackets, the old outhole mechanism) and the depth to the cabinet floor.

Photos of the underside with a ruler in shot help a lot if the routes need
changing.

## 3. Type in your measurements

Open `hardware/3d/measurements.scad` in OpenSCAD (or any text editor), change
the numbers, and save. Every part reads this file, so they always fit each
other, and the runs are re-routed and re-split to suit. Change nothing else in
it. Set `max_print_length` to the longest part your printer (or printing
service) can print.

If a measurement can't work, OpenSCAD stops with a message saying why, such as
a trapdoor hole being too short for the ball to fall past the open flap.

## 4. Check it

1. Open `assembly.scad` and press **F5** for a quick preview. Drag to turn it
   round, scroll to zoom, and look for anything that would clash with what's
   under your playfield.
2. Open **Window → Customizer** for the options: tick **open** to see the
   trapdoors open, **section** to cut beside a trapdoor's drive,
   **lift_section** to cut through the ball lift, and **show_playfield** to
   show the playfield.
3. Open each file in `hardware/3d/checks/` and press **F6**. Each must say
   **"Current top level object is empty"**:
   - `trapdoor_swing.scad`: as the trapdoors open, nothing they move hits
     anything else.
   - `ball_paths.scad`: a ball can roll the whole way, from each trapdoor and
     the outhole to the plunger in the shooter lane, without touching
     anything, and a ball rolling back down the lane rolls past the exit.
   - `parts_overlap.scad`: no two parts overlap where they're fitted.

   If something appears instead, that's where the parts collide. Or, from the
   repository, run all the checks at once: `pnpm run check:3d`.

## 5. Export the files for printing

From the repository, export every part, each run segment separately, into
`target/3d/`:

```sh
pnpm run export:3d
```

It prints each part's size. Or, in OpenSCAD: open a part, press **F6**, then
**File → Export → Export as STL**. For the run segments, choose each
**segment** in the Customizer in turn and export each one.

## 6. Get them printed

You don't need your own printer:

- **A library or makerspace:** many UK libraries and makerspaces have 3D
  printers, and people happy to help with a first print.
- **An online service:** upload the STL files and the parts arrive by post.
  [JLCPCB](https://jlcpcb.com/3d-printing) and
  [Craftcloud](https://craftcloud3d.com) both do this.
- **Your own printer:** a Bambu Lab A1 mini (about £180–250) is a common
  choice that works well out of the box.

Whichever you use, ask for:

- **Material: PETG**, not PLA. It's tougher, and it won't soften in the warmth
  under a playfield; PLA starts to at about 55 °C.
- **0.2 mm layers, 4 walls (perimeters), 30% infill.**
- **Supports** for the collector, the outhole funnel, the ball lift and its
  exit. The other parts print without.

Each part's file says which way up to print it.

## 7. Fit it

1. **Trapdoors.** Push each flap's hinge pin (2 mm steel rod) through the
   frame, the flap's knuckle and out the other side, and trim it flush. If the
   frame's holes are too tight, open them with a 2 mm drill. Slide the frame up
   into the holes and screw it on with 3.5 × 9.5 mm wood screws: **no longer
   than 9.5 mm**, or they'll come through the playfield's artwork.
2. **Servos.** Screw each servo into its hanger, shaft pointing left, and screw
   the hanger under the playfield behind its trapdoor. Screw a crank to each
   servo's single-arm horn and press it onto the shaft. Join the flap's rib to
   the crank with a link, with an M2 × 10 mm screw at each end (into the rib
   and into the crank). Set the angles in test mode (wiring guide section 7f).
3. **Collector.** Screw a lever microswitch under each catch box, its lever up
   through the slot, and screw the collector to the frame's front flange with
   M3 × 6 mm self-tapping screws.
4. **Runs.** Fit the return run's segments in order from the collector, each
   socket over the end of the part before, and screw their posts to the
   playfield. Then the outhole funnel under the outhole, the serve run's
   segments, and the ball lift under the hole beside the shooter lane.
5. **Ball lift.** Bolt the ball-serve coil under the lift's base, push the cup
   onto its plunger, and screw the outhole switch to the tab on the tube with
   its lever through the slot. On top of the playfield, screw the exit over the
   hole with its ramp through the opening in the lane's wall.
6. **Roll a ball through** by hand from each trapdoor and the outhole, and
   watch it reach the lift. Then push the ball up the tube by hand (or pulse the
   coil in test mode) and watch it roll into the shooter lane.

If something doesn't fit, change the measurement and print that part again:
that's the point of printing them. Small fit problems usually need
`fit_clearance` adjusting.
