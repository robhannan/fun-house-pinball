// A homography: the projective transform between two flat planes, such as
// the playfield map (in millimetres) and a photo of the playfield (in pixels).
// It handles a photo taken at a slight angle, but not a lens's own distortion.
//
// A homography is 9 numbers, a 3 x 3 matrix in row order, applied to (x, y, 1).

/** Applies `h` to the point `[x, y]`. */
export function applyHomography(h, [x, y]) {
  const w = h[6] * x + h[7] * y + h[8];
  return [(h[0] * x + h[1] * y + h[2]) / w, (h[3] * x + h[4] * y + h[5]) / w];
}

/** The homography that undoes `h`. */
export function invertHomography(h) {
  const [a, b, c, d, e, f, g, i, j] = h;
  const adj = [e * j - f * i, c * i - b * j, b * f - c * e, f * g - d * j, a * j - c * g, c * d - a * f, d * i - e * g, b * g - a * i, a * e - b * d];
  const det = a * adj[0] + b * adj[3] + c * adj[6];
  if (Math.abs(det) < 1e-12) throw new Error("the transform can't be undone");
  return scaled(adj, 1 / det);
}

function multiply(p, q) {
  const out = [];
  for (let r = 0; r < 3; r++)
    for (let c = 0; c < 3; c++) out.push(p[r * 3] * q[c] + p[r * 3 + 1] * q[3 + c] + p[r * 3 + 2] * q[6 + c]);
  return out;
}

function scaled(h, k) {
  return h.map((v) => v * k);
}

// Moves points to centre on the origin, about √2 from it on average, so the
// fit's arithmetic stays accurate (Hartley's normalisation).
function normaliser(points) {
  const cx = points.reduce((s, p) => s + p[0], 0) / points.length;
  const cy = points.reduce((s, p) => s + p[1], 0) / points.length;
  const spread = points.reduce((s, p) => s + Math.hypot(p[0] - cx, p[1] - cy), 0) / points.length;
  const k = Math.SQRT2 / (spread || 1);
  return [k, 0, -k * cx, 0, k, -k * cy, 0, 0, 1];
}

/**
 * The homography taking each point in `from` to the matching point in `to`,
 * fitted by least squares so that small errors in the points average out.
 * Needs at least four pairs, not all in a line.
 */
export function fitHomography(from, to) {
  if (from.length !== to.length) throw new Error("the point lists differ in length");
  if (from.length < 4) throw new Error("at least four points are needed");
  const tFrom = normaliser(from);
  const tTo = normaliser(to);
  // Each pair gives two equations in the first eight numbers (the ninth is 1).
  const ata = Array.from({ length: 8 }, () => new Array(8).fill(0));
  const atb = new Array(8).fill(0);
  for (let n = 0; n < from.length; n++) {
    const [x, y] = applyHomography(tFrom, from[n]);
    const [u, v] = applyHomography(tTo, to[n]);
    for (const [row, rhs] of [
      [[x, y, 1, 0, 0, 0, -x * u, -y * u], u],
      [[0, 0, 0, x, y, 1, -x * v, -y * v], v],
    ]) {
      for (let r = 0; r < 8; r++) {
        atb[r] += row[r] * rhs;
        for (let c = 0; c < 8; c++) ata[r][c] += row[r] * row[c];
      }
    }
  }
  const h = solve(ata, atb);
  const fitted = multiply(invertHomography(tTo), multiply([...h, 1], tFrom));
  return scaled(fitted, 1 / fitted[8]);
}

// Solves a x = b by Gaussian elimination with partial pivoting.
function solve(a, b) {
  const n = b.length;
  const m = a.map((row, r) => [...row, b[r]]);
  for (let col = 0; col < n; col++) {
    let pivot = col;
    for (let r = col + 1; r < n; r++) if (Math.abs(m[r][col]) > Math.abs(m[pivot][col])) pivot = r;
    if (Math.abs(m[pivot][col]) < 1e-9) throw new Error("the points are too close to a straight line");
    [m[col], m[pivot]] = [m[pivot], m[col]];
    for (let r = 0; r < n; r++) {
      if (r === col) continue;
      const k = m[r][col] / m[col][col];
      for (let c = col; c <= n; c++) m[r][c] -= k * m[col][c];
    }
  }
  return m.map((row, r) => row[n] / row[r]);
}
