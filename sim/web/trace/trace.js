// The playfield tracer: lines a photo of the real playfield up with the map
// (docs/playfield.svg, copied into ../generated/ by `pnpm run build:sim`),
// then lets you drag the map's assumed shapes onto the photo and download the
// corrected map.
//
// 1. You click the centres of the parts marked data-measured on the photo.
// 2. A homography fitted to those clicks straightens the photo onto the map,
//    in millimetres, and each click's error shows how good the fit is.
// 3. You drag the other shapes' corners; the download changes only the
//    attributes you edited, so the rest of the file stays as it was.
//
// The photo never leaves the browser. Clicks and edits are kept in
// localStorage, so a reload doesn't lose them.

import { applyHomography, fitHomography, invertHomography } from "./homography.js";
import { SHAPE_TAG, arcMidpoint, arcThrough, formatNumber, formatPath, parsePath, rewriteSvg } from "./shapes.js";

const MAP_FILE = "../generated/playfield.svg";
const PX_PER_MM = 2; // the straightened photo's resolution
const MAX_PHOTO_SIDE = 4000; // bigger photos are scaled down to straighten them
const HANDLE_MM = 3.5;
const FAR_MM = 5; // a click this far off is probably a misclick
const SVG_NS = "http://www.w3.org/2000/svg";
const ATTRIBUTES = {
  polygon: ["points"],
  line: ["x1", "y1", "x2", "y2"],
  circle: ["cx", "cy", "r"],
  rect: ["x", "y", "width", "height"],
  path: ["d"],
};

const $ = (id) => document.getElementById(id);

let source; // the map's text
let svg; // the map drawn over the straightened photo
let viewBox; // [x, y, width, height]
let points = []; // the measured parts: { label, map: [x, y] }
let shapes = []; // { el, index, editable, original }
let photo = null; // { img, key }
let clicks = new Map(); // label -> [x, y] in photo pixels
let skipped = new Set();
let current = 0; // the point being clicked
let fitted = null; // the homography from the map to the photo
const edits = new Map(); // shape index -> attribute values
let selected = null; // { shape, model }
let dragging = null;
let zoom = 1;

const store = {
  get(key) {
    try {
      return JSON.parse(localStorage.getItem(key) ?? "null");
    } catch {
      return null;
    }
  },
  set(key, value) {
    try {
      localStorage.setItem(key, JSON.stringify(value));
    } catch {
      // Private windows and blocked storage just don't remember.
    }
  },
};

// A short fingerprint of the map, so saved edits are only reapplied to the
// same map.
function fingerprint(text) {
  let h = 5381;
  for (let i = 0; i < text.length; i++) h = (h * 33 + text.charCodeAt(i)) | 0;
  return `fun-house-trace:map:${h}`;
}

async function load() {
  source = await (await fetch(MAP_FILE, { cache: "no-store" })).text();
  $("overlay").innerHTML = source;
  svg = $("overlay").querySelector("svg");
  svg.removeAttribute("width");
  svg.removeAttribute("height");
  viewBox = svg.getAttribute("viewBox").split(/\s+/).map(Number);
  svg.querySelector("rect").classList.add("background");

  const elements = [...svg.querySelectorAll("polygon, line, circle, rect, path")];
  if (elements.length !== [...source.matchAll(SHAPE_TAG)].length) {
    throw new Error("the map's shapes couldn't be matched to its text");
  }
  shapes = elements.map((el, index) => {
    const editable = isEditable(el);
    el.classList.toggle("editable", editable);
    el.classList.toggle("measured", el.hasAttribute("data-measured"));
    return { el, index, editable, original: snapshot(el) };
  });
  points = [...svg.querySelectorAll("[data-measured]")].map((el) => ({ label: el.dataset.measured, map: centreOf(el) }));

  // A small copy of the map shows which part to click next.
  $("guide").innerHTML = source;
  const guide = $("guide").querySelector("svg");
  guide.removeAttribute("width");
  guide.removeAttribute("height");
  guide.querySelector("rect").classList.add("background");
  const ring = document.createElementNS(SVG_NS, "circle");
  ring.id = "guide-ring";
  ring.setAttribute("r", "22");
  guide.appendChild(ring);

  const handles = document.createElementNS(SVG_NS, "g");
  handles.id = "handles";
  svg.appendChild(handles);

  for (const [index, attributes] of store.get(fingerprint(source)) ?? []) {
    const shape = shapes[index];
    if (!shape?.editable) continue;
    setAttributes(shape.el, attributes);
    edits.set(index, attributes);
  }
  showChanges();
  status("Choose a photo to start.");
}

