// Builds the browser simulator into sim/web/generated/. Run with:
//   pnpm run build:sim
import { $, fs, path } from "zx";

const root = path.resolve(import.meta.dirname, "..");
const out = path.join(root, "sim/web/generated");

$.cwd = root;
await $`cargo build --release --locked -p pinball-sim --target wasm32-unknown-unknown`;

await fs.ensureDir(out);
await fs.copy(path.join(root, "target/wasm32-unknown-unknown/release/pinball_sim.wasm"), path.join(out, "pinball_sim.wasm"));
await fs.copy(path.join(root, "docs/playfield.svg"), path.join(out, "playfield.svg"));
await fs.copy(path.join(root, "docs/backboard.svg"), path.join(out, "backboard.svg"));

console.log("Built. Run: pnpm run serve:sim");
console.log("Then open (or reload) http://localhost:8000");
