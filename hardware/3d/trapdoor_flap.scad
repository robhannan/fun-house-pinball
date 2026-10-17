// Trapdoor flap: one per trapdoor. A plate that sits flush in the top of the
// frame, pivoting on the hinge pin along its back (uphill) edge. It rests on
// the servo's lifter arm: up holds it closed, down lets it swing open.
//
// Print it upside down (top face on the bed) for a smooth top, no supports.
// Paint it, or glue a print of the artwork on top.

include <common.scad>

// Flipped so the top face is on the bed.
translate([0, 0, playfield_thickness]) rotate([180, 0, 0]) flap();

module flap() {
    difference() {
        union() {
            translate([-flap_width / 2, flap_front, playfield_thickness - flap_thickness])
                cube([flap_width, pin_y - flap_front, flap_thickness]);
            // The knuckle around the hinge pin.
            translate([0, pin_y, pin_z])
                rotate([0, 90, 0])
                    cylinder(r = knuckle_radius, h = flap_width, center = true);
        }
        // A free fit on the pin.
        translate([0, pin_y, pin_z])
            rotate([0, 90, 0])
                cylinder(d = hinge_pin_diameter + 0.5, h = flap_width + 2, center = true);
        // Keep the top flat where the knuckle would poke above it.
        translate([-flap_width, pin_y - 5, playfield_thickness])
            cube([2 * flap_width, 10, 5]);
    }
}
