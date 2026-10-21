// Shared geometry for the printed ball-return parts. Nothing to change here:
// table sizes are in measurements.scad, and each part's options are in its
// own file.
//
// Two kinds of coordinates are used:
// - Playfield coordinates: from the playfield's bottom-left corner, on its
//   underside. x right, y up the table, z up (the underside is z = 0).
// - Trapdoor coordinates: the same, but with the origin at the centre of a
//   trapdoor hole.

include <measurements.scad>

// A real pinball: 1 1/16 inch.
ball_diameter = 27;

// Smooth enough curves for printing.
$fn = 48;

// --- Screws -----------------------------------------------------------------

// A countersunk hole for a wood screw going up into the playfield, through a
// plate `thickness` thick whose top face (z = thickness) presses against the
// playfield. The head sits in the bottom face.
module wood_screw_hole(thickness) {
    head = wood_screw_diameter * 2;
    translate([0, 0, -0.01]) {
        cylinder(d = wood_screw_diameter + 0.3, h = thickness + 0.02);
        cylinder(d1 = head, d2 = wood_screw_diameter + 0.3, h = (head - wood_screw_diameter) / 2);
    }
}

// Hole sizes for M2 screws: tapped straight into the plastic, or a free fit.
m2_tap = 1.7;
m2_free = 2.4;

// --- Trapdoors: frame and flap (trapdoor coordinates) -----------------------

// The frame's sleeves slide up into the holes.
frame_wall = 2;
outer_width = hole_width - fit_clearance;
outer_length = hole_length - fit_clearance;
inner_width = outer_width - 2 * frame_wall;
inner_length = outer_length - 2 * frame_wall;
// The flange under the playfield, around all three sleeves.
flange_width = 10;
flange_thickness = 4;

// The flap sits flush in the top of the sleeve and pivots on the hinge pin
// along its back (uphill) edge. Open, it hangs straight down, clearing the
// hole for the ball.
flap_thickness = 3;
knuckle_radius = 2.5;
pin_y = inner_length / 2 - knuckle_radius - fit_clearance;
pin_z = playfield_thickness - knuckle_radius;
flap_width = inner_width - 2 * fit_clearance;
flap_front = -inner_length / 2 + fit_clearance;
flap_reach = pin_y - flap_front;  // hinge to front edge
// Closed, the flap is held flush by the servo through the linkage: a fixed
// ledge can't be used, because the flap's front edge swings down to open.
// How far the flap opens: straight down.
flap_open_angle = 90;

// --- Trapdoors: the drive linkage (trapdoor coordinates) --------------------
//
// A rib under the flap's right edge, near the front, joins a link to a crank
// on the servo, behind and below the frame. These positions were chosen so
// that the servo turns about 80 degrees to swing the flap from closed to
// hanging open, the link never gets close to locking up, and the link and
// crank stay below the frame.

rib_hole = [-18.5, -18.5];        // from the hinge pin, [y, z], flap closed
crank_centre = [32.1, -63.2];     // the servo's shaft, from the hinge pin
crank_radius = 24;
link_length = 52;
// Across the table, the drive parts sit side by side, right of the flap's
// edge: [left, right] x of each.
rib_x = [flap_width / 2 - 3, flap_width / 2];
link_x = [rib_x[1] + 1, rib_x[1] + 4];
crank_x = [link_x[1] + 0.4, link_x[1] + 3.4];
// The servo (an MG90S): its horn sits against the crank, then its top face.
servo_body = [22.8, 12.2, 22.5];  // long, wide, top face to base
servo_shaft_from_end = 5.9;       // shaft centre from the front end of the body
servo_tab_from_top = 6.5;         // mounting tabs below the top face
servo_tab_spacing = 27.8;
servo_tab_hole = 1.8;             // for the screws that come with the servo
servo_top_x = crank_x[1] + 3;
servo_shaft = [pin_y + crank_centre[0], pin_z + crank_centre[1]];

// Turns a [y, z] point about the hinge pin as the flap opens by `angle`.
function flap_turn(p, angle) = [
    pin_y + (p[0] - pin_y) * cos(angle) - (p[1] - pin_z) * sin(angle),
    pin_z + (p[0] - pin_y) * sin(angle) + (p[1] - pin_z) * cos(angle)
];

