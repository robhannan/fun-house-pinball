// Where every printed part goes under the playfield, in playfield coordinates,
// with each trapdoor's moving parts at any angle. Used by assembly.scad and by
// the collision checks in checks/; it isn't printed.

include <common.scad>
use <trapdoor_frame.scad>
use <trapdoor_flap.scad>
use <trapdoor_link.scad>
use <trapdoor_crank.scad>
use <trapdoor_servo_hanger.scad>
use <trapdoor_collector.scad>
use <return_run_segment.scad>
use <serve_run_segment.scad>
use <outhole_funnel.scad>
use <ball_lift.scad>
use <ball_lift_cup.scad>
use <ball_lift_exit.scad>

// Lays a flat part (drawn along +x, thickness in +z) in the y-z plane at
// across-table position `x`: from point `from` towards `to` ([y, z] points).
module in_drive_plane(x, from, to) {
    u = (to - from) / norm(to - from);
    multmatrix([[0, 0, 1, x], [u[0], -u[1], 0, from[0]], [u[1], u[0], 0, from[1]]]) children();
}

// One trapdoor's moving parts, in trapdoor coordinates, the flap open by
// `angle` degrees.
module trapdoor_moving(angle) {
    translate([0, pin_y, pin_z]) rotate([angle, 0, 0]) translate([0, -pin_y, -pin_z]) flap();
    in_drive_plane(link_x[0], rib_point(angle), crank_point(angle)) link();
    in_drive_plane(crank_x[0], servo_shaft, crank_point(angle)) crank();
}

// A stand-in for an MG90S servo, in trapdoor coordinates.
module servo_body() {
    translate([servo_top_x, servo_shaft[0] - servo_shaft_from_end, servo_shaft[1] - servo_body[1] / 2])
        cube([servo_body[2], servo_body[0], servo_body[1]]);
}

module at_trapdoors() {
    translate([trapdoor_x, trapdoor_y, 0]) children();
}

// Everything that screws to the playfield must land on it.
assert_on_playfield("trapdoor frame", [trapdoor_x, trapdoor_y],
    [trapdoor_spacing + outer_width / 2 + flange_width, outer_length / 2 + flange_width]);
assert_on_playfield("outhole funnel", [outhole_x, outhole_y], [1, 1] * (funnel_size / 2 + wall + 12));
assert_on_playfield("ball lift", [lift_hole_x, lift_hole_y], [1, 1] * ((lift_tube_inner + 2 * wall) / 2 + 12));
assert_on_playfield("shooter lane", [lane_x + lane_width / 2, lift_hole_y], [lane_width / 2, lane_opening_width / 2]);

// Everything that doesn't move, apart from the servos.
module fixed_parts() {
    at_trapdoors() {
        for (dx = trapdoor_offsets) translate([dx, 0, 0]) frame();
        collector();
        for (dx = trapdoor_offsets) translate([dx, 0, 0]) hanger_part();
    }
    for (k = [1 : return_segments]) run_segment(segment_ends(return_start, return_end, return_segments, k));
    outhole_funnel();
    for (k = [1 : serve_segments]) serve_segment(segment_ends(serve_start, serve_end, serve_segments, k));
    ball_lift();
    ball_lift_exit();
}

module servos() {
    at_trapdoors() for (dx = trapdoor_offsets) translate([dx, 0, 0]) servo_body();
}

module moving_parts(angle) {
    at_trapdoors() for (dx = trapdoor_offsets) translate([dx, 0, 0]) trapdoor_moving(angle);
}

// The cup on the coil's plunger, at rest.
module lift_cup_in_place() {
    translate([lift_hole_x, lift_hole_y, lift_centre[2] - 10]) cup();
}
