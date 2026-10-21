// Trapdoor servo hanger: print three. Each screws to the underside of the
// playfield behind its trapdoor, and holds that trapdoor's MG90S servo with its
// shaft pointing left, towards the link.
//
// Print it on its side: the big flat plate (the one the servo screws to) on
// the bed. No supports needed.

include <common.scad>

// Laid on its side for printing.
rotate([0, -90, 0]) translate([-hanger_plate_x, 0, 0]) hanger_part();

hanger_plate_x = servo_top_x + servo_tab_from_top;

module hanger_part() {
    body_front = servo_shaft[0] - servo_shaft_from_end;
    body_centre = body_front + servo_body[0] / 2;
    plate_length = servo_tab_spacing + 10;
    y0 = body_centre - plate_length / 2;
    bottom = servo_shaft[1] - servo_body[1] / 2 - 5;
    difference() {
        union() {
            // The plate the servo's tabs screw to.
            translate([hanger_plate_x, y0, bottom]) cube([3, plate_length, -bottom]);
            // The top plate, screwed to the playfield.
            translate([hanger_plate_x, y0, -4]) cube([14, plate_length, 4]);
            // Gussets.
            for (y = [y0, y0 + plate_length - 3])
                hull() {
                    translate([hanger_plate_x, y, -4]) cube([14, 3, 0.01]);
                    translate([hanger_plate_x, y, bottom]) cube([3, 3, 0.01]);
                }
        }
        // The servo's body passes through.
        translate([hanger_plate_x - 1, body_front - 0.3, servo_shaft[1] - servo_body[1] / 2 - 0.3])
            cube([5, servo_body[0] + 0.6, servo_body[1] + 0.6]);
        // Its tab screws.
        for (s = [-1, 1])
            translate([hanger_plate_x - 1, body_centre + s * servo_tab_spacing / 2, servo_shaft[1]])
                rotate([0, 90, 0]) cylinder(d = servo_tab_hole, h = 5);
        // Wood screws.
        for (y = [y0 + 8, y0 + plate_length - 8])
            translate([hanger_plate_x + 9, y, -4]) wood_screw_hole(4);
    }
}
