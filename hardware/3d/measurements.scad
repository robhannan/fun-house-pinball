// Measurements of YOUR table, shared by every printed part, in millimetres.
//
// These are placeholders until the real table is measured with calipers. Edit
// the numbers here (nothing else in this file), save, then open a part and
// export it. See docs/3d-printing.md.
//
// Positions are measured on the underside of the playfield, from its
// BOTTOM-LEFT corner (the corner nearest the player's left hand): x to the
// right, y up the table. The placeholders come from the playfield map and are
// only rough.

// --- The playfield ----------------------------------------------------------

// Thickness of the playfield at the trapdoor holes.
playfield_thickness = 12.7;

// The whole playfield board's width and length (the board runs on under the
// apron, so measure the board itself, not just what's visible).
playfield_width = 520;
playfield_length = 1010;

// --- The trapdoors ----------------------------------------------------------
// "Width" runs across the table (left to right); "length" runs up the table.

// Width of each trapdoor hole.
hole_width = 44;

// Length of each trapdoor hole.
hole_length = 48;

// Distance between the centres of neighbouring holes.
trapdoor_spacing = 50;

// Position of the CENTRE trapdoor hole's centre.
trapdoor_x = 240;
trapdoor_y = 614;

// --- The outhole ------------------------------------------------------------

// Position of the outhole's centre, and its diameter.
outhole_x = 240;
outhole_y = 60;
outhole_diameter = 32;

// --- The shooter lane and ball lift ----------------------------------------
// The ball comes up through a hole in the playfield beside the shooter lane
// (hidden under the apron), and rolls into the lane through an opening in the
// lane's side wall. The opening is assumed to be straight across from the
// hole; if it isn't, say so and the model can be changed.

// Position of the hole the ball is lifted up through, and its diameter.
lift_hole_x = 445;
lift_hole_y = 90;
lift_hole_diameter = 32;

// The shooter lane's side wall, between the hole and the lane: the x of its
// face on the hole's side, and its thickness.
lane_wall_x = 470;
lane_wall_thickness = 10;
// The opening in that wall: its width (along the lane) and height.
lane_opening_width = 34;
lane_opening_height = 32;
// The shooter lane's width, inside.
lane_width = 30;
// Where the plunger's tip is when it's at rest: its y.
plunger_tip_y = 45;
// The space above the playfield over the hole, under the apron (or whatever
// covers it).
lift_headroom = 45;

// The ball-serve (ball lift) coil assembly: the diameter of its plunger, and
// the distance between its two mounting screws.
lift_plunger_diameter = 12.7;
lift_coil_screw_spacing = 40;

// --- Screws and pins ---------------------------------------------------------

// Wood screws into the underside of the playfield. They must be SHORTER than
// the playfield is thick, or they'll come out through the artwork. 3.5 mm x
// 9.5 mm (No. 6 x 3/8") suits a 12.7 mm playfield.
wood_screw_diameter = 3.5;

// Steel rod used as each flap's hinge pin.
hinge_pin_diameter = 2;

// --- Printing ---------------------------------------------------------------

// Gap left between parts that fit together, for a typical printer. Make it
// bigger if parts are too tight, smaller if they're loose.
fit_clearance = 0.4;

// The longest part your printer (or printing service) can print. Long runs are
// split into segments no longer than this.
max_print_length = 170;
