// Trapdoor frame: print three, one per trapdoor. Its thin sleeve slides up into
// the hole from under the playfield, and it screws on with wood screws through
// its flange. The sleeve holds the flap's hinge pin.
//
// Print it as it is: flange down on the bed, no supports. The sleeve's walls
// are 1 mm thick, so use a 0.4 mm nozzle (most printers have one).

include <common.scad>

frame();

module frame() {
    difference() {
        union() {
            translate([-outer_width / 2, -outer_length / 2, 0])
                cube([outer_width, outer_length, playfield_thickness]);
            translate([-outer_width / 2 - flange_width, -outer_length / 2 - flange_width, -flange_thickness])
                cube([outer_width + 2 * flange_width, outer_length + 2 * flange_width, flange_thickness]);
        }
        // The hole the ball drops through.
        translate([-inner_width / 2, -inner_length / 2, -flange_thickness - 1])
            cube([inner_width, inner_length, playfield_thickness + flange_thickness + 2]);
        // The hinge pin: a snug fit through both walls, held in by the wood
        // either side (drill it out if tight).
        translate([0, pin_y, pin_z])
            rotate([0, 90, 0])
                cylinder(d = hinge_pin_diameter + 0.1, h = outer_width + 2, center = true);
        // Wood screws, one in each corner of the flange.
        for (x = [-1, 1], y = [-1, 1])
            translate([x * (outer_width / 2 + flange_width / 2), y * (outer_length / 2 + flange_width / 2), -flange_thickness])
                wood_screw_hole(flange_thickness);
        // A blind pilot hole for the collector's screw (M3 self-tapping).
        translate([collector_screws[1][0], collector_screws[1][1], -flange_thickness - 0.01])
            cylinder(d = 2.5, h = flange_thickness - 0.5);
    }
}
