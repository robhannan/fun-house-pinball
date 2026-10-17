// Check: as the trapdoors swing from closed to hanging open, their flaps, links
// and cranks never touch anything that doesn't move. This must render NOTHING
// (OpenSCAD then says "Current top level object is empty"); anything that
// appears is a collision. Run by `pnpm run check:3d`.

include <../placement.scad>

for (angle = [0.5, 10, 20, 30, 40, 50, 60, 70, 80, 90])
    intersection() {
        moving_parts(angle);
        union() {
            fixed_parts();
            servos();
        }
    }