// Only the parts the simulator uses can be edited: walls, the gate, switches,
// coils and lamps. The measured parts and the flippers (drawn from
// measurements) stay put, and so do decorations such as the apron's fill,
// which would otherwise sit on top of the walls they follow.
const PART = ["data-wall", "data-gate", "data-switch", "data-coil", "data-lamp"];
function isEditable(el) {
  if (el.closest("[transform]") || !PART.some((a) => el.hasAttribute(a))) return false;
  if (["data-measured", "data-flipper"].some((a) => el.hasAttribute(a))) return false;
  return el.tagName !== "path" || parsePath(el.getAttribute("d")) !== null;
}

function centreOf(el) {
  const n = (name) => Number(el.getAttribute(name));
  return el.tagName === "rect" ? [n("x") + n("width") / 2, n("y") + n("height") / 2] : [n("cx"), n("cy")];
}

function snapshot(el) {
  return Object.fromEntries(ATTRIBUTES[el.tagName].map((name) => [name, el.getAttribute(name)]));
}

function setAttributes(el, attributes) {
  for (const [name, value] of Object.entries(attributes)) el.setAttribute(name, value);
}

function status(text) {
  $("status").textContent = text;
}

// ---------------------------------------------------------------------------
// 1. The photo

$("photo-file").addEventListener("change", (event) => {
  const file = event.target.files[0];
  if (!file) return;
  const img = $("photo");
  img.onload = () => {
    photo = { img, key: `fun-house-trace:photo:${file.name}:${file.size}` };
    const saved = store.get(photo.key);
    clicks = new Map(saved?.clicks ?? []);
    skipped = new Set(saved?.skipped ?? []);
    current = nextPoint(0);
    showStep("points");
    zoomTo(1);
    showPoints();
    status(`${img.naturalWidth} × ${img.naturalHeight} px`);
  };
  img.src = URL.createObjectURL(file);
});

// ---------------------------------------------------------------------------
// 2. Lining it up

function nextPoint(from) {
  for (let n = 0; n < points.length; n++) {
    const i = (from + n) % points.length;
    if (!clicks.has(points[i].label) && !skipped.has(points[i].label)) return i;
  }
  return points.length;
}

function saveClicks() {
  store.set(photo.key, { clicks: [...clicks], skipped: [...skipped] });
}

$("photo").addEventListener("click", (event) => {
  if (current >= points.length) return;
  const img = event.currentTarget;
  const scale = img.naturalWidth / img.clientWidth;
  const { label } = points[current];
  clicks.set(label, [event.offsetX * scale, event.offsetY * scale]);
  skipped.delete(label);
  saveClicks();
  current = nextPoint(current + 1);
  showPoints();
});

$("skip").addEventListener("click", () => {
  if (current >= points.length) return;
  skipped.add(points[current].label);
  clicks.delete(points[current].label);
  saveClicks();
  current = nextPoint(current + 1);
  showPoints();
});

$("clear").addEventListener("click", () => {
  if (current >= points.length) return;
  clicks.delete(points[current].label);
  skipped.delete(points[current].label);
  saveClicks();
  showPoints();
});

