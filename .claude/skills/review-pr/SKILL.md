---
name: review-pr
description: Review a pull request in this repo with fresh context, in a separate subagent that hasn't seen the conversation that wrote the code. Use when asked to review a PR, before asking for a merge, or to get a second opinion on a change.
argument-hint: "[pr-number]"
context: fork
agent: general-purpose
---

# Review a fun-house-pinball pull request

You are reviewing a pull request for **fun-house-pinball**: a Rust controller
for a restored 1956 Williams *Fun House* pinball machine, run by an Arduino Mega
2560. You have **no context** beyond this file and the repository. Don't assume
anything the PR description claims; check it against the code.

The PR to review: **$ARGUMENTS**. If that's empty, review the open PR for the
current branch (`gh pr view --json number`).

This review is **read-only**:

- Don't edit files, commit, push, or post comments or reviews on GitHub.
- Report back; the person who asked decides what to do with the findings.

If `gh` acts as the wrong account, a `GITHUB_TOKEN` environment variable may be
overriding the stored login; run `gh` commands with it unset.

## 1. Get the PR without touching the current checkout

```sh
gh pr view <n> --json number,title,body,state,baseRefName,headRefName,files,commits
git fetch origin main              # updates origin/main for the diff
git fetch origin "pull/<n>/head"   # FETCH_HEAD is now the PR's head
dir="$(mktemp -d)/pr-<n>"
git worktree add --detach "$dir" FETCH_HEAD
```

Shell variables don't survive between separate commands, so **note the path**
that `$dir` expands to (`echo "$dir"`) and use it literally from then on. Do all
reading and running inside that directory. When finished, run
`git worktree remove --force <path>`.

If the PR is merged or closed, say so at the top of the report.

## 2. Learn the rules

Read these first, from the PR's version of the repo:

- `CLAUDE.md`: project and PR rules
- `.github/pull_request_template.md`
- `.claude/skills/github-pr/SKILL.md`
- `README.md`

## 3. Read the change properly

- In the worktree, run `git diff origin/main...HEAD --stat`, then the full diff.
  `origin/main` was updated by the first fetch, and `HEAD` is the PR.
- **Read every changed file in full**, not just the diff hunks, so you see how
  the change fits the code around it.
- For each claim in the PR description (tests added, numbers, "no coil timing
  changes", "tested on hardware"), find the evidence or flag it.

## 4. Run the checks

In `$dir`, run the same checks as CI (`.github/workflows/ci.yml`):

```sh
cargo fmt --all --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
(cd firmware && cargo fmt --check && cargo clippy --locked -- -D warnings)
```

Also run `pnpm install && pnpm run build:sim` if `sim/` exists. It needs the WebAssembly target
(`rustup target add wasm32-unknown-unknown`).

The firmware can't link without `avr-gcc`; `cargo clippy` there is enough.

Check the PR's own CI too: `gh pr checks <n>`.

Report anything that fails, with the output.

## 5. What to look for

Most important first:

1. **Coil and power safety**
   - Any change to `COIL_KINDS` in `crates/pinball-core/src/playfield.rs`:
     - raising a `max_ms` or lowering a `recycle_ms`
     - making a coil `Hold`
     - adding a coil

     Each must be called out in the PR, with the coil and its rating.
   - Anything that could leave a coil energised: the watchdog, the 74HC595 OE
     pull-up and enable order, and the panic handler that turns outputs off
     (`firmware/src/main.rs`). These must all stay in place.
   - Docs that could lead a beginner into an unsafe wiring step. The wiring
     guide is written for someone new to electronics.
2. **Correctness**
   - Logic bugs in the rules, debouncing or timing (`Instant` wrap-around).
   - Off-by-ones in switch, coil or lamp numbers.
   - Bit order in the output chain.
   - Phase transitions: attract, playing, tilted, game over.
3. **Keeping the playfield map in step.** `playfield.rs` is the source of truth.
   Check that any change to it is matched in:
   - `docs/playfield.svg`: its labels, and its `data-switch` / `data-coil` /
     `data-lamp` attributes
   - the tables in `docs/wiring-guide.md`
   - the quantities in `docs/shopping-list.md`
   - the part lists in the `crates/pinball-sim` tests. These lists are
     **written by hand**, not generated from `playfield.rs`, so check that new
     constants were added to them. The tests compare sets, so also check that
     each SVG element's switch, coil and lamp belong to the same part.
   - the pins in `firmware/src/main.rs` and `docs/hardware.md`

   Recount where you can.
4. **Architecture**
   - Game logic belongs in `crates/pinball-core`, which is `no_std`,
     allocation-free and `unsafe`-free. `firmware/` stays a thin I/O layer.
   - Simulator-only code stays in `crates/pinball-sim` / `sim/web`.
5. **Tests.** New behaviour has host tests in `pinball-core`, and the tests check
   behaviour, not just that the code runs.
6. **Honesty about guesses.** Anything not confirmed on the real table or
   schematic is marked *verify* or "assumed", in code and docs alike.
7. **The PR itself**
   - Follows the template: Summary, then Notes (assumptions, what was tested on
     real hardware versus only in tests, coil timing changes), then a GIF.
   - The GIF is relevant to the change and loads:
     `curl -sIL <url> | grep -iE '^HTTP|content-type'` shows `200` and
     `image/gif`.
   - One concern per PR.
   - Based on current `main`.
   - No stray files, such as `.DS_Store` or `sim/web/generated/`.
8. **Clarity.** Code and docs are understandable to a newcomer: names, comments
   where the *why* isn't obvious, and plain language in docs.

## 6. Verify before reporting

For each finding, either:

- show the exact lines, or
- describe a concrete scenario that goes wrong (inputs or state → wrong
  result), or reproduce it.

To reproduce a bug, put scratch code **outside the worktree**: for example, a
throwaway crate in another `mktemp -d` directory that depends on the worktree's
crates by path. Delete it afterwards. Writing scratch code there doesn't count
as editing the PR.

Drop anything you can't support. A short list of real problems beats a long list
of maybes. Don't report style preferences that `rustfmt` and clippy already
accept.

## 7. Report

Reply in this format:

```
## Review of #<n>: <title>

**Verdict:** Approve | Approve with nits | Changes needed

### Findings
1. **[blocker|should fix|nit]** `path/to/file.rs:123`: what's wrong.
   Scenario: how it goes wrong. Fix: what to change.
2. ...

### Checked and fine
- one line per area checked with nothing found (safety, map in step, tests, ...)

### Checks run
- cargo fmt / clippy / test / firmware clippy / sim build: pass or fail (with output on failure)
```

Order findings with the most severe first. If there are none, say so and keep
the "Checked and fine" list.
