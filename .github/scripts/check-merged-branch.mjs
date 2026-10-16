// Fails if this branch's pull request has already been merged (and there's no
// new open PR for it): commits pushed now would never reach main. Run by
// .github/workflows/branch-checks.yml.
//
// Inputs (environment):
//   BRANCH              the branch that was pushed
//   GITHUB_REPOSITORY   owner/repo
//   GH_TOKEN            token for the gh CLI
import { $ } from "zx";

$.verbose = false;

const { BRANCH: branch, GITHUB_REPOSITORY: repo } = process.env;
if (!branch || !repo) throw new Error("BRANCH and GITHUB_REPOSITORY must be set");

async function prNumbers(state) {
  const result = await $`gh pr list -R ${repo} --head ${branch} --state ${state} --json number`;
  return JSON.parse(result.stdout).map((pr) => pr.number);
}

const [merged, open] = await Promise.all([prNumbers("merged"), prNumbers("open")]);
if (merged.length > 0 && open.length === 0) {
  console.log(
    `::error::PR #${merged.join(", #")} from '${branch}' is already merged, so these commits won't reach main. ` +
      "Start a new branch from origin/main and open a new PR.",
  );
  process.exitCode = 1;
} else {
  console.log(`OK: no merged PR for '${branch}' without an open one.`);
}
