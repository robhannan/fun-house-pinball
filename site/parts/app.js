// The parts tracker. items.json is built from docs/shopping-list.md by
// site/build.mjs; the counts live in got.json on the parts-data branch,
// read and committed through the GitHub contents API.

const TOKEN_KEY = "funhouse-parts-token";
const $ = (id) => document.getElementById(id);

const { repo, branch, file, sections } = await (await fetch("items.json", { cache: "no-store" })).json();
const items = sections.flatMap((s) => s.groups.flatMap((g) => g.items));
const byId = new Map(items.map((it) => [it.id, it]));
const api = `https://api.github.com/repos/${repo}/contents/${file}`;

let counts = {}; // what the page shows
let saved = {}; // what got.json held when last read or written
let sha = null;
let token = readToken();
const dirty = new Set();
const pendingValues = {}; // unsaved changes, reapplied after a re-read
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

async function load() {
  const res = await fetch(`${api}?ref=${branch}`, { headers: headers(), cache: "no-store" });
  if (res.status === 401) throw new Error("GitHub didn't accept the token. Check it hasn't expired, or forget it and paste a new one.");
  if (res.status === 404) throw new Error(`Can't find ${file} on the ${branch} branch of ${repo}.`);
  if (res.status === 403) throw new Error("GitHub is limiting requests. Connect a token, or try again in an hour.");
  if (!res.ok) throw new Error(`GitHub answered ${res.status} when reading the counts.`);
  const body = await res.json();
  sha = body.sha;
  saved = JSON.parse(fromBase64(body.content));
  counts = { ...saved };
  for (const id of dirty) counts[id] = pendingValues[id];
}

function setStatus(text, warn = false) {
  $("status").textContent = text;
  $("status").classList.toggle("warn", warn);
}

function commitMessage(ids) {
  if (ids.length === 1) {
    const [id] = ids;
    return `Parts: ${byId.get(id)?.name ?? id}, ${saved[id] ?? 0} → ${counts[id] ?? 0}`;
  }
  return `Parts: update ${ids.length} counts`;
}

function serialise(values) {
  const tidy = Object.fromEntries(
    Object.entries(values)
      .filter(([, n]) => n > 0)
      .sort(([a], [b]) => a.localeCompare(b)),
  );
  return JSON.stringify(tidy, null, 2) + "\n";
}

async function save(retried = false) {
  if (saving) return schedule();
  if (!dirty.size) return;
  saving = true;
  const ids = [...dirty];
  const sent = Object.fromEntries(ids.map((id) => [id, counts[id]]));
  setStatus("Saving…");
  try {
    const res = await fetch(api, {
      method: "PUT",
      headers: { ...headers(), "Content-Type": "application/json" },
      body: JSON.stringify({ message: commitMessage(ids), content: toBase64(serialise({ ...saved, ...sent })), sha, branch }),
    });
    if ((res.status === 409 || res.status === 422) && !retried) {
      // Saved from somewhere else since we last read: re-read, keep our changes, try again.
      saving = false;
      await load();
      render();
      return save(true);
    }
    if (res.status === 401) throw new Error("GitHub didn't accept the token. Forget it and paste a new one.");
    if (res.status === 403 || res.status === 404) throw new Error("The token can't write to the repository. It needs Contents: Read and write.");
    if (!res.ok) throw new Error(`GitHub answered ${res.status} when saving.`);
    sha = (await res.json()).content.sha;
    Object.assign(saved, sent);
    for (const id of ids) {
      if (counts[id] === sent[id]) {
        dirty.delete(id);
        delete pendingValues[id];
      }
    }
    setStatus(dirty.size ? "Saving…" : "Saved");
  } catch (e) {
    setStatus(e.message, true);
  } finally {
    saving = false;
  }
  if (dirty.size) schedule();
}

function schedule() {
  clearTimeout(saveTimer);
  saveTimer = setTimeout(save, 1500);
}

function setCount(id, n) {
  if (!token) return;
  counts[id] = Math.max(0, Math.min(9999, Number.isFinite(n) ? n : 0));
  pendingValues[id] = counts[id];
  dirty.add(id);
  setStatus("Unsaved changes");
  render();
  schedule();
}

