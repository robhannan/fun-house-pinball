// Fails if build output, dependencies or OS files are committed. Run by
// .github/workflows/ci.yml, or locally with: pnpm run check:stray-files
import { $ } from "zx";

$.verbose = false;

const STRAY = [
  /(^|\/)\.DS_Store$/,
  /(^|\/)target\//,
  /(^|\/)node_modules\//,
  /^sim\/web\/generated\//,
];

const files = (await $`git ls-files`).stdout.split("\n").filter(Boolean);
const stray = files.filter((file) => STRAY.some((pattern) => pattern.test(file)));

if (stray.length > 0) {
  console.log("::error::These files shouldn't be committed:");
  for (const file of stray) console.log(file);
  process.exitCode = 1;
} else {
  console.log(`OK: none of ${files.length} tracked files are build output or OS files.`);
}