function showPoints() {
  const point = points[current];
  $("prompt").textContent = point
    ? `Click: ${point.label} (${current + 1} of ${points.length})`
    : `Done: ${clicks.size} points clicked. Line the photo up.`;
  const ring = $("guide-ring");
  ring.style.display = point ? "" : "none";
  if (point) {
    ring.setAttribute("cx", point.map[0]);
    ring.setAttribute("cy", point.map[1]);
  }

  const errors = fitted?.errors ?? new Map();
  const list = $("points");
  list.replaceChildren(
    ...points.map((p, i) => {
      const li = document.createElement("li");
      const error = errors.get(p.label);
      const state = clicks.has(p.label)
        ? error === undefined
          ? "✓"
          : `${error.toFixed(1)} mm`
        : skipped.has(p.label)
          ? "skipped"
          : "";
      li.innerHTML = `<span></span><span class="state"></span>`;
      li.firstChild.textContent = `${i + 1}. ${p.label}`;
      li.lastChild.textContent = state;
      li.classList.toggle("current", i === current);
      li.classList.toggle("done", clicks.has(p.label));
      li.classList.toggle("far", error > FAR_MM);
      li.addEventListener("click", () => {
        current = i;
        showStep("points");
        showPoints();
      });
      return li;
    }),
  );

  const stage = $("photo-stage");
  stage.querySelectorAll(".marker").forEach((m) => m.remove());
  if (!photo) return;
  points.forEach((p, i) => {
    const at = clicks.get(p.label);
    if (!at) return;
    const marker = document.createElement("div");
    marker.className = "marker";
    marker.style.left = `${(at[0] / photo.img.naturalWidth) * 100}%`;
    marker.style.top = `${(at[1] / photo.img.naturalHeight) * 100}%`;
    marker.innerHTML = `<span>${i + 1}</span>`;
    stage.appendChild(marker);
  });
  $("fit").disabled = clicks.size < 4;
}

$("fit").addEventListener("click", () => {
  const used = points.filter((p) => clicks.has(p.label));
  let h;
  try {
    h = fitHomography(
      used.map((p) => p.map),
      used.map((p) => clicks.get(p.label)),
    );
  } catch (error) {
    $("fit-result").textContent = `Couldn't line it up: ${error.message}.`;
    return;
  }
  // How far each click lands from where the map says it should, in mm.
  const back = invertHomography(h);
  const errors = new Map(
    used.map((p) => {
      const [x, y] = applyHomography(back, clicks.get(p.label));
      return [p.label, Math.hypot(x - p.map[0], y - p.map[1])];
    }),
  );
  fitted = { h, errors };
  const values = [...errors.values()];
  const mean = values.reduce((s, v) => s + v, 0) / values.length;
  const [worst] = [...errors].sort((a, b) => b[1] - a[1]);
  $("fit-result").textContent =
    `Average error ${mean.toFixed(1)} mm; worst ${worst[1].toFixed(1)} mm (${worst[0]}). ` +
    (worst[1] > FAR_MM
      ? "Check the red ones: redo a misclick, or skip a part that's moved. Errors growing towards the edges mean lens distortion: take the photo from further away with the 2× lens."
      : "Good enough to trace from.");
  showPoints();
  straighten();
  showStep("trace");
  zoomTo(1);
});

// Draws the photo as seen square-on, at PX_PER_MM, covering the map's viewBox.
function straighten() {
  const [vx, vy, vw, vh] = viewBox;
  const img = photo.img;
  const k = Math.min(1, MAX_PHOTO_SIDE / Math.max(img.naturalWidth, img.naturalHeight));
  const sw = Math.round(img.naturalWidth * k);
  const sh = Math.round(img.naturalHeight * k);
  const scratch = document.createElement("canvas");
  scratch.width = sw;
  scratch.height = sh;
  const sctx = scratch.getContext("2d", { willReadFrequently: true });
  sctx.drawImage(img, 0, 0, sw, sh);
  const src = sctx.getImageData(0, 0, sw, sh).data;

  const canvas = $("straightened");
  canvas.width = Math.round(vw * PX_PER_MM);
  canvas.height = Math.round(vh * PX_PER_MM);
  const ctx = canvas.getContext("2d");
  const out = ctx.createImageData(canvas.width, canvas.height);
  const h = fitted.h;
  for (let v = 0; v < canvas.height; v++) {
    const y = vy + (v + 0.5) / PX_PER_MM;
    for (let u = 0; u < canvas.width; u++) {
      const x = vx + (u + 0.5) / PX_PER_MM;
      const w = h[6] * x + h[7] * y + h[8];
      // Photo pixel centres are at +0.5.
      const px = ((h[0] * x + h[1] * y + h[2]) / w) * k - 0.5;
      const py = ((h[3] * x + h[4] * y + h[5]) / w) * k - 0.5;
      const x0 = Math.floor(px);
      const y0 = Math.floor(py);
      if (x0 < 0 || y0 < 0 || x0 + 1 >= sw || y0 + 1 >= sh) continue;
      const fx = px - x0;
      const fy = py - y0;
      const o = (v * canvas.width + u) * 4;
      const i00 = (y0 * sw + x0) * 4;
      const i10 = i00 + 4;
      const i01 = i00 + sw * 4;
      const i11 = i01 + 4;
      for (let c = 0; c < 3; c++) {
        const top = src[i00 + c] * (1 - fx) + src[i10 + c] * fx;
        const bottom = src[i01 + c] * (1 - fx) + src[i11 + c] * fx;
        out.data[o + c] = top * (1 - fy) + bottom * fy;
      }
      out.data[o + 3] = 255;
    }
  }
  ctx.putImageData(out, 0, 0);
}

