// Shape editing for the trace page, kept apart from the page so it can be
// tested: SVG path data, arcs, and writing edits back into the map's source
// text without reformatting the rest of the file.

/** The shape elements the page can edit, in source order. */
export const SHAPE_TAG = /<(polygon|line|circle|rect|path)\b[^>]*>/g;

/**
 * A number as the map writes it: at most two decimal places, so numbers that
 * weren't moved (such as a measured 12.7) are written exactly as they were.
 */
export function formatNumber(v) {
  return String(Math.round(v * 100) / 100);
}

/**
 * Path data as a list of commands, or null if it uses anything but absolute
 * M, L, A (circular, unrotated) and Z: the only kinds the map uses.
 */
export function parsePath(d) {
  const tokens = d.match(/[A-Za-z]|-?\d*\.?\d+(?:e-?\d+)?/g) ?? [];
  const out = [];
  let i = 0;
  const num = () => Number(tokens[i++]);
  while (i < tokens.length) {
    const cmd = tokens[i++];
    if (cmd === "M" || cmd === "L") out.push({ cmd, x: num(), y: num() });
    else if (cmd === "A") {
      const rx = num(), ry = num(), rotation = num(), large = num(), sweep = num();
      if (rx !== ry || rotation !== 0) return null;
      out.push({ cmd, r: rx, large, sweep, x: num(), y: num() });
    } else if (cmd === "Z") out.push({ cmd });
    else return null;
  }
  return out;
}

export function formatPath(commands) {
  const f = formatNumber;
  return commands
    .map((c) => {
      if (c.cmd === "Z") return "Z";
      if (c.cmd === "A") return `A ${f(c.r)} ${f(c.r)} 0 ${c.large} ${c.sweep} ${f(c.x)} ${f(c.y)}`;
      return `${c.cmd} ${f(c.x)} ${f(c.y)}`;
    })
    .join(" ");
}

/** The centre of an SVG arc from `a` to `b` (a radius too small is stretched, as SVG does). */
export function arcCentre(a, b, r, large, sweep) {
  const [mx, my] = [(a[0] + b[0]) / 2, (a[1] + b[1]) / 2];
  const [dx, dy] = [b[0] - a[0], b[1] - a[1]];
  const half = Math.hypot(dx, dy) / 2;
  const offset = Math.sqrt(Math.max(r * r - half * half, 0));
  const sign = large !== sweep ? 1 : -1;
  return [mx + (sign * offset * -dy) / (2 * half), my + (sign * offset * dx) / (2 * half)];
}

/** The point half-way along an SVG arc. */
export function arcMidpoint(a, b, r, large, sweep) {
  const c = arcCentre(a, b, r, large, sweep);
  const radius = Math.max(r, Math.hypot(b[0] - a[0], b[1] - a[1]) / 2);
  const start = Math.atan2(a[1] - c[1], a[0] - c[0]);
  let turn = Math.atan2(b[1] - c[1], b[0] - c[0]) - start;
  if (sweep && turn < 0) turn += 2 * Math.PI;
  if (!sweep && turn > 0) turn -= 2 * Math.PI;
  const mid = start + turn / 2;
  return [c[0] + radius * Math.cos(mid), c[1] + radius * Math.sin(mid)];
}

/** The SVG arc from `a` through `m` to `b`, or null if they're in a line. */
export function arcThrough(a, m, b) {
  const cross = (m[0] - a[0]) * (b[1] - a[1]) - (m[1] - a[1]) * (b[0] - a[0]);
  if (Math.abs(cross) < 1e-6) return null;
  // The circumcircle's centre.
  const d = 2 * (a[0] * (m[1] - b[1]) + m[0] * (b[1] - a[1]) + b[0] * (a[1] - m[1]));
  const sq = (p) => p[0] * p[0] + p[1] * p[1];
  const cx = (sq(a) * (m[1] - b[1]) + sq(m) * (b[1] - a[1]) + sq(b) * (a[1] - m[1])) / d;
  const cy = (sq(a) * (b[0] - m[0]) + sq(m) * (a[0] - b[0]) + sq(b) * (m[0] - a[0])) / d;
  const r = Math.hypot(a[0] - cx, a[1] - cy);
  // More than half a circle when the centre is on the same side of the chord
  // as the middle point.
  const side = (p) => (p[0] - a[0]) * (b[1] - a[1]) - (p[1] - a[1]) * (b[0] - a[0]);
  const large = Math.sign(side([cx, cy])) === Math.sign(side(m)) ? 1 : 0;
  return { r, large, sweep: cross > 0 ? 1 : 0 };
}

/**
 * Where a circle's handles go, in drawing order: the radius handle on its
 * right edge, then the centre. The centre is drawn last, so on a circle
 * smaller than a handle, dragging from the middle still moves it rather than
 * resizing it.
 */
export function circleHandles(cx, cy, r) {
  return [
    { kind: "radius", x: cx + r, y: cy },
    { kind: "centre", x: cx, y: cy },
  ];
}

/**
 * The map's source text with some shapes' attributes changed. `edits` maps a
 * shape's index (its position among SHAPE_TAG matches) to the attributes to
 * set; everything else, including formatting, stays as it was.
 */
export function rewriteSvg(source, edits) {
  let index = 0;
  return source.replace(SHAPE_TAG, (tag) => {
    const attributes = edits.get(index++);
    if (!attributes) return tag;
    let out = tag;
    for (const [name, value] of Object.entries(attributes)) {
      const pattern = new RegExp(`(\\s${name}=")[^"]*(")`);
      if (!pattern.test(out)) throw new Error(`${tag} has no ${name}`);
      out = out.replace(pattern, `$1${value}$2`);
    }
    return out;
  });
}
