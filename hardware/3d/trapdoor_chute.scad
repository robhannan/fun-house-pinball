// Trapdoor chute: one per trapdoor. It screws under the frame with two M3 x 6 mm
// self-tapping screws, catches the ball, and rolls it away towards the outhole.
// A lever microswitch (KW11 type) screws to the tab under its floor, with the
// lever poking up through the slot, so the game knows a ball came through.
//
// Print it upside down (flange on the bed). The channel's floor needs
// supports under its slope; most slicers add them with "supports: on".

include <common.scad>

/* [Chute] */
// Which way the ball rolls out. ("right" isn't offered: the servo is there.)
exit = "front"; // [front, left]
// How far below the frame the funnel drops before the channel starts.
drop = 45; // [35:1:100]
// Slope of the channel, in degrees.
slope = 12; // [5:1:30]
// Length of the channel.
length = 80; // [30:5:250]
// Where the microswitch sits along the channel.
switch_at = 25; // [15:1:60]

channel_width = ball_diameter + 4;
channel_height = ball_diameter + 4;
wall = chute_wall;

chute();

module chute() {
    centre_x = (chute_right - inner_width / 2) / 2;
    along = exit == "front" ? inner_length : chute_opening;
    across = exit == "front" ? chute_opening : inner_length;
    difference() {
        union() {
            translate([centre_x, 0, -flange_thickness])
                rotate([0, 0, exit == "front" ? -90 : 180])
                    funnel_and_channel(along, across);
            flange();
        }
        // Open the top.
        translate([-inner_width / 2, -inner_length / 2, -flange_thickness - 3.5])
            cube([chute_opening, inner_length, 10]);
        translate([centre_x, 0, -flange_thickness])
            rotate([0, 0, exit == "front" ? -90 : 180])
                cavity(along, across);
        for (p = chute_screws)
            translate([p[0], p[1], -flange_thickness - 4]) cylinder(d = 3.4, h = 5);
    }
}

// The plate that screws to the frame's flange.
module flange() {
    x0 = -outer_width / 2 - flange_width;
    y0 = -outer_length / 2 - flange_width;
    translate([x0, y0, -flange_thickness - 3])
        cube([chute_right + wall - x0, outer_length + 2 * flange_width, 3]);
}

// In local coordinates: the opening is centred on the origin, `along` long in
// x and `across` wide in y, its top at z = 0; the ball leaves along +x.
module funnel_and_channel(along, across) {
    // Funnel walls.
    hull() {
        translate([-along / 2 - wall, -across / 2 - wall, -3]) cube([along + 2 * wall, across + 2 * wall, 3]);
        translate([along / 2, -channel_width / 2 - wall, -drop - wall]) cube([wall, channel_width + 2 * wall, channel_height + wall]);
    }
    // Channel walls and floor.
    translate([along / 2, 0, -drop])
        rotate([0, slope, 0]) {
            translate([0, -channel_width / 2 - wall, -wall])
                cube([length, channel_width + 2 * wall, channel_height * 0.7 + wall]);
            // The microswitch tab, under the floor beside the slot.
            translate([switch_at - 2, channel_width / 2 - 8, -wall - 14])
                difference() {
                    cube([24, 2.5, 14]);
                    for (dx = [6.25, 15.75])
                        translate([dx, -1, 5]) rotate([-90, 0, 0]) cylinder(d = 2.2, h = 5);
                }
        }
}

module cavity(along, across) {
    hull() {
        translate([-along / 2, -across / 2, -0.01]) cube([along, across, 1]);
        translate([along / 2 - 0.01, -channel_width / 2, -drop]) cube([0.02, channel_width, channel_height]);
    }
    translate([along / 2, 0, -drop])
        rotate([0, slope, 0]) {
            translate([-1, -channel_width / 2, 0]) cube([length + 2, channel_width, channel_height * 2]);
            // The slot for the switch's lever.
            translate([switch_at, channel_width / 2 - 8 - 4.2, -wall - 1]) cube([14, 4, wall + 2]);
        }
}