function state(it) {
  const got = counts[it.id] ?? 0;
  if (it.need == null) return got > 0 ? "done" : "none";
  if (got >= it.need) return "done";
  return got > 0 ? "partial" : "none";
}

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

  const input = el("input", { type: "number", min: "0", inputMode: "numeric", id: `got-${it.id}` });
  input.setAttribute("aria-label", `How many ${it.name} you've got`);
  input.onchange = () => setCount(it.id, parseInt(input.value, 10));
  const minus = el("button", { type: "button", textContent: "−", onclick: () => setCount(it.id, (counts[it.id] ?? 0) - 1) });
  minus.setAttribute("aria-label", `One fewer ${it.name}`);
  const plus = el("button", { type: "button", textContent: "+", onclick: () => setCount(it.id, (counts[it.id] ?? 0) + 1) });
  plus.setAttribute("aria-label", `One more ${it.name}`);

  const row = el(
    "div",
    { className: "row" },
    el("div", { className: "info" }, name, note),
    el("div", { className: "counter" }, minus, input, plus, el("span", { className: "need", textContent: `/ ${it.qty}` }), el("span", { className: "pill" })),
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

function render() {
  let done = 0;
  let partial = 0;
  let none = 0;
  let shown = 0;
  for (const it of items) {
    const st = state(it);
    const got = counts[it.id] ?? 0;
    const row = document.querySelector(`.row[data-id="${it.id}"]`);
    const input = row.querySelector("input");
    if (document.activeElement !== input) input.value = got;
    row.querySelectorAll("input, button").forEach((c) => (c.disabled = !token));
    row.classList.toggle("done", st === "done");
    row.classList.toggle("partial", st === "partial");
    const pill = row.querySelector(".pill");
    pill.className = `pill ${st}`;
    pill.textContent = st === "done" ? (it.need == null ? "Got some" : "Got") : st === "partial" ? `${it.need - got} to go` : "To get";
    if (tracked(it)) {
      if (st === "done") done++;
      else if (st === "partial") partial++;
      else none++;
    }
    const text = `${it.name} ${it.note} ${it.where} ${it.links.map((l) => l.label).join(" ")}`.toLowerCase();
    const matches = (filter === "all" || (filter === "got" ? got > 0 : st !== "done")) && (!query || text.includes(query));
    row.hidden = !matches;
    if (matches) shown++;
  }
  for (const s of sections) {
    const t = s.groups.flatMap((g) => g.items).filter(tracked);
    const sec = document.querySelector(`[data-section="${s.key}"]`);
    sec.querySelector(".count").textContent = t.length ? `${t.filter((it) => state(it) === "done").length} of ${t.length} complete` : "";
    sec.querySelectorAll(".grp").forEach((g) => (g.hidden = !g.querySelector(".row:not([hidden])")));
    sec.hidden = !sec.querySelector(".row:not([hidden])");
  }
  const total = done + partial + none;
  $("empty").hidden = shown > 0;
  $("s-done").textContent = `${done} / ${total}`;
  $("s-partial").textContent = partial;
  $("s-none").textContent = none;
  $("s-bar").style.width = `${total ? (100 * done) / total : 0}%`;

  // Counts for parts that were renamed or removed from the shopping list.
  const orphans = Object.entries(counts).filter(([id, n]) => n > 0 && !byId.has(id));
  $("orphans").hidden = !orphans.length;
  $("orphans").textContent = `Saved counts for parts no longer on the list: ${orphans.map(([id, n]) => `${id} (${n})`).join(", ")}. Edit ${file} on the ${branch} branch to move or remove them.`;
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
    setStatus(token ? (dirty.size ? "Unsaved changes" : "Up to date") : "Read only. Connect to GitHub to change counts.", !token);
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
  refresh();
};
$("forget").onclick = () => {
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
  if (document.visibilityState === "visible" && !dirty.size && !saving) refresh();
});

build();
showConnection();
render();
await refresh();
