// Turns the printable parts in hardware/3d/ into STL files. Run with:
//   pnpm run export:3d     writes target/3d/<part>.stl, ready for a slicer or a
//                          printing service
//   pnpm run check:3d      only checks every part (and the assembly) renders
//                          cleanly, with no warnings
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

// Shared settings aren't parts; the assembly is only checked, not printed.
const notParts = new Set(["measurements.scad", "common.scad", "assembly.scad"]);
const files = (await fs.readdir(dir)).filter((f) => f.endsWith(".scad")).sort();
const parts = files.filter((f) => !notParts.has(f));
const toRender = checkOnly ? [...parts, "assembly.scad"] : parts;

const problems = [];
for (const file of toRender) {
  const stl = path.join(out, file.replace(/\.scad$/, ".stl"));
  const run = spawnSync(openscad, ["-o", stl, path.join(dir, file)], { encoding: "utf8" });
  const messages = `${run.stdout}${run.stderr}`;
  const warnings = messages.split("\n").filter((line) => /WARNING|ERROR/.test(line));
  if (run.status !== 0 || warnings.length > 0) {
    problems.push(`${file}: ${warnings.join(" / ") || `OpenSCAD exited with ${run.status}`}`);
  } else if (!checkOnly) {
    console.log(`${path.relative(root, stl)}`);
  }
}

if (checkOnly) await fs.remove(out);
if (problems.length > 0) {
  for (const problem of problems) console.log(`::error::${problem}`);
  process.exitCode = 1;
} else {
  console.log(checkOnly ? `OK: ${toRender.length} models render cleanly.` : `Exported ${parts.length} parts.`);
}
