// Ball-lift cup: one. It slides onto the top of the ball-serve coil's plunger,
// clamped by an M3 x 8 mm grub screw in the hole in its side, and cradles the
// waiting ball, so the coil kicks it straight up the tube. Assumed (*verify*):
// a plain plunger end.
//
// Print it as it is: flat end on the bed. No supports needed.

include <common.scad>

// The grub screw's height up the cup, and the plunger hole's depth.
grub_height = 3;
socket_depth = 6;

cup();

module cup() {
    height = 10;
    difference() {
        cylinder(d = lift_tube_inner - 1.5, h = height);
        // A seat for the ball.
        translate([0, 0, height + ball_diameter / 2 - 3]) sphere(d = ball_diameter + 1);
        // The plunger.
        translate([0, 0, -1]) cylinder(d = lift_plunger_diameter + 0.3, h = socket_depth + 1);
        // An M3 grub screw through the side, tapped into the plastic, clamps
        // the cup to the plunger so it can't fly up the tube with the ball.
        translate([0, 0, grub_height]) rotate([0, 90, 0]) cylinder(d = 2.5, h = lift_tube_inner);
    }
}