// The rib's hole, [y, z], with the flap open by `angle`.
function rib_point(angle) = flap_turn([pin_y + rib_hole[0], pin_z + rib_hole[1]], angle);

// The crank's pin, [y, z], holding the flap open by `angle`: where a circle of
// the link's length round the rib's hole meets the crank's circle.
function crank_point(angle) =
    let(
        a = rib_point(angle), s = servo_shaft,
        d = norm(a - s), u = (a - s) / d,
        along = (crank_radius * crank_radius - link_length * link_length + d * d) / (2 * d),
        h = sqrt(max(crank_radius * crank_radius - along * along, 0)),
        m = s + u * along
    )
    [m[0] + h * u[1], m[1] - h * u[0]];

// The crank's angle (degrees, anticlockwise from +y towards +z) for `angle`.
function crank_angle(angle) =
    let(b = crank_point(angle) - servo_shaft) atan2(b[1], b[0]);

// --- Trapdoors: catch boxes and collector (trapdoor coordinates) ------------
//
// The flap sweeps a quarter-circle under its hole as it opens, so each
// trapdoor's catch box stays outside it: its back wall is low, its right wall
// steps aside for the rib, and its floor is below the swing. The ball lands,
// runs forward past that trapdoor's microswitch and out through the front, into
// a collector trough across the front of all three that feeds the return run.

wall = 2.5;
channel_inner = ball_diameter + 4;
channel_height = 20;
// Openings in walls that the ball rolls through.
doorway_height = ball_diameter + 4;
// The flap's swing, with a margin.
sweep_radius = flap_reach + 2;
catch_left = -inner_width / 2 - wall;           // the left wall's inside face
catch_right = rib_x[0] - 1.5 - wall;            // the right wall's inside face
catch_front = -inner_length / 2;                // the front wall's inside face
catch_back = pin_y - knuckle_radius - 1 - wall; // in front of the hanging flap
catch_floor_back = pin_z - sweep_radius - 3;
catch_floor_front = catch_floor_back - 4;
catch_opening = catch_right - catch_left;
assert(catch_opening >= ball_diameter + 2,
    "The trapdoor hole is too narrow for the ball to fall past the drive: check hole_width in measurements.scad.");
assert(catch_back - catch_front >= ball_diameter + 2,
    "The trapdoor hole is too short for the ball to fall past the open flap: check hole_length in measurements.scad.");
// The collector trough, in front of the catch boxes.
trough_y = catch_front - wall - channel_inner / 2;
trough_floor_end = catch_floor_front - joint_step_value();
trough_floor_centre = trough_floor_end - trapdoor_spacing * tan(6);
// The exit, heading down the table from the trough's middle.
exit_length = 20;
exit_start = [0, trough_y - channel_inner / 2, trough_floor_centre];
exit_end = [0, trough_y - channel_inner / 2 - exit_length, trough_floor_centre];

// The three trapdoors' centres, across the table from the centre one.
trapdoor_offsets = [-trapdoor_spacing, 0, trapdoor_spacing];
// The collector's top, just under the frame's flange (with a hair's gap, so
// the collision checks can tell touching from overlapping).
collector_top = -flange_thickness - 0.1;
// Where the collector screws to the frame's front flange: one in front of each
// catch box.
collector_screws = [for (dx = trapdoor_offsets) [dx - 8, -outer_length / 2 - flange_width / 2]];

// --- Channels and joints (any coordinates) -----------------------------------
//
// The ball runs along U-shaped channels, open on top. Where two parts meet, the
// downstream part starts with a socket that the upstream part's plain end slides
// into. The downstream floor is `joint_step` lower, so the ball steps down,
// never up, and the upstream floor rests just above it: the socket's floor is
// flush with the channel's, so every part has a flat bottom to print on.

socket_length = 15;
// A hair's gap between the upstream part's end and the downstream channel, so
// the parts never quite touch end to end.
joint_gap = 0.2;
function joint_step_value() = wall + fit_clearance;
joint_step = joint_step_value();

