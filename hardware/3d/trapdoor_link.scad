// Trapdoor link: print three. It joins the rib under a flap to the crank on
// that trapdoor's servo, with an M2 x 10 mm screw through each end: one into
// the rib, one into the crank.
//
// Print it flat, no supports.

include <common.scad>

link();

module link() {
    difference() {
        hull() {
            cylinder(r = 3.5, h = 3);
            translate([link_length, 0, 0]) cylinder(r = 3.5, h = 3);
        }
        for (x = [0, link_length]) translate([x, 0, -1]) cylinder(d = m2_free, h = 5);
    }
}
