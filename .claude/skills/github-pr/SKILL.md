---
name: github-pr
description: Open or update a pull request for this repo. Use whenever creating a PR, pushing follow-up work for review, or writing a PR description. Covers branch hygiene, local checks, filling in the PR template, and choosing a GIF relevant to the change.
---

# Opening a PR in fun-house-pinball

## 1. Start from the right branch

Do this before every commit you intend to push, not just the first one.

```sh
git fetch origin
gh pr list --state all --head "$(git branch --show-current)" --json number,state
```

- On `main`: create a branch from `origin/main` with a short, descriptive name.
- The current branch's PR is **MERGED** or **CLOSED**: don't push to it. Create
  a new branch from `origin/main` and move the work across, for example with
  `git cherry-pick`.
- Never edit a merged PR's description to describe later work.
- Keep unrelated changes on separate branches, each cut from `origin/main`.

If `gh` acts as the wrong GitHub account, a `GITHUB_TOKEN` environment variable
may be overriding the stored login. Run `gh auth status` to check.

## 2. Run the same checks as CI

```sh
cargo fmt --all --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
(cd firmware && cargo fmt --check && cargo clippy --locked -- -D warnings)
```

The firmware can't link without `avr-gcc`. If it isn't installed, say in the PR
that only CI has done the full AVR build.

## 3. Write the description from the template

Use `.github/pull_request_template.md`, keeping its sections in order:

- **Summary**: one line on what changed and why, then bullets grouped by area,
  such as `crates/pinball-core`, `firmware/` and hardware/docs. Name files in
  backticks.
- **Notes**: anything a reviewer needs. Cover:
  - assumptions and guesses, especially rules or counts not checked against
    the real table or schematic
  - what was tested on the real machine versus only with `cargo test`
  - **any coil timing change**, with the coil and its rating

  If there's nothing to say, write `None`.
- **GIF**: see the next step.

**If the PR changes `docs/playfield.svg` or `docs/backboard.svg`**, show the
updated drawing at the end of the Summary, under a `### Rendered playfield` (or
`### Rendered backboard`) heading. Link the raw file at the PR's latest commit,
so it shows this PR's version and keeps working after the branch is deleted:

```html
<img src="https://raw.githubusercontent.com/robhannan/fun-house-pinball/<full commit SHA>/docs/playfield.svg" alt="the updated playfield wiring map" width="600">
```

Push first, so the commit exists on GitHub. After pushing more commits that
change the drawing, update the SHA in the description.

Delete the template's `<!-- ... -->` comments. Write the body to a file and pass
it with `gh pr create --body-file`; don't use a heredoc inside `--body`.

## 4. Replace the GIF with one relevant to this change

The template ships with a generic pinball GIF as a placeholder. Swap it for one
that fits this PR. Examples:

- **Coil or driver work**: sparks, electricity, a solenoid or a hammer
- **Scoring or rules**: a scoreboard, points, a jackpot
- **Bug fix**: squashing a bug
- **Docs or tooling**: books, a checklist, tidying up

Pinball-themed GIFs are always welcome.

How to find one:

1. Search the web restricted to `giphy.com` for a short query, such as
   `pinball flipper gif` or `electric sparks gif`.
2. From a result page `https://giphy.com/gifs/<slug>-<ID>`, build the direct
   link `https://media.giphy.com/media/<ID>/giphy.gif`.
3. Check that it resolves to a GIF:
   `curl -sIL <url> | grep -iE '^HTTP|content-type'` should show `200` and
   `image/gif`.
4. If you can, download it and view a frame to confirm it shows what the page
   title says.

Rules:

- Make the alt text describe the GIF, e.g.
  `![flipper hitting a ball](https://media.giphy.com/media/<ID>/giphy.gif)`.
- If no suitable GIF can be verified, keep the template's pinball GIF rather than
  link one you haven't checked.

## 5. Open the PR and watch CI

```sh
git push -u origin <branch>
gh pr create --base main --title "<imperative summary>" --body-file <file>
gh pr checks <number> --watch
```

Report the PR link and the CI result. If CI fails, fix it on the same branch,
but only while the PR is still open (see step 1).
