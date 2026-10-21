// Trapdoor crank: print three. It screws to the single-arm horn that comes with
// each MG90S servo (use whichever of the small holes line up with the horn's),
// and turns the link that opens and closes the flap. The link's screw goes
// into the hole at the far end.
//
// Print it flat, no supports.

include <common.scad>

crank();

module crank() {
    difference() {
        hull() {
            cylinder(d = 10, h = 3);
            translate([crank_radius, 0, 0]) cylinder(d = 7, h = 3);
        }
        // The horn's centre screw.
        translate([0, 0, -1]) cylinder(d = 2.6, h = 5);
        // Small holes to screw the crank to the horn.
        for (x = [4 : 2.5 : 15]) translate([x, 0, -1]) cylinder(d = 1.3, h = 5);
        // The link's pivot, tapped for an M2 screw.
        translate([crank_radius, 0, -1]) cylinder(d = m2_tap, h = 5);
    }
}
