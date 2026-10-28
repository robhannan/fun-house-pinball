// The parts tracker. items.json is built from docs/shopping-list.md by
// site/build.mjs; what you've got and what's on order live in got.json on
// the parts-data branch, read and committed through the GitHub contents API.
import { commitMessage, itemState, parseCounts, record, serialiseCounts, withGot } from "./counts.js";

const TOKEN_KEY = "funhouse-parts-token";
const $ = (id) => document.getElementById(id);

const { repo, branch, file, sections } = await (await fetch("items.json", { cache: "no-store" })).json();
const items = sections.flatMap((s) => s.groups.flatMap((g) => g.items));
const byId = new Map(items.map((it) => [it.id, it]));
const api = `https://api.github.com/repos/${repo}/contents/${file}`;

let records = {}; // what the page shows: { id: { got, ordered } }
let saved = {}; // what got.json held when last read or written
let sha = null;
let token = readToken();
const pending = {}; // unsaved changes by id, reapplied after a re-read
let saveTimer = null;
let saving = false;
let filter = "all";
let query = "";

function readToken() {
  try {
    return localStorage.getItem(TOKEN_KEY);
  } catch {
    return null;
  }
}

function writeToken(value) {
  try {
    if (value) localStorage.setItem(TOKEN_KEY, value);
    else localStorage.removeItem(TOKEN_KEY);
  } catch {}
}

function headers() {
  const h = { Accept: "application/vnd.github+json", "X-GitHub-Api-Version": "2022-11-28" };
  if (token) h.Authorization = `Bearer ${token}`;
  return h;
}

const fromBase64 = (b64) => new TextDecoder().decode(Uint8Array.from(atob(b64.replace(/\s/g, "")), (c) => c.charCodeAt(0)));
const toBase64 = (text) => btoa(String.fromCharCode(...new TextEncoder().encode(text)));
const hasPending = () => Object.keys(pending).length > 0;

async function load() {
  const res = await fetch(`${api}?ref=${branch}`, { headers: headers(), cache: "no-store" });
  if (res.status === 401) throw new Error("GitHub didn't accept the token. Check it hasn't expired, or forget it and paste a new one.");
  if (res.status === 404) throw new Error(`Can't find ${file} on the ${branch} branch of ${repo}.`);
  if (res.status === 403) throw new Error("GitHub is limiting requests. Connect a token, or try again in an hour.");
  if (!res.ok) throw new Error(`GitHub answered ${res.status} when reading the counts.`);
  const body = await res.json();
  sha = body.sha;
  saved = parseCounts(fromBase64(body.content));
  records = { ...saved, ...pending };
}

function setStatus(text, warn = false) {
  $("status").textContent = text;
  $("status").classList.toggle("warn", warn);
}

// A save that fails for a reason trying again won't fix, such as a bad token.
class StopSaving extends Error {}

const SAVE_DELAY_MS = 1500;
let retryDelay = SAVE_DELAY_MS;

async function save() {
  if (saving) return schedule();
  if (!hasPending() || !token) return;
  saving = true;
  let stop = false;
  setStatus("Saving…");
  try {
    for (let attempt = 0; ; attempt++) {
      const sent = { ...pending };
      const res = await fetch(api, {
        method: "PUT",
        headers: { ...headers(), "Content-Type": "application/json" },
        body: JSON.stringify({ message: commitMessage(sent, saved, byId), content: toBase64(serialiseCounts({ ...saved, ...sent })), sha, branch }),
      });
      if ((res.status === 409 || res.status === 422) && attempt === 0) {
        // Saved from somewhere else since we last read: re-read, keep our changes, try again.
        await load();
        render();
        continue;
      }
      if (res.status === 401) throw new StopSaving("GitHub didn't accept the token. Forget it and paste a new one.");
      if (res.status === 403 || res.status === 404) throw new StopSaving("The token can't write to the repository. It needs Contents: Read and write.");
      if (!res.ok) throw new Error(`GitHub answered ${res.status} when saving. Trying again shortly.`);
      sha = (await res.json()).content.sha;
      Object.assign(saved, sent);
      // Changes made while this save was in flight stay pending for the next one.
      for (const [id, rec] of Object.entries(sent)) if (pending[id] === rec) delete pending[id];
      break;
    }
    retryDelay = SAVE_DELAY_MS;
    setStatus(hasPending() ? "Saving…" : "Saved");
  } catch (e) {
    stop = e instanceof StopSaving;
    retryDelay = Math.min(retryDelay * 2, 60_000);
    setStatus(e instanceof TypeError ? "Couldn't reach GitHub. Trying again shortly." : e.message, true);
  } finally {
    saving = false;
  }
  // After an error that won't fix itself, wait for the next change or a new token.
  if (hasPending() && !stop) schedule(retryDelay);
}

