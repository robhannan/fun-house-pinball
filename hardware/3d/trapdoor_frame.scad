// Trapdoor frame: one piece for all three trapdoors. Its sleeves slide up into
// the three holes from under the playfield, and it screws on with wood screws
// through its flange. Each sleeve holds a flap's hinge pin.
//
// Print it as it is: flange down on the bed. No supports needed.

include <common.scad>

frame();

module frame() {
    span = 2 * trapdoor_spacing + outer_width;
    difference() {
        union() {
            for (dx = trapdoor_offsets)
                translate([dx - outer_width / 2, -outer_length / 2, 0])
                    cube([outer_width, outer_length, playfield_thickness]);
            translate([-span / 2 - flange_width, -outer_length / 2 - flange_width, -flange_thickness])
                cube([span + 2 * flange_width, outer_length + 2 * flange_width, flange_thickness]);
        }
        for (dx = trapdoor_offsets) translate([dx, 0, 0]) {
            // The hole the ball drops through.
            translate([-inner_width / 2, -inner_length / 2, -flange_thickness - 1])
                cube([inner_width, inner_length, playfield_thickness + flange_thickness + 2]);
            // The hinge pin: a snug fit, so it stays put (drill it out if tight).
            translate([0, pin_y, pin_z])
                rotate([0, 90, 0])
                    cylinder(d = hinge_pin_diameter + 0.1, h = outer_width + 2, center = true);
        }
        // Wood screws round the flange.
        for (x = [-1, 1], y = [-1, 1])
            translate([x * (span / 2 + flange_width / 2), y * (outer_length / 2 + flange_width / 2), -flange_thickness])
                wood_screw_hole(flange_thickness);
        for (x = [-0.5, 0.5])
            translate([x * trapdoor_spacing, -outer_length / 2 - flange_width / 2, -flange_thickness])
                wood_screw_hole(flange_thickness);
        // Blind pilot holes for the collector's screws (M3 self-tapping).
        for (p = collector_screws)
            translate([p[0], p[1], -flange_thickness - 0.01]) cylinder(d = 2.5, h = flange_thickness - 0.5);
    }
}
