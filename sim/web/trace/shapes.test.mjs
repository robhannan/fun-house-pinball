// Tests for the trace page's shape editing. Run with: pnpm run test:site
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import path from "node:path";
import { test } from "node:test";

import { SHAPE_TAG, arcMidpoint, arcThrough, circleHandles, formatPath, parsePath, rewriteSvg } from "./shapes.js";

const MAP = await readFile(path.join(import.meta.dirname, "../../../docs/playfield.svg"), "utf8");

test("reads and writes the map's outline path", () => {
  const d = MAP.match(/<path d="([^"]+)"[^>]*data-wall/)[1];
  const commands = parsePath(d);
  assert.ok(commands, "the outline is a path the page can edit");
  // Unchanged, it's written back exactly as it was.
  assert.equal(formatPath(commands), d);
  assert.ok(commands.some((c) => c.cmd === "A"), "it has the arch");
});

test("leaves paths it can't edit alone", () => {
  assert.equal(parsePath("M 0 0 Q 10 0 10 10"), null);
  assert.equal(parsePath("M 0 0 l 10 10"), null);
  assert.equal(parsePath("M 0 0 A 10 20 0 0 1 10 10"), null);
});

test("a half circle's middle is at its top", () => {
  const [x, y] = arcMidpoint([0, 100], [200, 100], 100, 0, 1);
  assert.ok(Math.abs(x - 100) < 1e-9 && Math.abs(y) < 1e-9, `${x}, ${y}`);
});

test("an arc through its own middle is the same arc", () => {
  for (const [a, b, r, large, sweep] of [
    [[12.7, 219], [507, 274], 250, 0, 1],
    [[0, 0], [100, 0], 80, 0, 0],
    [[0, 0], [100, 0], 60, 1, 1],
  ]) {
    const m = arcMidpoint(a, b, r, large, sweep);
    const arc = arcThrough(a, m, b);
    assert.ok(Math.abs(arc.r - r) < 1e-6, `radius ${arc.r} vs ${r}`);
    assert.deepEqual([arc.large, arc.sweep], [large, sweep]);
  }
  assert.equal(arcThrough([0, 0], [1, 1], [2, 2]), null);
});

test("a circle's centre handle is drawn on top of its radius handle", () => {
  // A 5.4 mm rollover is smaller than a handle, so the two overlap; the one
  // drawn last gets the click.
  const handles = circleHandles(103, 259, 2.7);
  assert.equal(handles.at(-1).kind, "centre");
  assert.deepEqual([handles.at(-1).x, handles.at(-1).y], [103, 259]);
});

test("rewriting changes only the edited attributes", () => {
  const shapes = [...MAP.matchAll(SHAPE_TAG)];
  const index = shapes.findIndex((m) => m[0].includes('data-lamp="2"'));
  const out = rewriteSvg(MAP, new Map([[index, { points: "20,600 90,680 20,760" }]]));
  const changed = out.split("\n").filter((line, n) => line !== MAP.split("\n")[n]);
  assert.equal(changed.length, 1);
  assert.match(changed[0], /points="20,600 90,680 20,760" fill="#ffd33d"/);
  assert.equal(rewriteSvg(MAP, new Map()), MAP);
});
