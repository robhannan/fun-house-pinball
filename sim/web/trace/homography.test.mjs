// Tests for the trace page's photo-to-map fit. Run with: pnpm run test:site
import assert from "node:assert/strict";
import { test } from "node:test";

import { applyHomography, fitHomography, invertHomography } from "./homography.js";

// A photo of a 520 x 1079 mm board, taken a little off square: the far end
// looks narrower than the near end.
const CAMERA = [2.1, 0.15, 300, 0.05, 2.3, 120, 0.00004, 0.00011, 1];
const MAP_POINTS = [
  [235, 219], [112, 374], [363, 374], [237, 349], [235, 514], [52, 677],
  [422, 677], [155, 889], [317, 889], [237, 984],
];

function near(actual, expected, tolerance, message) {
  assert.ok(Math.hypot(actual[0] - expected[0], actual[1] - expected[1]) < tolerance, `${message}: ${actual} vs ${expected}`);
}

test("recovers a camera's transform from points on the map", () => {
  const photo = MAP_POINTS.map((p) => applyHomography(CAMERA, p));
  const fitted = fitHomography(MAP_POINTS, photo);
  for (const p of [[0, 0], [520, 1079], [100, 900]]) {
    near(applyHomography(fitted, p), applyHomography(CAMERA, p), 1e-6, "mapped point");
  }
});

test("four points are enough", () => {
  const square = [[0, 0], [100, 0], [100, 100], [0, 100]];
  const quad = [[10, 20], [200, 0], [220, 210], [0, 180]];
  const fitted = fitHomography(square, quad);
  square.forEach((p, n) => near(applyHomography(fitted, p), quad[n], 1e-6, "corner"));
});

test("averages out small clicking errors", () => {
  // Every click is up to 3 photo pixels out, in a fixed pattern.
  const photo = MAP_POINTS.map((p, n) => {
    const [u, v] = applyHomography(CAMERA, p);
    return [u + 3 * Math.sin(n * 2.1), v + 3 * Math.cos(n * 1.7)];
  });
  const back = invertHomography(fitHomography(MAP_POINTS, photo));
  // About 2.2 photo pixels to the millimetre, so within a couple of mm.
  MAP_POINTS.forEach((p, n) => near(applyHomography(back, photo[n]), p, 2.5, "clicked point"));
});

test("inverting undoes the transform", () => {
  const back = invertHomography(CAMERA);
  for (const p of MAP_POINTS) near(applyHomography(back, applyHomography(CAMERA, p)), p, 1e-6, "round trip");
});

test("refuses too few points, or points in a line", () => {
  assert.throws(() => fitHomography([[0, 0], [1, 0], [0, 1]], [[0, 0], [1, 0], [0, 1]]), /four points/);
  const line = [[0, 0], [10, 0], [20, 0], [30, 0], [40, 0]];
  assert.throws(() => fitHomography(line, line), /straight line/);
});
