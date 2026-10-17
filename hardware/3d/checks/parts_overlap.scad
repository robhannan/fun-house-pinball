// Check: no two separately printed parts (or a servo) overlap where they're
// fitted. This must render NOTHING (OpenSCAD then says "Current top level
// object is empty"); anything that appears is an overlap. Run by
// `pnpm run check:3d`.

include <../placement.scad>

// Each part in place, by number.
part_count = 7 + return_segments + serve_segments;
module part(i) {
    if (i == 0) at_trapdoors() frame();
    else if (i == 1) at_trapdoors() collector();
    else if (i <= 4) at_trapdoors() translate([trapdoor_offsets[i - 2], 0, 0]) { hanger_part(); servo_body(); }
    else if (i == 5) outhole_funnel();
    else if (i == 6) ball_lift();
    else if (i < 7 + return_segments)
        run_segment(segment_ends(return_start, return_end, return_segments, i - 6));
    else serve_segment(segment_ends(serve_start, serve_end, serve_segments, i - 6 - return_segments));
}

for (i = [0 : part_count - 1], j = [0 : part_count - 1])
    if (i < j) intersection() { part(i); part(j); }
