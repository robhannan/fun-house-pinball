// Outhole funnel: screws under the outhole. Drained balls drop into it from
// above, and trapdoor balls arrive along the return run at the back; both roll
// out of its right side along the serve run to the ball lift.
//
// Print it upside down (the flange on the bed), with supports ("supports: on"
// in the slicer, or ask the printing service).

include <common.scad>

translate([-outhole_x, -outhole_y, 0]) rotate([180, 0, 0]) outhole_funnel();

module outhole_funnel() {
    plate = funnel_size + 2 * wall + 24;
    top = -3;
    difference() {
        union() {
            // The flange round the outhole.
            translate([outhole_x - plate / 2, outhole_y - plate / 2, top]) cube([plate, plate, 3]);
            // The box under it.
            translate([outhole_x - funnel_size / 2 - wall, outhole_y - funnel_size / 2 - wall, funnel_centre[2] - wall - 1])
                cube([funnel_size + 2 * wall, funnel_size + 2 * wall, top - funnel_centre[2] + wall + 1]);
            // Short stubs through the walls: in from the return run at the
            // back, out to the serve run on the right.
            channel(funnel_inlet + [0, wall, 0], funnel_inlet - [0, 1, 0], socket = true);
            channel(funnel_outlet, serve_start);
        }
        // Inside the box, its floor sloping down towards the outlet on the
        // right.
        hull() {
            translate([outhole_x - funnel_size / 2, outhole_y - funnel_size / 2, funnel_centre[2] + 3]) cube([0.01, funnel_size, 0.01]);
            translate([outhole_x + funnel_size / 2, outhole_y - funnel_size / 2, funnel_centre[2] - 1]) cube([0.01, funnel_size, 0.01]);
            translate([outhole_x - funnel_size / 2, outhole_y - funnel_size / 2, top + 0.01]) cube([funnel_size, funnel_size, 5]);
        }
        // The outhole above.
        translate([outhole_x, outhole_y, top - 1]) cylinder(d = outhole_diameter + 2, h = 5);
        // Openings to the inlet and outlet.
        // (Measured from the box's low floor, so taller than the ball.)
        translate([outhole_x - channel_inner / 2, outhole_y, funnel_centre[2]]) cube([channel_inner, funnel_size, doorway_height + 8]);
        translate([outhole_x, outhole_y, funnel_centre[2] - 1]) rotate([0, 0, serve_angle])
            translate([0, -channel_inner / 2, 0]) cube([funnel_size, channel_inner, doorway_height + 8]);
        for (x = [-1, 1], y = [-1, 1])
            translate([outhole_x + x * (plate / 2 - 6), outhole_y + y * (plate / 2 - 6), top]) wood_screw_hole(3);
    }
}