// ---------------------------------------------------------------------------
// 3. Tracing

// How a selected shape's handles move it. Each handle has x, y and set(x, y),
// and may have remove() (double-click); insert(x, y) adds a corner.
function modelFor(el) {
  const n = (name) => Number(el.getAttribute(name));
  switch (el.tagName) {
    case "polygon": {
      const pts = el.getAttribute("points").trim().split(/\s+/).map((p) => p.split(",").map(Number));
      return {
        handles: () =>
          pts.map((p, i) => ({
            x: p[0],
            y: p[1],
            set: (x, y) => (pts[i] = [x, y]),
            remove: () => pts.length > 3 && pts.splice(i, 1),
          })),
        insert: (x, y) => pts.splice(nearestEdge(pts, true, [x, y]) + 1, 0, [x, y]),
        attributes: () => ({ points: pts.map((p) => p.map(formatNumber).join(",")).join(" ") }),
      };
    }
    case "line": {
      const ends = [
        [n("x1"), n("y1")],
        [n("x2"), n("y2")],
      ];
      return {
        handles: () => ends.map((p, i) => ({ x: p[0], y: p[1], set: (x, y) => (ends[i] = [x, y]) })),
        attributes: () => ({
          x1: formatNumber(ends[0][0]),
          y1: formatNumber(ends[0][1]),
          x2: formatNumber(ends[1][0]),
          y2: formatNumber(ends[1][1]),
        }),
      };
    }
    case "circle": {
      let [cx, cy, r] = [n("cx"), n("cy"), n("r")];
      return {
        handles: () => [
          { x: cx, y: cy, set: (x, y) => ([cx, cy] = [x, y]) },
          { x: cx + r, y: cy, set: (x, y) => (r = Math.max(1, Math.hypot(x - cx, y - cy))) },
        ],
        attributes: () => ({ cx: formatNumber(cx), cy: formatNumber(cy), r: formatNumber(r) }),
      };
    }
    case "rect": {
      let [x0, y0] = [n("x"), n("y")];
      let [x1, y1] = [x0 + n("width"), y0 + n("height")];
      return {
        handles: () => [
          { x: x0, y: y0, set: (x, y) => ([x0, y0] = [Math.min(x, x1 - 1), Math.min(y, y1 - 1)]) },
          { x: x1, y: y1, set: (x, y) => ([x1, y1] = [Math.max(x, x0 + 1), Math.max(y, y0 + 1)]) },
        ],
        attributes: () => ({
          x: formatNumber(x0),
          y: formatNumber(y0),
          width: formatNumber(x1 - x0),
          height: formatNumber(y1 - y0),
        }),
      };
    }
    case "path": {
      const commands = parsePath(el.getAttribute("d"));
      // Each command's start point: the previous command's end.
      const starts = () => {
        let at = null;
        let first = null;
        return commands.map((c) => {
          const start = at;
          if (c.cmd === "M") first = [c.x, c.y];
          at = c.cmd === "Z" ? first : [c.x, c.y];
          return start;
        });
      };
      return {
        handles: () => {
          const out = [];
          const from = starts();
          commands.forEach((c, i) => {
            if (c.cmd === "Z") return;
            out.push({
              x: c.x,
              y: c.y,
              set: (x, y) => Object.assign(c, { x, y }),
              remove: () => c.cmd === "L" && commands.filter((d) => d.cmd !== "Z").length > 3 && commands.splice(i, 1),
            });
            if (c.cmd === "A") {
              const [mx, my] = arcMidpoint(from[i], [c.x, c.y], c.r, c.large, c.sweep);
              out.push({
                x: mx,
                y: my,
                arc: true,
                set: (x, y) => {
                  const arc = arcThrough(from[i], [x, y], [c.x, c.y]);
                  if (arc) Object.assign(c, arc);
                },
              });
            }
          });
          return out;
        },
        // Only straight edges (a line, or the closing edge) get new corners.
        insert: (x, y) => {
          const from = starts();
          let best = null;
          commands.forEach((c, i) => {
            if (c.cmd !== "L" && c.cmd !== "Z") return;
            const to = c.cmd === "Z" ? commands[0] : c;
            const d = segmentDistance([x, y], from[i], [to.x, to.y]);
            if (!best || d < best.d) best = { d, i };
          });
          if (best) commands.splice(best.i, 0, { cmd: "L", x, y });
        },
        attributes: () => ({ d: formatPath(commands) }),
      };
    }
  }
  return null;
}