function schedule(delay = SAVE_DELAY_MS) {
  clearTimeout(saveTimer);
  saveTimer = setTimeout(save, delay);
}

function change(id, rec) {
  if (!token) return;
  records[id] = rec;
  pending[id] = rec;
  setStatus("Unsaved changes");
  render();
  schedule();
}

const setCount = (id, n) => change(id, withGot(record(records, id), n, byId.get(id).need));
const toggleOrdered = (id) => change(id, { ...record(records, id), ordered: !record(records, id).ordered });

const tracked = (it) => it.need != null && it.need > 0 && !it.optional;

function el(tag, props = {}, ...children) {
  const node = Object.assign(document.createElement(tag), props);
  node.append(...children);
  return node;
}

function rowEl(it) {
  const name = el("div", { className: "name", textContent: it.name });
  if (it.distributor) name.append(el("span", { className: "tag", textContent: "Distributor", title: "Buy from a reputable distributor" }));
  if (it.optional) name.append(el("span", { className: "tag opt", textContent: "Optional" }));

  const note = el("div", { className: "note" });
  const parts = [];
  for (const link of it.links) parts.push(el("a", { href: link.url, textContent: link.label, target: "_blank", rel: "noopener" }));
  if (it.where) parts.push(it.where);
  if (it.note) parts.push(it.note);
  parts.forEach((part, i) => note.append(...(i ? [" · ", part] : [part])));

  const got = () => record(records, it.id).got;
  const input = el("input", { type: "number", min: "0", inputMode: "numeric", id: `got-${it.id}` });
  input.setAttribute("aria-label", `How many ${it.name} you've got`);
  input.onchange = () => setCount(it.id, parseInt(input.value, 10));
  const minus = el("button", { type: "button", textContent: "−", onclick: () => setCount(it.id, got() - 1) });
  minus.setAttribute("aria-label", `One fewer ${it.name}`);
  const plus = el("button", { type: "button", textContent: "+", onclick: () => setCount(it.id, got() + 1) });
  plus.setAttribute("aria-label", `One more ${it.name}`);
  const order = el("button", { type: "button", className: "order", textContent: "Ordered", onclick: () => toggleOrdered(it.id) });
  order.title = "Mark the rest as ordered and on its way";

  const row = el(
    "div",
    { className: "row" },
    el("div", { className: "info" }, name, note),
    el("div", { className: "counter" }, minus, input, plus, el("span", { className: "need", textContent: `/ ${it.qty}` }), order, el("span", { className: "pill" })),
  );
  row.dataset.id = it.id;
  return row;
}

function build() {
  const list = $("list");
  for (const s of sections) {
    const sec = el("section", { className: "section" }, el("div", { className: "section-head" }, el("h2", { textContent: s.title }), el("span", { className: "count" })));
    sec.dataset.section = s.key;
    for (const g of s.groups) {
      const grp = el("div", { className: "grp" });
      if (g.title) grp.append(el("h3", { textContent: g.title }));
      grp.append(...g.items.map(rowEl));
      sec.append(grp);
    }
    list.append(sec);
  }
  list.append(el("p", { className: "empty", id: "empty", hidden: true, textContent: "Nothing matches." }));
}

const PILL = {
  done: (it) => (it.need == null ? "Got some" : "Got"),
  ordered: () => "On order",
  partial: (it, rec) => `${it.need - rec.got} to go`,
  none: () => "To get",
};

const FILTERS = {
  all: () => true,
  order: (st) => st === "partial" || st === "none",
  ordered: (st) => st === "ordered",
  got: (st, rec) => rec.got > 0,
};

