// Builds the GitHub Pages site into site/generated/: the browser simulator
// (built by sim/build.mjs) and the parts tracker, with its items read from
// docs/shopping-list.md. Fails if a row can't be read. Run with:
//   pnpm run build:site
import { $, fs, path } from "zx";
import { parseShoppingList } from "./shopping-list.mjs";

const root = path.resolve(import.meta.dirname, "..");
const out = path.join(root, "site/generated");

$.verbose = false;
$.cwd = root;

const { sections, errors } = parseShoppingList(await fs.readFile(path.join(root, "docs/shopping-list.md"), "utf8"));
if (errors.length) {
  for (const error of errors) console.log(`::error file=docs/shopping-list.md::${error}`);
  process.exit(1);
}

// The repository the counts are saved to: the one being built in CI,
// otherwise this checkout's origin.
const remote = process.env.GITHUB_REPOSITORY ?? (await $`git remote get-url origin`).stdout.trim();
const repo = remote.match(/([^/:]+\/[^/]+?)(?:\.git)?$/)[1];

// The simulator needs Rust with the wasm32 target (rust-toolchain.toml).
// Quiet unless it fails, when zx prints its output.
await $`node sim/build.mjs`;

await fs.emptyDir(out);
await fs.copy(path.join(root, "sim/web"), path.join(out, "sim"));
await fs.copy(path.join(root, "site/index.html"), path.join(out, "index.html"));
await fs.copy(path.join(root, "site/parts"), path.join(out, "parts"), { filter: (file) => !file.endsWith(".test.mjs") });
// Counts and orders are saved to parts.json. got.json holds counts from
// before the ordered option; it's only read until parts.json exists, so a
// tab still running the old page can't overwrite the new file.
await fs.writeJson(path.join(out, "parts/items.json"), { repo, branch: "parts-data", file: "parts.json", legacyFile: "got.json", sections });

const items = sections.flatMap((s) => s.groups.flatMap((g) => g.items));
console.log(`Built the simulator, and ${items.length} parts in ${sections.length} sections, saving to ${repo}.`);
console.log("Preview with: pnpm run serve:site");
