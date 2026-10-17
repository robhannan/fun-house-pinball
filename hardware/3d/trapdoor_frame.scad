// Trapdoor frame: one per trapdoor. It slides up into the hole from under the
// playfield and screws on with four wood screws through its flange. It holds
// the flap's hinge pin, and the servo that lifts the flap hangs from it.
//
// Print it as it is: flange down on the bed, no supports. Settings for your
// table are in measurements.scad; see docs/3d-printing.md.

include <common.scad>

frame();

module frame() {
    difference() {
        union() {
            // The sleeve, flush with the playfield's top surface.
            translate([-outer_width / 2, -outer_length / 2, 0])
                cube([outer_width, outer_length, playfield_thickness]);
            // The flange, against the playfield's underside.
            translate([-outer_width / 2 - flange_width, -outer_length / 2 - flange_width, -flange_thickness])
                cube([outer_width + 2 * flange_width, outer_length + 2 * flange_width, flange_thickness]);
            servo_mount();
        }
        // The hole the ball drops through.
        translate([-inner_width / 2, -inner_length / 2, -flange_thickness - 1])
            cube([inner_width, inner_length, playfield_thickness + flange_thickness + 2]);
        // The hinge pin: a snug fit, so it stays put (drill it out if tight).
        translate([0, pin_y, pin_z])
            rotate([0, 90, 0])
                cylinder(d = hinge_pin_diameter + 0.1, h = outer_width + 2, center = true);
        // Wood screws at the corners.
        for (x = [-1, 1], y = [-1, 1])
            translate([x * (outer_width / 2 + flange_width / 2), y * (outer_length / 2 + flange_width / 2), -flange_thickness])
                wood_screw_hole(flange_thickness);
        // Blind pilot holes for the chute's screws.
        for (p = chute_screws)
            translate([p[0], p[1], -flange_thickness - 0.01])
                cylinder(d = 2.5, h = flange_thickness - 0.5);
    }
}

// A plate hanging under the flange on the right, that the servo's mounting tabs
// screw to. The servo's body goes through it, its shaft pointing left.
module servo_mount() {
    tab_x = servo_top_x + servo_tab_from_top;
    body_y0 = servo_shaft_y - servo_shaft_from_end;
    body_centre = body_y0 + servo_body[0] / 2;
    plate_length = servo_tab_spacing + 10;
    bottom = servo_shaft_z - servo_body[1] / 2 - 4;
    difference() {
        union() {
            translate([tab_x, body_centre - plate_length / 2, bottom])
                cube([2.5, plate_length, -flange_thickness - bottom]);
            // Gussets back to the flange, so it doesn't flex.
            for (y = [body_centre - plate_length / 2, body_centre + plate_length / 2 - 2.5])
                hull() {
                    translate([tab_x, y, -flange_thickness - 0.01]) cube([2.5, 2.5, 0.01]);
                    translate([tab_x, y, bottom]) cube([2.5, 2.5, 0.01]);
                    translate([tab_x + 8, y, -flange_thickness - 0.01]) cube([0.01, 2.5, 0.01]);
                }
        }
        // The servo's body passes through.
        translate([tab_x - 1, body_y0 - 0.3, servo_shaft_z - servo_body[1] / 2 - 0.3])
            cube([5, servo_body[0] + 0.6, servo_body[1] + 0.6]);
        // Its tab screws.
        for (s = [-1, 1])
            translate([tab_x - 1, body_centre + s * servo_tab_spacing / 2, servo_shaft_z])
                rotate([0, 90, 0]) cylinder(d = servo_tab_hole, h = 5);
    }
}