function render() {
  const totals = { done: 0, ordered: 0, toOrder: 0 };
  let shown = 0;
  for (const it of items) {
    const rec = record(records, it.id);
    const st = itemState(it, rec);
    const row = document.querySelector(`.row[data-id="${it.id}"]`);
    const input = row.querySelector("input");
    if (document.activeElement !== input) input.value = rec.got;
    row.querySelectorAll("input, button").forEach((c) => (c.disabled = !token));
    for (const s of ["done", "ordered", "partial"]) row.classList.toggle(s, st === s);
    const order = row.querySelector(".order");
    order.setAttribute("aria-pressed", String(rec.ordered));
    order.setAttribute("aria-label", `${it.name} ordered`);
    // Nothing left to order once you have the full amount.
    order.hidden = st === "done" && it.need != null;
    const pill = row.querySelector(".pill");
    pill.className = `pill ${st}`;
    pill.textContent = PILL[st](it, rec);
    if (tracked(it)) {
      if (st === "done") totals.done++;
      else if (st === "ordered") totals.ordered++;
      else totals.toOrder++;
    }
    const text = `${it.name} ${it.note} ${it.where} ${it.links.map((l) => l.label).join(" ")}`.toLowerCase();
    const matches = FILTERS[filter](st, rec) && (!query || text.includes(query));
    row.hidden = !matches;
    if (matches) shown++;
  }
  for (const s of sections) {
    const t = s.groups.flatMap((g) => g.items).filter(tracked);
    const sec = document.querySelector(`[data-section="${s.key}"]`);
    sec.querySelector(".count").textContent = t.length ? `${t.filter((it) => itemState(it, record(records, it.id)) === "done").length} of ${t.length} complete` : "";
    sec.querySelectorAll(".grp").forEach((g) => (g.hidden = !g.querySelector(".row:not([hidden])")));
    sec.hidden = !sec.querySelector(".row:not([hidden])");
  }
  const total = totals.done + totals.ordered + totals.toOrder;
  $("empty").hidden = shown > 0;
  $("s-done").textContent = `${totals.done} / ${total}`;
  $("s-ordered").textContent = totals.ordered;
  $("s-to-order").textContent = totals.toOrder;
  $("s-bar").style.width = `${total ? (100 * totals.done) / total : 0}%`;

  // Saved parts that were renamed or removed from the shopping list.
  const orphans = Object.entries(records).filter(([id, rec]) => (rec.got > 0 || rec.ordered) && !byId.has(id));
  $("orphans").hidden = !orphans.length;
  $("orphans").textContent = `Saved counts for parts no longer on the list: ${orphans.map(([id, rec]) => `${id} (${rec.got}${rec.ordered ? ", ordered" : ""})`).join(", ")}. Edit ${file} on the ${branch} branch to move or remove them.`;
}

function showConnection() {
  $("connect-summary").textContent = token ? "Connected to GitHub" : "Connect to GitHub to save counts";
  $("forget").hidden = !token;
  $("connect").open = false;
}

async function refresh() {
  try {
    await load();
    render();
    setStatus(token ? (hasPending() ? "Unsaved changes" : "Up to date") : "Read only. Connect to GitHub to change counts.", !token);
  } catch (e) {
    setStatus(e.message, true);
  }
}

$("list-link").href = `https://github.com/${repo}/blob/main/docs/shopping-list.md`;
$("data-path").textContent = `${file} on the ${branch} branch`;
$("repo-name").textContent = repo;
$("token-form").onsubmit = (e) => {
  e.preventDefault();
  const value = $("token").value.trim();
  if (!value) return;
  token = value;
  writeToken(token);
  $("token").value = "";
  showConnection();
  // Save anything changed before the old token was forgotten.
  refresh().then(() => hasPending() && schedule());
};
$("forget").onclick = () => {
  clearTimeout(saveTimer);
  token = null;
  writeToken(null);
  showConnection();
  render();
  setStatus("Read only. Connect to GitHub to change counts.", true);
};
document.querySelectorAll(".filters button").forEach(
  (b) =>
    (b.onclick = () => {
      filter = b.dataset.filter;
      document.querySelectorAll(".filters button").forEach((x) => x.setAttribute("aria-pressed", String(x === b)));
      render();
    }),
);
$("search").oninput = (e) => {
  query = e.target.value.trim().toLowerCase();
  render();
};
// Pick up counts saved from another device when coming back to the tab.
document.addEventListener("visibilitychange", () => {
  if (document.visibilityState === "visible" && !hasPending() && !saving) refresh();
});

build();
showConnection();
render();
await refresh();