// A channel whose floor runs from `from` to `to` ([x, y, z] points on the
// floor's top surface, along its middle). With `socket`, it starts with a
// socket for the upstream part.
module channel(from, to, socket = false) {
    run = to - from;
    plan = norm([run[0], run[1]]);
    length = norm(run);
    translate(from)
        rotate([0, 0, atan2(run[1], run[0])])
            rotate([0, -atan2(run[2], plan), 0])
                difference() {
                    union() {
                        start = socket ? joint_gap : 0;
                        translate([start, -channel_inner / 2 - wall, -wall])
                            cube([length - start, channel_inner + 2 * wall, channel_height + wall]);
                        if (socket)
                            translate([-socket_length, -channel_inner / 2 - 2 * wall - fit_clearance, -wall])
                                cube([socket_length + joint_gap + 0.01, channel_inner + 4 * wall + 2 * fit_clearance, channel_height + wall + joint_step]);
                    }
                    translate([-0.01, -channel_inner / 2, 0])
                        cube([length + 0.02, channel_inner, channel_height + 10]);
                    if (socket)
                        translate([-socket_length - 1, -channel_inner / 2 - wall - fit_clearance, 0])
                            cube([socket_length + 1 + joint_gap, channel_inner + 2 * wall + 2 * fit_clearance, channel_height + 10]);
                }
}

// A hanger holding a channel under the playfield: a post up from beside the
// channel's floor point `at`, out in the direction `side_angle` (degrees, in
// plan), ending in a tab that screws to the playfield.
module hanger(at, side_angle) {
    post = [5, 8];  // [out from the channel, along it]
    translate([at[0], at[1], 0])
        rotate([0, 0, side_angle])
            translate([channel_inner / 2 + wall - 0.01, 0, 0])
                difference() {
                    union() {
                        translate([0, -post[1] / 2, at[2] - wall]) cube([post[0], post[1], -at[2] + wall]);
                        // The tab, with a 45-degree chamfer under it so it
                        // prints without supports.
                        hull() {
                            translate([0, -7, -3]) cube([14, 14, 3]);
                            translate([0, -post[1] / 2, -3 - 9]) cube([post[0], post[1], 0.01]);
                        }
                    }
                    translate([9, 0, -3]) wood_screw_hole(3);
                }
}

// --- The runs to the outhole and the ball lift (playfield coordinates) -------
//
// Every joint steps the floor down by `joint_step`: each segment starts with a
// socket round the end of the part before it.

// Segment `k` (1 to `count`) of a run whose floor goes from `a` to `b`, split
// evenly and stepped down at each joint: [from, to].
function segment_ends(a, b, count, k) =
    let(step = [0, 0, -joint_step * k])
    [a + (b - a) * (k - 1) / count + step, a + (b - a) * k / count + step];

// The return run: from the collector's exit down the table to the outhole
// funnel, level with the playfield (its own slope rolls the ball).
return_start = [trapdoor_x, trapdoor_y, 0] + exit_end;
funnel_size = channel_inner + 6;
return_end = [outhole_x, outhole_y + funnel_size / 2 + wall + joint_gap, return_start[2]];
return_plan = norm([return_end[0] - return_start[0], return_end[1] - return_start[1]]);
return_segments = ceil(return_plan / (max_print_length - socket_length - 10));
return_last = segment_ends(return_start, return_end, return_segments, return_segments)[1];

// The outhole funnel: the return run comes in at the back, drained balls drop
// in from above, and the serve run leaves from its right towards the lift.
funnel_inlet = [outhole_x, outhole_y + funnel_size / 2, return_last[2] - joint_step];
funnel_centre = [outhole_x, outhole_y, funnel_inlet[2] - 2];

