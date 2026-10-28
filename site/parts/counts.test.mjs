// Tests for the parts tracker's saved data. Run with: pnpm run test:site
import assert from "node:assert/strict";
import { test } from "node:test";

import { commitMessage, itemState, parseCounts, serialiseCounts, withGot } from "./counts.js";

test("reads a file saved before the ordered option", () => {
  assert.deepEqual(parseCounts('{ "mega": 2, "leds": 1 }'), {
    mega: { got: 2, ordered: false },
    leds: { got: 1, ordered: false },
  });
});

test("reads counts and orders, including an order for a part you have none of", () => {
  assert.deepEqual(parseCounts('{ "got": { "mega": 2 }, "ordered": ["mega", "hc595"] }'), {
    mega: { got: 2, ordered: true },
    hc595: { got: 0, ordered: true },
  });
  assert.deepEqual(parseCounts('{ "got": {}, "ordered": [] }'), {});
});

test("refuses a file that isn't an object of counts", () => {
  for (const text of ["null", "[]", "3", '"mega"']) assert.throws(() => parseCounts(text), TypeError, text);
  assert.throws(() => parseCounts("{"), SyntaxError);
});

test("writes sorted counts and orders, dropping zeros, and reads them back", () => {
  const records = { mega: { got: 2, ordered: false }, hc595: { got: 0, ordered: true }, leds: { got: 0, ordered: false } };
  const text = serialiseCounts(records);
  assert.equal(text, '{\n  "got": {\n    "mega": 2\n  },\n  "ordered": [\n    "hc595"\n  ]\n}\n');
  assert.deepEqual(parseCounts(text), { mega: { got: 2, ordered: false }, hc595: { got: 0, ordered: true } });
});

test("getting the full amount takes a part off order; fewer doesn't", () => {
  const ordered = { got: 0, ordered: true };
  assert.deepEqual(withGot(ordered, 12, 20), { got: 12, ordered: true });
  assert.deepEqual(withGot(ordered, 20, 20), { got: 20, ordered: false });
  assert.deepEqual(withGot(ordered, 3, null), { got: 3, ordered: true });
  assert.deepEqual(withGot(ordered, -1, 20), { got: 0, ordered: true });
});

test("a row's state", () => {
  const item = { need: 20 };
  assert.equal(itemState(item, { got: 20, ordered: true }), "done");
  assert.equal(itemState(item, { got: 5, ordered: true }), "ordered");
  assert.equal(itemState(item, { got: 5, ordered: false }), "partial");
  assert.equal(itemState(item, { got: 0, ordered: false }), "none");
  assert.equal(itemState({ need: null }, { got: 1, ordered: false }), "done");
  assert.equal(itemState({ need: null }, { got: 0, ordered: true }), "ordered");
});

test("commit messages name the part and the change", () => {
  const byId = new Map([["hc595", { name: "74HC595 shift register" }]]);
  const before = { hc595: { got: 2, ordered: false } };
  assert.equal(commitMessage({ hc595: { got: 4, ordered: false } }, before, byId), "Parts: 74HC595 shift register, 2 → 4");
  assert.equal(commitMessage({ hc595: { got: 2, ordered: true } }, before, byId), "Parts: 74HC595 shift register ordered");
  assert.equal(commitMessage({ hc595: { got: 2, ordered: false } }, { hc595: { got: 2, ordered: true } }, byId), "Parts: 74HC595 shift register no longer on order");
  assert.equal(commitMessage({ a: { got: 1, ordered: false }, b: { got: 1, ordered: false } }, {}, byId), "Parts: update 2 parts");
});
