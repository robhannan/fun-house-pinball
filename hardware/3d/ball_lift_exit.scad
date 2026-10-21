// Ball-lift exit: one, printed in two halves (choose which with "segment").
// It screws on top of the playfield over the ball lift's hole, beside the
// shooter lane (under the apron). Above the hole, the tube bends from straight
// up to sideways, so the kicked ball comes out heading for the lane, and rolls
// down the ramp through the opening in the lane's wall. The ramp only slopes
// down into the lane, so a ball rolling back down the lane can't get in.
//
// The halves are split down the middle of the bend. Screw each one to the
// playfield, and join them through the tab on top with an M3 x 16 mm
// self-tapping screw, from the front half into the back.
//
// Print each half as it is: its flat, cut face on the bed. No supports needed.

include <common.scad>

/* [Half] */
// Which half: 1 is the back (up the table), 2 the front.
segment = 1; // [1:1:2]

echo("SEGMENTS 2");

// Laid on its cut face for printing.
if (segment == 1)
    translate([-lift_hole_x, 0, -lift_hole_y - joint_gap / 2]) rotate([90, 0, 0]) ball_lift_exit_half(1);
else
    translate([-lift_hole_x, 0, lift_hole_y - joint_gap / 2]) rotate([-90, 0, 0]) ball_lift_exit_half(2);

// Both halves, in place.
module ball_lift_exit() {
    ball_lift_exit_half(1);
    ball_lift_exit_half(2);
}

// The back (1) or front (2) half, in place.
module ball_lift_exit_half(k) {
    intersection() {
        ball_lift_exit_whole();
        if (k == 1) translate([-1000, lift_hole_y + joint_gap / 2, -1000]) cube(2000);
        else translate([-1000, lift_hole_y - joint_gap / 2 - 2000, -1000]) cube(2000);
    }
}

// The quarter-circle bend round bend_centre, from straight up over the hole
// to sideways towards the lane, as a solid tube of radius `r`.
module bend(r) {
    translate(bend_centre) rotate([90, 0, 0]) rotate([0, 0, 90])
        rotate_extrude(angle = 90) intersection() {
            translate([bend_radius, 0]) circle(r = r);
            // Solid right up to the axis, where a thick tube would cross it.
            translate([0, -r]) square([bend_radius + r, 2 * r]);
        }
}

module ball_lift_exit_whole() {
    t = playfield_thickness;
    outer = lift_tube_inner / 2 + wall;
    hx = lift_hole_x;
    hy = lift_hole_y;
    plate_half = outer + 12;
    screw_y = outer + 6;
    // The tab joining the halves, on the outside of the bend.
    tab = bend_centre + (bend_radius + outer + 4) * [-cos(45), 0, sin(45)];
    difference() {
        union() {
            bend(outer);
            // The base, screwed to the playfield.
            translate([hx - outer - 10, hy - plate_half, t]) cube([ramp_start - (hx - outer - 10), 2 * plate_half, 3]);
            // The ramp, through the opening in the lane's wall.
            translate([0, hy + ramp_width / 2, t]) rotate([90, 0, 0]) linear_extrude(ramp_width)
                polygon([[ramp_start - 1, 0], [lane_x, 0], [lane_x, ramp_end], [ramp_start - 1, ramp_floor(ramp_start - 1)]]);
            translate(tab - [5, 8, 5]) cube([10, 16, 10]);
        }
        // Inside: the bend, the hole it rises from, and on towards the lane.
        bend(lift_tube_inner / 2);
        translate([hx, hy, t - 1]) cylinder(d = lift_tube_inner, h = 4);
        translate([ramp_start - 0.01, hy, bend_centre[2] + bend_radius]) rotate([0, 90, 0])
            cylinder(d = lift_tube_inner, h = lane_x - ramp_start + 1);
        // The screw joining the halves: a free fit through the front half,
        // tapped into the back.
        translate(tab - [0, 9, 0]) rotate([-90, 0, 0]) cylinder(d = 3.4, h = 9);
        translate(tab - [0, 0.5, 0]) rotate([-90, 0, 0]) cylinder(d = 2.5, h = 9.5);
        // Wood screws, down into the playfield: two each side.
        for (s = [-1, 1], x = [hx - 8, ramp_start - 6])
            translate([x, hy + s * screw_y, t + 3]) mirror([0, 0, 1]) wood_screw_hole(3);
    }
}
