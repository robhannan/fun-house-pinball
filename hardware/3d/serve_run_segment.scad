// Serve run: the channel that carries the ball from the outhole funnel across
// to the ball lift, sloping down. Like the return run it's in segments: print
// every one (choose which with "segment"). Each starts with a socket that the
// part before it slides into, and has two hangers that screw to the playfield.
//
// Exported tilted so its floor is flat on the bed. No supports needed.

include <common.scad>

/* [Segment] */
// Which segment, from the outhole's end.
segment = 1; // [1:1:12]

echo(str("SEGMENTS ", serve_segments));
assert(segment <= serve_segments, str("There are only ", serve_segments, " serve-run segments."));

ends = segment_ends(serve_start, serve_end, serve_segments, segment);
// Laid flat for printing: along x, floor level.
rotate([0, -serve_slope, 0]) rotate([0, 0, -serve_angle])
    translate([-ends[0][0], -ends[0][1], -ends[0][2] + 2 * wall + fit_clearance]) serve_segment(ends);

module serve_segment(ends) {
    a = ends[0];
    b = ends[1];
    channel(a, b, socket = true);
    hanger(a + (b - a) * 0.3, serve_angle + 90);
    hanger(a + (b - a) * 0.7, serve_angle - 90);
}
