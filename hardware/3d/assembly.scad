// How a trapdoor's printed parts fit together under the playfield. Open this to
// see what goes where; it isn't printed. The playfield and the servo are drawn
// see-through.

include <common.scad>
use <trapdoor_frame.scad>
use <trapdoor_flap.scad>
use <trapdoor_lifter.scad>
use <trapdoor_chute.scad>

/* [View] */
// Show the playfield (turn off to see underneath).
show_playfield = true;
// Show the flap open (as when the game opens the trapdoor).
open = false;
// Angle the flap drops to when open.
open_angle = 40; // [10:5:60]
// Cut through the middle of the lifter, to see inside the frame.
section = false;

intersection() {
    union() {
        if (show_playfield) color("burlywood", 0.35) playfield();
        color("dimgray") frame();
        color("gold")
            translate([0, pin_y, pin_z]) rotate([open ? open_angle : 0, 0, 0]) translate([0, -pin_y, -pin_z]) flap();
        // The lifter swings about the servo's shaft: straight up when the flap
        // is closed, back towards the hinge to let it open.
        color("tomato")
            translate([lifter_x + lifter_thickness, servo_shaft_y, servo_shaft_z])
                rotate([open ? -lifter_angle(open_angle) : 0, 0, 0]) rotate([0, -90, 0]) lifter();
        color("steelblue", 0.5) servo();
        color("seagreen") chute();
    }
    if (section)
        translate([lifter_x + lifter_thickness / 2 - 400, -200, -200]) cube(400);
    else
        cube(1000, center = true);
}

// How far the lifter turns back from upright to hold the flap open at `theta`
// degrees: its tip (3.5 mm radius) stays against the flap's underside.
function lifter_angle(theta) =
    let(
        tip = lifter_length - 3.5,
        // A point on the flap's underside below the pin, and the direction
        // pointing down out of the underside, both turned with the flap.
        a = [pin_y + (playfield_thickness - flap_thickness - pin_z) * -sin(theta),
             pin_z + (playfield_thickness - flap_thickness - pin_z) * cos(theta)],
        down = [sin(theta), -cos(theta)],
        gaps = [for (phi = [0 : 0.25 : 90])
            let(c = [servo_shaft_y + tip * sin(phi), servo_shaft_z + tip * cos(phi)])
            abs((c - a) * down - 3.5)],
        best = min(gaps)
    )
    search(best, gaps)[0] * 0.25;

echo(str("Lifter turns back ", lifter_angle(open_angle), " degrees for a ", open_angle, " degree flap."));

module playfield() {
    difference() {
        translate([-hole_width * 1.5, -hole_length * 1.5, 0])
            cube([hole_width * 3, hole_length * 3, playfield_thickness]);
        translate([-hole_width / 2, -hole_length / 2, -1]) cube([hole_width, hole_length, playfield_thickness + 2]);
    }
}

module servo() {
    body_y0 = servo_shaft_y - servo_shaft_from_end;
    translate([servo_top_x, body_y0, servo_shaft_z - servo_body[1] / 2])
        cube([servo_body[2], servo_body[0], servo_body[1]]);
}
