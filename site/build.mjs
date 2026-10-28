// Builds the GitHub Pages site into site/generated/: the parts tracker, with
// its items read from docs/shopping-list.md. Fails if a row can't be read.
// Run with:
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

await fs.emptyDir(out);
await fs.copy(path.join(root, "site/index.html"), path.join(out, "index.html"));
await fs.copy(path.join(root, "site/parts"), path.join(out, "parts"));
await fs.writeJson(path.join(out, "parts/items.json"), { repo, branch: "parts-data", file: "got.json", sections });

const items = sections.flatMap((s) => s.groups.flatMap((g) => g.items));
console.log(`Built ${items.length} parts in ${sections.length} sections, saving to ${repo}.`);
console.log("Preview with: pnpm run serve:site");
