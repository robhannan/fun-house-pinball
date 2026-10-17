// Trapdoor collector: one piece under all three trapdoors. Each trapdoor has its
// own catch box, kept clear of the flap as it swings down and of the drive
// beside it. A ball lands in the box, rolls forward over that trapdoor's
// microswitch and out into the trough across the front, which runs it to the
// middle and out down the table into the return run.
//
// A lever microswitch (KW11 type) screws to the tab under each box's floor,
// with its lever poking up through the slot. The collector screws to the
// frame's front flange with M3 x 6 mm self-tapping screws.
//
// Print it upside down (the flat rim on the bed), with supports ("supports:
// on" in the slicer, or ask the printing service).

include <common.scad>

collector();

// The front rim, under the frame's front flange.
rim_y = [-outer_length / 2 - flange_width, catch_front - wall];

module collector() {
    difference() {
        union() {
            for (dx = trapdoor_offsets) translate([dx, 0, 0]) catch_box();
            trough();
            // The rim that screws to the frame.
            translate([-trapdoor_spacing + catch_left - wall, rim_y[0], collector_top - 3])
                cube([2 * trapdoor_spacing + catch_right - catch_left + 2 * wall, rim_y[1] - rim_y[0], 3]);
        }
        for (dx = trapdoor_offsets) translate([dx, 0, 0]) {
            catch_inside();
            // The outlet through the front, into the trough.
            translate([(catch_left + catch_right) / 2 - channel_inner / 2, catch_front - 2 * wall - 1, catch_floor_front - 0.01])
                cube([channel_inner, 2 * wall + 2, doorway_height]);
            // The switch's slot.
            translate([(catch_left + catch_right) / 2 - 2, catch_front + 6, catch_floor_front - 10])
                cube([4, 14, 20]);
        }
        // The trough's exit, through its front wall.
        translate([-channel_inner / 2, trough_y - channel_inner / 2 - wall - 1, trough_floor_centre])
            cube([channel_inner, wall + 2, doorway_height]);
        for (p = collector_screws)
            translate([p[0], p[1], collector_top - 4]) cylinder(d = 3.4, h = 5);
    }
}

// The flap's swing, as a curve: the highest a wall inside the flap's width
// can reach at `y` without being hit.
function wall_top(y) =
    let(d = pin_y - y)
    d >= sweep_radius ? collector_top : min(collector_top, pin_z - sqrt(sweep_radius * sweep_radius - d * d));

module catch_box() {
    bottom = catch_floor_front - wall - 14;
    // Left and front walls: outside the flap's swing, up to the frame.
    translate([catch_left - wall, catch_front - wall, bottom])
        cube([wall, catch_back - catch_front + 2 * wall, collector_top - bottom]);
    translate([catch_left - wall, catch_front - wall, bottom])
        cube([catch_right - catch_left + 2 * wall, wall, collector_top - bottom]);
    // Right wall: inside the flap's width, so its top follows the swing.
    ys = [for (i = [0 : 12]) catch_front - wall + (catch_back - catch_front + 2 * wall) * i / 12];
    multmatrix([[0, 0, 1, catch_right], [1, 0, 0, 0], [0, 1, 0, 0]])
        linear_extrude(height = wall)
            polygon(concat([[ys[0], bottom]], [for (y = ys) [y, wall_top(y)]], [[ys[12], bottom]]));
    // Back wall: low, under where the flap hangs.
    translate([catch_left - wall, catch_back, bottom])
        cube([catch_right - catch_left + 2 * wall, wall, catch_floor_back + 2 - bottom]);
    // The floor, sloping forward.
    hull() {
        translate([catch_left, catch_back - 0.01, catch_floor_back - wall]) cube([catch_right - catch_left, 0.01, wall]);
        translate([catch_left, catch_front, catch_floor_front - wall]) cube([catch_right - catch_left, 0.01, wall]);
    }
    // The switch's tab, under the floor.
    translate([(catch_left + catch_right) / 2 + 4, catch_front + 4, bottom])
        difference() {
            cube([2.5, 24, catch_floor_front - wall - bottom]);
            for (y = [8.25, 17.75]) translate([-1, y, 5]) rotate([0, 90, 0]) cylinder(d = 2.2, h = 5);
        }
}

module catch_inside() {
    // Everything above the floor, inside the walls.
    hull() {
        translate([catch_left, catch_back - 0.01, catch_floor_back]) cube([catch_right - catch_left, 0.01, 60]);
        translate([catch_left, catch_front, catch_floor_front]) cube([catch_right - catch_left, 0.01, 60]);
    }
}

module trough() {
    ends = [for (dx = [-trapdoor_spacing, trapdoor_spacing]) dx + (catch_left + catch_right) / 2];
    for (e = ends) {
        // Closed at the far end, open at the middle.
        channel([e + sign(e) * (channel_inner / 2), trough_y, trough_floor_end], [0, trough_y, trough_floor_centre]);
        translate([e + sign(e) * channel_inner / 2 + (e > 0 ? 0 : -wall), trough_y - channel_inner / 2 - wall, trough_floor_end - wall])
            cube([wall, channel_inner + 2 * wall, channel_height + wall]);
    }
    // Starts at the trough's front wall, so it doesn't stick into the trough.
    channel(exit_start - [0, wall - 0.01, 0], exit_end);
}
