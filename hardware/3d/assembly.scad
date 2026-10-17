// The whole ball return, in place under the playfield. Open this to see what
// goes where; it isn't printed. Drag to turn it, scroll to zoom, and use
// Window > Customizer for the options below.

include <placement.scad>

/* [View] */
// Show the playfield (turn off to see underneath).
show_playfield = false;
// Show the trapdoors open (as when the game opens them).
open = false;
// Cut beside the right trapdoor's drive (removing its servo), to see how it works.
section = false;

angle = open ? flap_open_angle : 0;
cut_x = trapdoor_x + trapdoor_spacing + crank_x[1] + 0.3;

intersection() {
    union() {
        if (show_playfield) color("burlywood", 0.4) playfield();
        color("dimgray") fixed_parts();
        color("gold") moving_parts(angle);
        color("steelblue") servos();
        color("tomato") lift_cup_in_place();
    }
    if (section) translate([cut_x - 2000, -1000, -1000]) cube(2000);
    else translate([-1000, -1000, -1000]) cube(3000);
}

module playfield() {
    difference() {
        translate([0, 0, 0]) cube([lift_hole_x + 60, trapdoor_y + 80, playfield_thickness]);
        at_trapdoors() for (dx = trapdoor_offsets)
            translate([dx - hole_width / 2, -hole_length / 2, -1]) cube([hole_width, hole_length, playfield_thickness + 2]);
        translate([outhole_x, outhole_y, -1]) cylinder(d = outhole_diameter, h = playfield_thickness + 2);
        translate([lift_hole_x, lift_hole_y, -1]) cylinder(d = lift_hole_diameter, h = playfield_thickness + 2);
    }
}
