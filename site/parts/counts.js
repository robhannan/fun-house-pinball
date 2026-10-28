// What got.json holds and how a part's row reads it, kept free of the page
// so site/parts/counts.test.mjs can test it. Each part has a record:
// { got: how many you have, ordered: whether the rest is on order }.

const EMPTY = Object.freeze({ got: 0, ordered: false });

export const record = (records, id) => records[id] ?? EMPTY;

// got.json is { "got": { id: count }, "ordered": [id, ...] }. Files saved
// before the ordered option are a flat { id: count }, read the same way.
export function parseCounts(text) {
  const data = JSON.parse(text);
  if (data === null || typeof data !== "object" || Array.isArray(data)) throw new TypeError("not an object of counts");
  const legacy = !("got" in data || "ordered" in data);
  const got = legacy ? data : (data.got ?? {});
  const records = {};
  for (const [id, n] of Object.entries(got)) records[id] = { got: Number(n) || 0, ordered: false };
  for (const id of legacy ? [] : (data.ordered ?? [])) records[id] = { ...record(records, id), ordered: true };
  return records;
}

export function serialiseCounts(records) {
  const ids = Object.keys(records).sort((a, b) => a.localeCompare(b));
  const got = Object.fromEntries(ids.filter((id) => records[id].got > 0).map((id) => [id, records[id].got]));
  const ordered = ids.filter((id) => records[id].ordered);
  return JSON.stringify({ got, ordered }, null, 2) + "\n";
}

// A new count, clamped. Having the full amount means nothing is on order.
export function withGot(rec, n, need) {
  const got = Math.max(0, Math.min(9999, Number.isFinite(n) ? n : 0));
  return { got, ordered: rec.ordered && !(need != null && got >= need) };
}

// "done", "ordered", "partial" or "none".
export function itemState(item, rec) {
  if (item.need == null ? rec.got > 0 : rec.got >= item.need) return "done";
  if (rec.ordered) return "ordered";
  return rec.got > 0 ? "partial" : "none";
}

export function commitMessage(changes, before, byId) {
  const ids = Object.keys(changes);
  if (ids.length !== 1) return `Parts: update ${ids.length} parts`;
  const [id] = ids;
  const name = byId.get(id)?.name ?? id;
  const was = record(before, id);
  const now = changes[id];
  if (was.got !== now.got) return `Parts: ${name}, ${was.got} → ${now.got}`;
  return now.ordered ? `Parts: ${name} ordered` : `Parts: ${name} no longer on order`;
}
