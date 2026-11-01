// Measurements of YOUR table, shared by every printed part, in millimetres.
//
// Numbers marked "measured" were taken on the table, with a tape measure or
// calipers; the rest are placeholders until measured. Edit the numbers here
// (nothing else in this file), save, then open a part and export it. See
// docs/3d-printing.md.
//
// Positions are measured from the playfield's BOTTOM-LEFT corner (the corner
// nearest the player's left hand): x to the right and y up the table, as the
// player sees it from the front, even when measuring the underside.
// docs/playfield.svg is drawn to the same scale, and the simulator's tests
// check that the board, trapdoors and outhole here match it, so change both
// together.

// --- The playfield ----------------------------------------------------------

// Thickness of the playfield at the trapdoor holes. Measured with calipers.
playfield_thickness = 13.5;

// The whole playfield board's width and length (the board runs on under the
// apron, so measure the board itself, not just what's visible). Measured.
playfield_width = 520;
playfield_length = 1079;

// --- The trapdoors ----------------------------------------------------------
// "Width" runs across the table (left to right); "length" runs up the table.

// Width of each trapdoor hole. Measured with calipers.
hole_width = 30.5;

// Length of each trapdoor hole. Measured with calipers.
hole_length = 65;

// Thickness of the metal plate glued on top of each flap. Measured.
plate_thickness = 1;

// Distance between the centres of neighbouring holes. Measured with calipers.
trapdoor_spacing = 65.5;

// Position of the CENTRE trapdoor hole's centre. Measured.
trapdoor_x = 235;
trapdoor_y = 565;

// --- The outhole ------------------------------------------------------------

// Position of the outhole's centre, and its diameter. Measured.
outhole_x = 237;
outhole_y = 95;
outhole_diameter = 37.8;

// --- The shooter lane and ball lift ----------------------------------------
// The ball comes up through a hole in the playfield beside the shooter lane
// (hidden under the apron), and rolls into the lane through an opening in the
// lane's side wall. The opening is assumed to be straight across from the
// hole; if it isn't, say so and the model can be changed.

// Position of the hole the ball is lifted up through, and its diameter.
lift_hole_x = 434;
lift_hole_y = 90;
lift_hole_diameter = 32;

// The shooter lane's side wall, between the hole and the lane: the x of its
// face on the hole's side (measured), and its thickness.
lane_wall_x = 459;
lane_wall_thickness = 12.9;
// The opening in that wall: its width (along the lane) and height.
lane_opening_width = 34;
lane_opening_height = 32;
// The shooter lane's width, inside. Measured.
lane_width = 35;
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
// 9.5 mm (No. 6 x 3/8") suits a 13.5 mm playfield.
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
