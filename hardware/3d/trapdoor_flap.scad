// Trapdoor flap: print three. It sits flush in the top of a frame sleeve,
// pivoting on the hinge pin along its back edge. The rib underneath its right
// edge joins the link to the servo, which holds it flush when closed; open, it
// hangs straight down.
//
// Print it top face down on the bed, no supports. Paint it, or glue a print of
// the artwork on top.

include <common.scad>

// Flipped so the top face is on the bed.
translate([0, 0, playfield_thickness]) rotate([180, 0, 0]) flap();

module flap() {
    difference() {
        union() {
            translate([-flap_width / 2, flap_front, playfield_thickness - flap_thickness])
                cube([flap_width, flap_reach, flap_thickness]);
            // The knuckle round the hinge pin.
            translate([0, pin_y, pin_z]) rotate([0, 90, 0])
                cylinder(r = knuckle_radius, h = flap_width, center = true);
            // The rib, under the right edge.
            hull() {
                translate([rib_x[0], pin_y + rib_hole[0] - 3, playfield_thickness - flap_thickness - 0.01])
                    cube([3, 6, 0.01]);
                translate([rib_x[0], pin_y + rib_hole[0], pin_z + rib_hole[1]])
                    rotate([0, 90, 0]) cylinder(r = 3, h = 3);
            }
        }
        // A free fit on the pin.
        translate([0, pin_y, pin_z]) rotate([0, 90, 0])
            cylinder(d = hinge_pin_diameter + 0.5, h = flap_width + 2, center = true);
        // Keep the top flat where the knuckle would poke above it.
        translate([-flap_width, pin_y - 5, playfield_thickness]) cube([2 * flap_width, 10, 5]);
        // The link's pivot: an M2 screw, tapped into the rib.
        translate([rib_x[0] - 1, pin_y + rib_hole[0], pin_z + rib_hole[1]])
            rotate([0, 90, 0]) cylinder(d = m2_tap, h = 5);
    }
}
