// Lifter arm: one per trapdoor. Screw it to the single-arm horn that comes with
// the MG90S servo. The flap rests on its rounded tip: pointing straight up, it
// holds the flap closed and flush; turned down, the flap follows it open.
// Test mode's flaps stage sets both angles.
//
// Print it flat, no supports.

include <common.scad>

lifter();

module lifter() {
    width = 7;
    difference() {
        hull() {
            cylinder(d = width + 2, h = lifter_thickness);
            translate([lifter_length - width / 2, 0, 0]) cylinder(d = width, h = lifter_thickness);
        }
        // The horn's centre screw.
        translate([0, 0, -1]) cylinder(d = 2.6, h = lifter_thickness + 2);
        // A row of small holes: screw through whichever line up with the
        // horn's holes.
        for (x = [4 : 2.5 : 14])
            translate([x, 0, -1]) cylinder(d = 1.3, h = lifter_thickness + 2);
    }
}
