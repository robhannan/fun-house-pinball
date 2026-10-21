// Ball lift: screws under the hole beside the shooter lane. The serve run
// brings every ball here, where it waits on the ball-serve coil's plunger and
// holds the outhole switch closed. When the game serves a ball, the coil kicks
// it up the tube into the ball-lift exit on top of the playfield, which turns
// it into the shooter lane.
//
// The coil assembly bolts under the base through the two slots (drill them to
// suit if your coil's screws are elsewhere). A lever microswitch screws to the
// tab on the tube's back, its lever poking in through the slot to touch the
// waiting ball.
//
// Print it upside down (the flange on the bed), with supports ("supports: on"
// in the slicer, or ask the printing service).

include <common.scad>

translate([-lift_hole_x, -lift_hole_y, 0]) rotate([180, 0, 0]) ball_lift();

module ball_lift() {
    outer = lift_tube_inner + 2 * wall;
    // The cup sits in a recess, its seat level with the inlet's floor.
    recess = 10;
    base = lift_centre[2] - recess - 4;
    plate = outer + 24;
    difference() {
        union() {
            // The tube, up to the playfield.
            translate([lift_hole_x, lift_hole_y, base]) cylinder(d = outer, h = -base);
            // The flange round the hole.
            translate([lift_hole_x - plate / 2, lift_hole_y - plate / 2, -3]) cube([plate, plate, 3]);
            // The base the coil bolts to.
            translate([lift_hole_x - lift_coil_screw_spacing / 2 - 8, lift_hole_y - 10, base]) cube([lift_coil_screw_spacing + 16, 20, 4]);
            // In from the serve run.
            channel(lift_inlet, lift_centre - serve_dir * (lift_tube_inner / 2 - 2), socket = true);
            // The switch's tab, on the back of the tube.
            translate([lift_hole_x - 12, lift_hole_y + outer / 2 - 1, lift_centre[2] - 4])
                difference() {
                    cube([24, 2.5 + 1, 16]);
                    for (x = [7.25, 16.75]) translate([x, -1, 9]) rotate([-90, 0, 0]) cylinder(d = 2.2, h = 6);
                }
        }
        // Inside the tube, including the cup's recess.
        translate([lift_hole_x, lift_hole_y, lift_centre[2] - recess]) cylinder(d = lift_tube_inner, h = 200);
        // Where the inlet joins it.
        translate([lift_hole_x, lift_hole_y, lift_centre[2]]) rotate([0, 0, serve_angle + 180])
            translate([0, -channel_inner / 2, 0]) cube([outer, channel_inner, doorway_height + 6]);
        // The plunger comes up through the base.
        translate([lift_hole_x, lift_hole_y, base - 1]) cylinder(d = lift_plunger_diameter + 1, h = 10);
        // The coil's screws.
        for (s = [-1, 1])
            hull() for (dy = [-3, 3])
                translate([lift_hole_x + s * lift_coil_screw_spacing / 2, lift_hole_y + dy, base - 1]) cylinder(d = 3.4, h = 10);
        // The switch's slot.
        translate([lift_hole_x - 2, lift_hole_y + lift_tube_inner / 2 - 1, lift_centre[2] + 2]) cube([4, wall + 3, 12]);
        // Wood screws.
        for (x = [-1, 1], y = [-1, 1])
            translate([lift_hole_x + x * (plate / 2 - 6), lift_hole_y + y * (plate / 2 - 6), -3]) wood_screw_hole(3);
    }
}
