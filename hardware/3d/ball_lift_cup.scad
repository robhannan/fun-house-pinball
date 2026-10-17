// Ball-lift cup: one. It pushes onto the top of the ball-serve coil's plunger
// and cradles the waiting ball, so the coil kicks it straight up the tube.
//
// Print it as it is: flat end on the bed. No supports needed.

include <common.scad>

cup();

module cup() {
    height = 10;
    difference() {
        cylinder(d = lift_tube_inner - 1.5, h = height);
        // A seat for the ball.
        translate([0, 0, height + ball_diameter / 2 - 3]) sphere(d = ball_diameter + 1);
        // The plunger.
        translate([0, 0, -1]) cylinder(d = lift_plunger_diameter + 0.3, h = 7);
    }
}
