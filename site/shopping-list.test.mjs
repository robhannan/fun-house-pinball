// Tests for the shopping list parser. Run with: pnpm run test:site
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import path from "node:path";
import { test } from "node:test";

import { parseQty, parseShoppingList } from "./shopping-list.mjs";

const items = (sections) => sections.flatMap((s) => s.groups.flatMap((g) => g.items));

test("reads each kind of quantity the list uses", () => {
  assert.deepEqual(parseQty("2"), { need: 2, qty: "2" });
  assert.deepEqual(parseQty("1 pack each"), { need: 1, qty: "1 pack each" });
  assert.deepEqual(parseQty("3–5"), { need: 5, qty: "3–5" });
  assert.deepEqual(parseQty("about 25 parts"), { need: 25, qty: "about 25 parts" });
  assert.deepEqual(parseQty("As needed"), { need: null, qty: "As needed" });
  assert.equal(parseQty("plenty"), undefined);
});

const SAMPLE = `# Shopping list

## Estimated cost

| | Estimate |
|---|---|
| **Everything** | **£1** |

## 1. Power

| Item | Qty | Notes | Link | Est. £ |
|---|---|---|---|---|
| 🛡️ Fuse holder | 4 | E.g. Bulgin FX0454 (10 A). **Have an electrician fit it.** | [Rapid](https://example.com) | 4 |
| 5 V adaptor | 1 | For sound | [RS](https://example.com) | 16 |
| 1 kΩ resistor | — | In the resistor kit | — | — |

## 2. Servos

| Item | Qty | Notes | Link | Est. £ |
|---|---|---|---|---|
| 5 V adaptor | 1 | For servos | [RS](https://example.com) | 16 |

### Later

| Item | Qty | Notes | Est. £ |
|---|---|---|---|
| *Optional:* Bells | 0–3 | Later | varies |
`;

test("keeps whole notes, links and flags, and skips parts from another kit", () => {
  const { sections, errors } = parseShoppingList(SAMPLE);
  assert.deepEqual(errors, []);
  assert.deepEqual(
    sections.map((s) => s.title),
    ["1. Power", "2. Servos"],
  );
  const [fuse, , servo, bells] = items(sections);
  assert.equal(fuse.note, "E.g. Bulgin FX0454 (10 A). Have an electrician fit it.");
  assert.equal(fuse.distributor, true);
  assert.deepEqual(fuse.links, [{ label: "Rapid", url: "https://example.com" }]);
  assert.equal(items(sections).length, 4);
  assert.equal(servo.note, "For servos");
  assert.equal(bells.optional, true);
  assert.equal(bells.name, "Bells");
  assert.equal(bells.need, 3);
  assert.equal(sections[1].groups[1].title, "Later");
});

test("a name in two sections gets its section on both copies", () => {
  const ids = items(parseShoppingList(SAMPLE).sections).map((it) => it.id);
  assert.ok(ids.includes("5-v-adaptor-1"));
  assert.ok(ids.includes("5-v-adaptor-2"));
  assert.ok(!ids.includes("5-v-adaptor"));
});

test("reports quantities it can't read and repeats within a section", () => {
  const { errors } = parseShoppingList(`## 1. Things

| Item | Qty |
|---|---|
| Widget | lots |
| Gadget | 1 |
| Gadget | 2 |
`);
  assert.equal(errors.length, 2);
  assert.match(errors[0], /line 5: can't read the quantity "lots"/);
  assert.match(errors[1], /"Gadget" appears twice in section 1/);
});

test("the real shopping list parses, with unique ids", async () => {
  const markdown = await readFile(path.join(import.meta.dirname, "../docs/shopping-list.md"), "utf8");
  const { sections, errors } = parseShoppingList(markdown);
  assert.deepEqual(errors, []);
  const ids = items(sections).map((it) => it.id);
  assert.ok(ids.length > 50);
  assert.equal(new Set(ids).size, ids.length);
});