function segmentDistance(p, a, b) {
  const [dx, dy] = [b[0] - a[0], b[1] - a[1]];
  const t = Math.max(0, Math.min(1, ((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / (dx * dx + dy * dy || 1)));
  return Math.hypot(p[0] - a[0] - t * dx, p[1] - a[1] - t * dy);
}

// The index of the corner that starts the edge nearest `p`.
function nearestEdge(pts, closed, p) {
  let best = 0;
  let bestD = Infinity;
  const count = closed ? pts.length : pts.length - 1;
  for (let i = 0; i < count; i++) {
    const d = segmentDistance(p, pts[i], pts[(i + 1) % pts.length]);
    if (d < bestD) [best, bestD] = [i, d];
  }
  return best;
}

function toMap(event) {
  const point = svg.createSVGPoint();
  point.x = event.clientX;
  point.y = event.clientY;
  const { x, y } = point.matrixTransform(svg.getScreenCTM().inverse());
  return [x, y];
}

// A dragged corner lands on the nearest half millimetre.
const snap = (v) => Math.round(v * 2) / 2;

function select(shape) {
  if (selected) selected.shape.el.classList.remove("selected");
  selected = shape ? { shape, model: modelFor(shape.el) } : null;
  if (shape) shape.el.classList.add("selected");
  $("selection").textContent = shape ? `Selected: ${describe(shape)}.` : "Nothing selected.";
  $("reset-one").disabled = !shape || !edits.has(shape.index);
  drawHandles();
}

function drawHandles() {
  const group = $("handles");
  group.replaceChildren();
  if (!selected) return;
  for (const handle of selected.model.handles()) {
    const dot = document.createElementNS(SVG_NS, "circle");
    dot.setAttribute("cx", handle.x);
    dot.setAttribute("cy", handle.y);
    dot.setAttribute("r", HANDLE_MM);
    if (handle.arc) dot.classList.add("arc");
    dot.handle = handle;
    group.appendChild(dot);
  }
}

// Applies the selected shape's model to its element and records the edit.
function commit() {
  const { shape, model } = selected;
  const attributes = model.attributes();
  setAttributes(shape.el, attributes);
  const unchanged = Object.entries(attributes).every(([name, value]) => shape.original[name] === value);
  if (unchanged) edits.delete(shape.index);
  else edits.set(shape.index, attributes);
  $("reset-one").disabled = !edits.has(shape.index);
  store.set(fingerprint(source), [...edits]);
  drawHandles();
  showChanges();
}

function describe(shape) {
  const el = shape.el;
  const parts = [];
  if (el.dataset.switch) parts.push(`switch ${el.dataset.switch}`);
  if (el.dataset.coil) parts.push(`coil ${el.dataset.coil}`);
  if (el.dataset.lamp) parts.push(`lamp L${el.dataset.lamp}`);
  if (el.hasAttribute("data-wall")) parts.push("wall");
  if (el.hasAttribute("data-gate")) parts.push("gate");
  const kind = { polygon: "shape", line: "line", circle: "circle", rect: "rectangle", path: "outline" }[el.tagName];
  return parts.length ? `${kind} (${parts.join(", ")})` : `${kind} #${shape.index}`;
}

function showChanges() {
  $("changes").replaceChildren(
    ...[...edits.keys()].map((index) => {
      const li = document.createElement("li");
      li.textContent = describe(shapes[index]);
      return li;
    }),
  );
  $("download").disabled = edits.size === 0;
}

function overlayEvents() {
  svg.addEventListener("pointerdown", (event) => {
    const handle = event.target.handle;
    if (handle) {
      dragging = handle;
      svg.setPointerCapture(event.pointerId);
      event.preventDefault();
      return;
    }
    const shape = shapes.find((s) => s.el === event.target && s.editable);
    select(shape ?? null);
  });
  svg.addEventListener("pointermove", (event) => {
    if (!dragging) return;
    const [x, y] = toMap(event);
    // The handle moves the model's own points, so it stays valid while the
    // handles are redrawn.
    dragging.set(snap(x), snap(y));
    commit();
  });
  const stop = () => (dragging = null);
  svg.addEventListener("pointerup", stop);
  svg.addEventListener("pointercancel", stop);
  svg.addEventListener("dblclick", (event) => {
    if (!selected) return;
    const handle = event.target.handle;
    if (handle) {
      if (handle.remove?.()) commit();
      return;
    }
    if (event.target === selected.shape.el && selected.model.insert) {
      const [x, y] = toMap(event);
      selected.model.insert(snap(x), snap(y));
      commit();
    }
  });
}

$("reset-one").addEventListener("click", () => {
  if (!selected) return;
  const shape = selected.shape;
  setAttributes(shape.el, shape.original);
  select(shape);
  commit();
});

$("reset-all").addEventListener("click", () => {
  for (const index of edits.keys()) setAttributes(shapes[index].el, shapes[index].original);
  edits.clear();
  store.set(fingerprint(source), []);
  select(null);
  showChanges();
});

$("download").addEventListener("click", () => {
  const blob = new Blob([rewriteSvg(source, edits)], { type: "image/svg+xml" });
  const link = document.createElement("a");
  link.href = URL.createObjectURL(blob);
  link.download = "playfield.svg";
  link.click();
  URL.revokeObjectURL(link.href);
});

$("back").addEventListener("click", () => showStep("points"));
$("opacity").addEventListener("input", (event) => {
  document.documentElement.style.setProperty("--map-opacity", event.target.value);
});
$("labels").addEventListener("change", (event) => $("overlay").classList.toggle("hide-labels", !event.target.checked));

// ---------------------------------------------------------------------------
// Steps and zoom

function showStep(step) {
  $("step-points").classList.toggle("hidden", step === "photo");
  $("step-trace").classList.toggle("hidden", step !== "trace" || !fitted);
  $("photo-view").classList.toggle("hidden", step !== "points");
  $("trace-view").classList.toggle("hidden", step !== "trace");
  zoomTo(zoom);
}

function zoomTo(level) {
  const view = $("photo-view").classList.contains("hidden") ? $("trace-view") : $("photo-view");
  const stage = view.firstElementChild;
  // Keep the middle of the view where it was.
  const cx = (view.scrollLeft + view.clientWidth / 2) / (view.scrollWidth || 1);
  const cy = (view.scrollTop + view.clientHeight / 2) / (view.scrollHeight || 1);
  zoom = Math.max(1, Math.min(level, 16));
  stage.style.width = `${view.clientWidth * zoom}px`;
  view.scrollLeft = cx * view.scrollWidth - view.clientWidth / 2;
  view.scrollTop = cy * view.scrollHeight - view.clientHeight / 2;
}

$("zoom-in").addEventListener("click", () => zoomTo(zoom * 1.5));
$("zoom-out").addEventListener("click", () => zoomTo(zoom / 1.5));
$("zoom-fit").addEventListener("click", () => zoomTo(1));
addEventListener("keydown", (event) => {
  if (event.target instanceof HTMLInputElement) return;
  if (event.key === "+" || event.key === "=") zoomTo(zoom * 1.5);
  else if (event.key === "-") zoomTo(zoom / 1.5);
  else if (event.key === "Escape") select(null);
});

load().then(overlayEvents, (error) => {
  status(`Couldn't load the map: ${error.message}. Build it with pnpm run build:sim and serve it with pnpm run serve:sim.`);
});
