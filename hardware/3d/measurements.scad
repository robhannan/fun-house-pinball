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
outhole_y = 25;
outhole_diameter = 32;

// --- The ball lift ----------------------------------------------------------

// Position of the hole in the shooter lane that the ball is lifted up through,
// and its diameter.
lift_hole_x = 502;
lift_hole_y = 40;
lift_hole_diameter = 30;

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