// The serve run: from the outhole funnel across to the ball lift, sloping down.
serve_slope = 6;
lift_size = channel_inner + 6;
serve_angle = atan2(lift_hole_y - outhole_y, lift_hole_x - outhole_x);
serve_dir = [cos(serve_angle), sin(serve_angle), 0];
// The funnel's outlet: a stub through its right wall, long enough for the
// serve run's socket, and at the serve run's slope so the two line up.
funnel_outlet = funnel_centre + serve_dir * (funnel_size / 2 - 1) - [0, 0, 1];
funnel_outlet_length = wall + 1 + socket_length + 3;
serve_start = funnel_outlet + serve_dir * funnel_outlet_length - [0, 0, funnel_outlet_length * tan(serve_slope)];
serve_plan_end = [lift_hole_x, lift_hole_y, 0] - serve_dir * (lift_size / 2 + wall);
serve_plan = norm([serve_plan_end[0] - serve_start[0], serve_plan_end[1] - serve_start[1]]);
serve_segments = ceil(serve_plan / (max_print_length - socket_length - 10));
serve_end = [serve_plan_end[0], serve_plan_end[1], serve_start[2] - serve_plan * tan(serve_slope)];
serve_last = segment_ends(serve_start, serve_end, serve_segments, serve_segments)[1];

// The ball lift: the ball waits at the bottom of a tube under the hole beside
// the shooter lane, on the plunger of the ball-serve coil.
lift_inlet = serve_last - [0, 0, joint_step];
// Its inlet runs on at the serve run's slope to the tube.
lift_inlet_length = lift_size / 2 + wall - (ball_diameter + 3) / 2 + 2;
lift_centre = [lift_hole_x, lift_hole_y, lift_inlet[2] - lift_inlet_length * tan(serve_slope)];
lift_tube_inner = ball_diameter + 3;
assert(lift_hole_diameter >= lift_tube_inner,
    "The ball lift's hole is too small for the ball to come up through it: check lift_hole_diameter in measurements.scad.");

// --- The ball lift's exit (playfield coordinates, on top of the playfield) ---
//
// Above the hole, the tube bends from straight up to sideways, towards the
// shooter lane, so the kicked ball can only come out heading for the lane. It
// then rolls down a short ramp, through the opening in the lane's wall, into
// the lane. The ramp only slopes down into the lane, so a ball rolling back
// down the lane can't climb into it. A kick too weak to get the ball round the
// bend drops it back down the tube; with #45, the game then serves it again.
lane_x = lane_wall_x + lane_wall_thickness;   // the lane's side
// The bend: the ball's centre follows a quarter-circle of this radius, from
// the playfield's surface over the hole, up and over to the lane. As big as
// fits: before the lane's wall, under the headroom, and low enough at the top
// for the ball to roll through the wall's opening.
bend_radius = min(
    lane_wall_x - fit_clearance - lift_hole_x,
    lift_headroom - lift_tube_inner / 2 - wall,
    lane_opening_height - ball_diameter / 2 - 1.5);
bend_centre = [lift_hole_x + bend_radius, lift_hole_y, playfield_thickness];
// The ramp: from the end of the bend down to the lane's side, where it ends
// ramp_end thick, a small step down into the lane.
ramp_start = lift_hole_x + bend_radius;
ramp_lip = bend_radius - lift_tube_inner / 2;   // above the playfield
ramp_end = 0.8;
ramp_width = lane_opening_width - 2 * fit_clearance;
// The top of the ramp's floor, above the playfield, at `x`.
function ramp_floor(x) = ramp_end + (ramp_lip - ramp_end) * (lane_x - x) / (lane_x - ramp_start);
assert(ramp_lip >= ramp_end + 1,
    "There isn't room for the ball lift's bend: the hole needs to be further from the shooter lane's wall, or there needs to be more headroom or a taller opening. Check lift_hole_x, lane_wall_x, lift_headroom and lane_opening_height in measurements.scad.");
assert(lane_opening_width >= ball_diameter + 4,
    "The opening in the shooter lane's wall is too narrow for the ball: check lane_opening_width in measurements.scad.");

// Every part that screws to the playfield must land on it.
module assert_on_playfield(name, centre, half) {
    assert(centre[0] - half[0] >= 0 && centre[0] + half[0] <= playfield_width
        && centre[1] - half[1] >= 0 && centre[1] + half[1] <= playfield_length,
        str("The ", name, " would hang off the edge of the playfield: check its position and playfield_width/playfield_length in measurements.scad."));
}
