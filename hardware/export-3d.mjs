// Turns the printable parts in hardware/3d/ into STL files. Run with:
//   pnpm run export:3d     writes target/3d/<part>.stl (and one file per
//                          segment of the long runs), ready for a slicer or a
//                          printing service
//   pnpm run check:3d      checks every part renders cleanly and fits the
//                          printer, and runs the collision checks in
//                          hardware/3d/checks/
//
// Needs a recent OpenSCAD on the PATH (`openscad`, or `openscad-nightly` as CI
// installs it), or set OPENSCAD to its path. See docs/3d-printing.md.
import { spawnSync } from "node:child_process";
import { tmpdir } from "node:os";

import { argv, fs, path, which } from "zx";

const root = path.resolve(import.meta.dirname, "..");
const dir = path.join(root, "hardware/3d");
const checkOnly = argv.check === true;
const out = checkOnly ? await fs.mkdtemp(path.join(tmpdir(), "fun-house-3d-")) : path.join(root, "target/3d");
await fs.ensureDir(out);

const openscad =
  process.env.OPENSCAD ?? (await which("openscad", { nothrow: true })) ?? (await which("openscad-nightly", { nothrow: true }));
if (!openscad) {
  console.error("OpenSCAD isn't installed: see docs/3d-printing.md.");
  process.exit(1);
}

const measurements = await fs.readFile(path.join(dir, "measurements.scad"), "utf8");
const maxPrint = Number(/^max_print_length\s*=\s*([\d.]+)/m.exec(measurements)?.[1] ?? 170);

/** Runs OpenSCAD, returning its exit status and everything it printed. */
function render(file, output, defines = []) {
  const args = ["-o", output, ...defines.flatMap((d) => ["-D", d]), file];
  const run = spawnSync(openscad, args, { encoding: "utf8" });
  return { status: run.status, text: `${run.stdout}${run.stderr}` };
}

/** The lowest and highest corners of an STL file's contents: [[x, y, z], [x, y, z]]. */
async function bounds(stl) {
  const data = await fs.readFile(stl);
  const points = [];
  if (data.subarray(0, 5).toString() === "solid" && data.subarray(0, 300).includes("facet")) {
    for (const m of data.toString().matchAll(/vertex\s+(\S+)\s+(\S+)\s+(\S+)/g)) points.push(m.slice(1, 4).map(Number));
  } else {
    const count = data.readUInt32LE(80);
    for (let i = 0; i < count; i++)
      for (let v = 0; v < 3; v++) {
        const at = 84 + i * 50 + 12 + v * 12;
        points.push([data.readFloatLE(at), data.readFloatLE(at + 4), data.readFloatLE(at + 8)]);
      }
  }
  const low = [Infinity, Infinity, Infinity];
  const high = [-Infinity, -Infinity, -Infinity];
  for (const p of points)
    for (let axis = 0; axis < 3; axis++) {
      low[axis] = Math.min(low[axis], p[axis]);
      high[axis] = Math.max(high[axis], p[axis]);
    }
  return [low, high];
}

/** The size of an STL file's contents, [x, y, z]. */
async function size(stl) {
  const [low, high] = await bounds(stl);
  return high.map((h, axis) => h - low[axis]);
}

const problems = [];
const warningsIn = (text) => text.split("\n").filter((line) => /WARNING|ERROR/.test(line));

// Shared settings and the layout aren't parts; the assembly is only checked.
const notParts = new Set(["measurements.scad", "common.scad", "placement.scad", "assembly.scad"]);
const parts = (await fs.readdir(dir)).filter((f) => f.endsWith(".scad") && !notParts.has(f)).sort();
let exported = 0;
for (const file of parts) {
  const name = file.replace(/\.scad$/, "");
  const first = render(path.join(dir, file), path.join(out, `${name}.stl`));
  // Long runs come in segments: export each.
  const segments = Number(/SEGMENTS (\d+)/.exec(first.text)?.[1] ?? 0);
  const outputs = segments > 0 ? Array.from({ length: segments }, (_, i) => i + 1) : [0];
  for (const k of outputs) {
    const stl = path.join(out, k === 0 ? `${name}.stl` : `${name}_${k}.stl`);
    const result = k <= 1 ? first : render(path.join(dir, file), stl, [`segment=${k}`]);
    if (k === 1) await fs.move(path.join(out, `${name}.stl`), stl, { overwrite: true });
    const warnings = warningsIn(result.text);
    if (result.status !== 0 || warnings.length > 0) {
      problems.push(`${path.basename(stl)}: ${warnings.join(" / ") || `OpenSCAD exited with ${result.status}`}`);
      continue;
    }
    const [x, y, z] = await size(stl);
    if (Math.max(x, y, z) > maxPrint) {
      problems.push(`${path.basename(stl)}: ${x.toFixed(0)} x ${y.toFixed(0)} x ${z.toFixed(0)} mm is bigger than max_print_length (${maxPrint} mm)`);
    }
    exported += 1;
    if (!checkOnly) console.log(`${path.relative(root, stl)}  ${x.toFixed(0)} x ${y.toFixed(0)} x ${z.toFixed(0)} mm`);
  }
}

// The assembly must render; its depth is the space needed under the playfield.
const assemblyStl = path.join(checkOnly ? out : await fs.mkdtemp(path.join(tmpdir(), "fun-house-3d-")), "assembly.stl");
const assembly = render(path.join(dir, "assembly.scad"), assemblyStl);
if (assembly.status !== 0 || warningsIn(assembly.text).length > 0) {
  problems.push(`assembly.scad: ${warningsIn(assembly.text).join(" / ") || `OpenSCAD exited with ${assembly.status}`}`);
} else {
  // The playfield's underside is z = 0.
  const depth = -(await bounds(assemblyStl))[0][2];
  console.log(`The ball return reaches ${depth.toFixed(0)} mm below the playfield, plus the ball-serve coil under the ball lift.`);
}

if (checkOnly) {
  // Every collision check must come out empty.
  const checks = (await fs.readdir(path.join(dir, "checks"))).filter((f) => f.endsWith(".scad")).sort();
  for (const file of checks) {
    const result = render(path.join(dir, "checks", file), path.join(out, `check-${file}.stl`));
    const warnings = warningsIn(result.text).filter((line) => !line.includes("Current top level object is empty"));
    if (warnings.length > 0) problems.push(`checks/${file}: ${warnings.join(" / ")}`);
    else if (!result.text.includes("Current top level object is empty")) {
      problems.push(`checks/${file}: parts collide (render it in OpenSCAD to see where)`);
    }
  }
  await fs.remove(out);
  if (problems.length === 0) console.log(`OK: ${exported} parts render and fit the printer, and ${checks.length} collision checks pass.`);
} else if (problems.length === 0) {
  console.log(`Exported ${exported} parts.`);
}

if (problems.length > 0) {
  for (const problem of problems) console.log(`::error::${problem}`);
  process.exitCode = 1;
}
