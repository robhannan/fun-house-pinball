// Check: a ball can roll the whole way, from each open trapdoor and from the
// outhole to the ball lift and up into the shooter lane, without touching
// anything. This must render NOTHING (OpenSCAD then says "Current top level
// object is empty"); anything that appears is a collision. Run by
// `pnpm run check:3d`.

include <../placement.scad>

// Which stretch to check: "all", or one of the modules below (for finding a
// collision).
stretch = "all";

// A ball 0.3 mm smaller than a real one, so rolling on a floor isn't a
// collision; its centre is `lift` above the floor.
r = ball_diameter / 2 - 0.3;
lift = ball_diameter / 2;

// A ball moving in a straight line between two centres.
module sweep(a, b) {
    hull() {
        translate(a) sphere(r);
        translate(b) sphere(r);
    }
}

// Rolling along floor points (each a little above the floor's surface).
module along(points) {
    for (i = [0 : len(points) - 2]) sweep(points[i] + [0, 0, lift + 0.5], points[i + 1] + [0, 0, lift + 0.5]);
}

// The top of a catch box's sloping floor at `y`.
function catch_floor(y) =
    catch_floor_front + (y - catch_front) / (catch_back - catch_front) * (catch_floor_back - catch_floor_front);
// Each trapdoor's catch box centre, across the table (trapdoor coordinates).
catch_x = [for (dx = trapdoor_offsets) dx + (catch_left + catch_right) / 2];
// The top of the trough's floor at `x` (trapdoor coordinates).
function trough_floor(x) =
    let(end = max(abs(catch_x[0]), abs(catch_x[2])) + channel_inner / 2)
    trough_floor_centre + (trough_floor_end - trough_floor_centre) * min(abs(x) / end, 1);

// Down through each open trapdoor into its catch box, forward over its outlet's
// edge, and down into the trough.
module trapdoors() {
    at_trapdoors() for (cx = catch_x) {
        sweep([cx, (catch_front + catch_back) / 2, playfield_thickness + lift],
              [cx, (catch_front + catch_back) / 2, catch_floor((catch_front + catch_back) / 2) + lift + 1]);
        along([[cx, catch_front + lift, catch_floor(catch_front + lift)],
               [cx, catch_front - 2 * wall, catch_floor_front],
               [cx, trough_y, trough_floor(cx)]]);
    }
}

// Along the trough to its middle, and out of the exit.
module trough() {
    at_trapdoors() {
        for (cx = [catch_x[0], catch_x[2]]) along([[cx, trough_y, trough_floor(cx)], [0, trough_y, trough_floor_centre]]);
        along([[0, trough_y, trough_floor_centre], exit_start + [0, -lift, 0], exit_end]);
    }
}

// The return run, segment by segment.
module return_run() {
    for (k = [1 : return_segments]) {
        e = segment_ends(return_start, return_end, return_segments, k);
        along([e[0] + (e[1] - e[0]) / norm(e[1] - e[0]) * lift, e[1]]);
    }
}

// Into the outhole funnel, from the return run and through the outhole, and
// out to the serve run.
module outhole() {
    sweep([outhole_x, outhole_y, playfield_thickness + lift], funnel_centre + [0, 0, lift + 3]);
    along([return_last + [0, -lift, -joint_step], funnel_centre + [0, 0, 1.5], serve_start]);
}

// The serve run, segment by segment.
module serve_run() {
    for (k = [1 : serve_segments]) {
        e = segment_ends(serve_start, serve_end, serve_segments, k);
        along([e[0] + (e[1] - e[0]) / norm(e[1] - e[0]) * lift, e[1]]);
    }
}

// Into the ball lift, and up the tube into the shooter lane.
module lift() {
    along([serve_last + serve_dir * lift - [0, 0, joint_step], [lift_hole_x, lift_hole_y, lift_centre[2]]]);
    sweep([lift_hole_x, lift_hole_y, lift_centre[2] + lift + 1], [lift_hole_x, lift_hole_y, playfield_thickness + lift]);
}

intersection() {
    union() {
        if (stretch == "all" || stretch == "trapdoors") trapdoors();
        if (stretch == "all" || stretch == "trough") trough();
        if (stretch == "all" || stretch == "return_run") return_run();
        if (stretch == "all" || stretch == "outhole") outhole();
        if (stretch == "all" || stretch == "serve_run") serve_run();
        if (stretch == "all" || stretch == "lift") lift();
    }
    union() {
        fixed_parts();
        moving_parts(flap_open_angle);
        servos();
    }
}
