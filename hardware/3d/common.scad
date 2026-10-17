// Shared helpers for the printed parts. Nothing to change here: table sizes
// are in measurements.scad, and each part's options are in its own file.

include <measurements.scad>

// A real pinball: 1 1/16 inch.
ball_diameter = 27;

// Smooth enough curves for printing.
$fn = 48;

// A countersunk hole for a wood screw going up into the playfield, through a
// plate `thickness` thick lying on z = 0 (its top face is the side that
// presses against the playfield).
module wood_screw_hole(thickness) {
    head = wood_screw_diameter * 2;
    translate([0, 0, -0.01]) {
        cylinder(d = wood_screw_diameter + 0.3, h = thickness + 0.02);
        // The head sits in the face away from the playfield.
        cylinder(d1 = head, d2 = wood_screw_diameter + 0.3, h = (head - wood_screw_diameter) / 2);
    }
}

// --- Trapdoor geometry, shared so the frame, flap, lifter and chute fit -----
//
// Coordinates: the origin is the centre of the hole on the playfield's
// underside. x runs across the table (left to right), y up the table (towards
// the back), z up. The playfield's top surface is at z = playfield_thickness.

// The frame's sleeve slides up into the hole.
frame_wall = 2;
outer_width = hole_width - fit_clearance;
outer_length = hole_length - fit_clearance;
inner_width = outer_width - 2 * frame_wall;
inner_length = outer_length - 2 * frame_wall;
// The flange under the playfield.
flange_width = 10;
flange_thickness = 4;

// The flap: a plate that sits flush in the top of the frame, pivoting on the
// hinge pin along its uphill (back) edge.
flap_thickness = 3;
knuckle_radius = 2.5;
pin_y = inner_length / 2 - knuckle_radius - fit_clearance;
pin_z = playfield_thickness - knuckle_radius;
flap_width = inner_width - 2 * fit_clearance;
flap_front = -inner_length / 2 + fit_clearance;

// The servo (an MG90S) hangs under the flange on the right, its shaft pointing
// left into the frame, and turns the lifter arm that the flap rests on.
servo_body = [22.8, 12.2, 22.5];  // long, wide, top face to base
servo_shaft_from_end = 5.9;       // shaft centre from the near end of the body
servo_tab_from_top = 6.5;         // mounting tabs below the shaft's face
servo_tab_spacing = 27.8;
servo_tab_hole = 1.8;             // for the screws that come with the servo
// Where the lifter arm swings: just inside the frame's right wall.
lifter_thickness = 3;
lifter_x = inner_width / 2 - 1 - lifter_thickness;
servo_top_x = lifter_x + lifter_thickness + 3;  // the horn sits ~3 mm out
servo_shaft_y = flap_front + 9;                 // under the flap's front edge
servo_shaft_z = -flange_thickness - servo_body[1] / 2 - 1;
// The lifter reaches from the shaft up to the underside of the closed flap.
lifter_length = playfield_thickness - flap_thickness - servo_shaft_z;

// The chute catches the ball beside the lifter and servo: from the frame's left
// wall to just short of the lifter arm.
chute_wall = 2.5;
chute_right = lifter_x - 1 - chute_wall;
chute_opening = chute_right + inner_width / 2;
// Pilot holes in the frame's flange for the chute's two screws (M3 x 6 mm
// self-tapping): one on the left, one at the front.
chute_screws = [
    [-(outer_width / 2 + flange_width / 2), 0],
    [(chute_right - inner_width / 2) / 2, -(outer_length / 2 + flange_width / 2)],
];
assert(chute_opening >= ball_diameter + 1,
    "The hole is too narrow for the ball to fall past the lifter arm: check hole_width in measurements.scad.");
