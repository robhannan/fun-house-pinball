// Builds the browser simulator into sim/web/generated/. Run with:
//   npm run build:sim
import { $, fs, path } from "zx";

const root = path.resolve(import.meta.dirname, "..");
const out = path.join(root, "sim/web/generated");

$.cwd = root;
await $`cargo build --release --locked -p pinball-sim --target wasm32-unknown-unknown`;

await fs.ensureDir(out);
await fs.copy(path.join(root, "target/wasm32-unknown-unknown/release/pinball_sim.wasm"), path.join(out, "pinball_sim.wasm"));
await fs.copy(path.join(root, "docs/playfield.svg"), path.join(out, "playfield.svg"));

console.log("Built. Run: python3 -m http.server -d sim/web 8000");
console.log("Then open http://localhost:8000");
