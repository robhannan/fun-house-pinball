// Trapdoor flap: print three. It sits in the top of a frame's sleeve,
// pivoting on the hinge pin along its back edge. A 1 mm metal plate glued on
// top makes it flush with the playfield: cut each plate to the size this file
// prints in OpenSCAD's console ("Cut the metal plate to ..."). The rib under
// its middle joins the link to the servo, which holds it flush when closed;
// open, it hangs straight down.
//
// Print it top face down on the bed, no supports.

include <common.scad>

echo(str("Cut the metal plate to ", plate_size[0], " x ", plate_size[1], " mm (across x along the table)."));

// Flipped so the top face is on the bed.
translate([0, 0, flap_top]) rotate([180, 0, 0]) flap();

module flap() {
    difference() {
        union() {
            translate([-flap_width / 2, flap_front, flap_top - flap_thickness])
                cube([flap_width, flap_reach, flap_thickness]);
            // The knuckle round the hinge pin.
            translate([0, pin_y, pin_z]) rotate([0, 90, 0])
                cylinder(r = knuckle_radius, h = flap_width, center = true);
            // The rib, under the middle.
            hull() {
                translate([rib_x[0], pin_y + rib_hole[0] - 3, flap_top - flap_thickness - 0.01])
                    cube([rib_x[1] - rib_x[0], 6, 0.01]);
                translate([rib_x[0], pin_y + rib_hole[0], pin_z + rib_hole[1]])
                    rotate([0, 90, 0]) cylinder(r = 3, h = rib_x[1] - rib_x[0]);
            }
        }
        // A free fit on the pin.
        translate([0, pin_y, pin_z]) rotate([0, 90, 0])
            cylinder(d = hinge_pin_diameter + 0.5, h = flap_width + 2, center = true);
        // Keep the top flat where the knuckle would poke above it.
        translate([-flap_width, pin_y - 5, flap_top]) cube([2 * flap_width, 10, 5]);
        // The link's pivot: an M2 screw, tapped into the rib from the link's side.
        translate([rib_x[0] - 1, pin_y + rib_hole[0], pin_z + rib_hole[1]])
            rotate([0, 90, 0]) cylinder(d = m2_tap, h = rib_x[1] - rib_x[0] + 2);
    }
}
