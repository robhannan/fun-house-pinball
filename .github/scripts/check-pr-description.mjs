// Checks a pull request description against .github/pull_request_template.md
// and the PR rules in CLAUDE.md. Run by .github/workflows/pr-checks.yml, or
// locally with: PR_BODY="$(gh pr view <n> --json body -q .body)" npm run check:pr-description
//
// Inputs (environment):
//   PR_BODY   the PR description
//   BASE_SHA  base commit of the PR (optional; enables the coil-safety check)
//   HEAD_SHA  head commit of the PR (optional)
import { $ } from "zx";

$.verbose = false;

// The template's placeholder GIF, which every PR should replace.
const PLACEHOLDER_GIF_ID = "xULW8hDrQXqSTkbMju";

const body = process.env.PR_BODY ?? "";
const problems = [];

if (!/^## Summary/m.test(body)) {
  problems.push("Missing a '## Summary' section (see .github/pull_request_template.md).");
}
if (!/^## Notes/m.test(body)) {
  problems.push("Missing a '## Notes' section (see .github/pull_request_template.md).");
}
if (body.includes("<!--")) {
  problems.push("Template comments (<!-- ... -->) are still in the description; delete them.");
}

// The last image in the description is the GIF.
const gif = [...body.matchAll(/!\[[^\]]*\]\((https?:\/\/[^)\s]+)\)/g)].at(-1)?.[1];
if (!gif) {
  problems.push("No GIF at the bottom of the description (see .github/pull_request_template.md).");
} else {
  if (gif.includes(PLACEHOLDER_GIF_ID)) {
    problems.push("The GIF is still the template's placeholder; replace it with one relevant to this change.");
  }
  const loadError = await checkImage(gif);
  if (loadError) problems.push(`The GIF doesn't load as an image: ${gif} (${loadError}).`);
}

// Coil safety (CLAUDE.md): any change to COIL_KINDS must be called out with
// the coil and its rating.
const { BASE_SHA: base, HEAD_SHA: head } = process.env;
if (base && head) {
  const diff = await $`git diff ${`${base}...${head}`} -- crates/pinball-core/src/playfield.rs`;
  if (/^[+-]\s*k\[coil::/m.test(diff.stdout) && !/rating/i.test(body)) {
    problems.push(
      "COIL_KINDS changed, so the description must name each changed coil and its rating (CLAUDE.md, coil safety).",
    );
  }
}

for (const problem of problems) console.log(`::error::${problem}`);
if (problems.length > 0) {
  process.exitCode = 1;
} else {
  console.log("PR description looks good.");
}

/** Returns why `url` doesn't load as an image, or undefined if it does. */
async function checkImage(url) {
  try {
    const response = await fetch(url, { method: "HEAD", redirect: "follow", signal: AbortSignal.timeout(20_000) });
    const type = response.headers.get("content-type") ?? "no content type";
    if (!response.ok) return `HTTP ${response.status}`;
    if (!type.startsWith("image/")) return type;
    return undefined;
  } catch (error) {
    return error.message;
  }
}
