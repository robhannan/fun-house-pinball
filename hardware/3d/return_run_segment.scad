// Return run: the channel that carries the ball from the trapdoor collector down
// the table to the outhole funnel, level with the playfield (the playfield's
// own slope keeps the ball rolling). It's in segments that each fit a printer:
// print every one (choose which with "segment"). Each starts with a socket that
// the part before it slides into, and has two hangers that screw to the
// playfield.
//
// Print it as it is: floor on the bed. No supports needed.

include <common.scad>

/* [Segment] */
// Which segment, from the trapdoors' end.
segment = 1; // [1:1:12]

echo(str("SEGMENTS ", return_segments));
assert(segment <= return_segments, str("There are only ", return_segments, " return-run segments."));

ends = segment_ends(return_start, return_end, return_segments, segment);
// Moved to the origin for printing.
translate([-ends[0][0], -ends[0][1], -ends[0][2] + 2 * wall + fit_clearance]) run_segment(ends);

module run_segment(ends) {
    a = ends[0];
    b = ends[1];
    side = atan2(b[1] - a[1], b[0] - a[0]);
    channel(a, b, socket = true);
    hanger(a + (b - a) * 0.3, side + 90);
    hanger(a + (b - a) * 0.7, side - 90);
}
