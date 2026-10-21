// Ball-lift exit: one. It screws on top of the playfield over the ball lift's
// hole, beside the shooter lane (under the apron). The ball, kicked up through
// the hole, hits the hood's sloping roof, which turns it towards the lane, and
// rolls down the ramp through the opening in the lane's wall. The ramp only
// slopes down into the lane, so a ball rolling back down the lane can't get in.
//
// Print it as it is, flat bottom on the bed, with supports under the roof
// ("supports: on" in the slicer, or ask the printing service).

include <common.scad>

translate([-lift_hole_x, -lift_hole_y, -playfield_thickness]) ball_lift_exit();

module ball_lift_exit() {
    t = playfield_thickness;
    // A shape drawn in the x-z plane (as [x, height above the playfield]
    // points), made `width` wide across y, centred on the hole.
    module profile(points, width) {
        translate([0, lift_hole_y + width / 2, t]) rotate([90, 0, 0])
            linear_extrude(width) polygon(points);
    }
    difference() {
        union() {
            // The hood.
            profile([[hood_back, 0], [hood_front, 0], [hood_front, hood_high + wall], [hood_back, hood_low + wall]],
                2 * (hood_half + wall));
            // The ramp, through the opening in the lane's wall.
            profile([[hood_front - 1, 0], [lane_x, 0], [lane_x, ramp_end], [hood_front - 1, ramp_floor(hood_front - 1)]],
                ramp_width);
            // Screw tabs, one each side.
            for (s = [-1, 1])
                translate([lift_hole_x - 6, lift_hole_y + s * (hood_half + wall + 6) - 6, t]) cube([12, 12, 3]);
        }
        // Inside the hood, open towards the lane.
        profile([[hood_back + wall, ramp_lip], [ramp_start, ramp_lip], [hood_front + 0.01, ramp_floor(hood_front + 0.01)],
                 [hood_front + 0.01, hood_high], [hood_back + wall, hood_low]],
            2 * hood_half);
        // The hole the ball comes up through.
        translate([lift_hole_x, lift_hole_y, t - 1]) cylinder(d = lift_hole_diameter, h = ramp_lip + 2);
        // Wood screws, down into the playfield.
        for (s = [-1, 1])
            translate([lift_hole_x, lift_hole_y + s * (hood_half + wall + 6), t + 3]) mirror([0, 0, 1]) wood_screw_hole(3);
    }
}
