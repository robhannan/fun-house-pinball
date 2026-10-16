# fun-house-pinball

Rust controller for a restored 1956 Williams *Fun House* (IPDB #967): an
electromechanical (EM) table rebuilt with new parts and driven by an Arduino
Mega 2560. See `README.md` for setup and `docs/hardware.md` for wiring.

## Project rules

- **Game logic goes in `crates/pinball-core`:** it's `no_std`, allocation-free
  and host-tested with `cargo test`. `firmware/` only does I/O and the main loop,
  so keep it thin.
- **`firmware/` is a separate cargo project** (excluded from the workspace) with
  its own pinned nightly in `firmware/rust-toolchain.toml`. Run its commands from
  inside `firmware/`. When bumping avr-hal, bump that toolchain to match avr-hal's
  own `rust-toolchain.toml`.
- **Coil safety:** never raise a `max_ms` or lower a `recycle_ms` in `COIL_KINDS`
  (`playfield.rs`) without saying so in the PR, with the coil and its rating. Keep
  the watchdog, the OE pull-up and the panic handler that disables outputs.
- **Keep the playfield map in step:** `playfield.rs` is the source of truth for
  switch, coil and lamp assignments. Update `docs/hardware.md`,
  `docs/wiring-guide.md`, `docs/playfield.svg`, the quantities in
  `docs/shopping-list.md` and the pins in
  `firmware/src/main.rs` alongside it.
- **Parts list:** add every new part to `PARTS` in `playfield.rs`, as well as
  to the `sw`/`coil`/`lamp` modules. Tests check it against `COIL_KINDS`, and
  other checks build on it.
- **Scripts are written in zx, not bash,** wherever possible: `.mjs` files that
  `import { $ } from "zx"`, with zx pinned in `package.json`. Use **pnpm**,
  not npm: `pnpm install`, then `pnpm run <script>`. Keep workflow `run:`
  steps to a single `pnpm run` call. Shell one-liners in docs and skills are fine.
- **Turn failures into checks:** when a review, CI or the real machine finds a
  failure in the code or build, add a test or CI check that would catch it next
  time, wherever that's possible. Process rules, such as the PR description and
  branch hygiene, are handled by the `github-pr` skill, not CI.
- **Don't present guesses as facts:** feature counts and rules not confirmed on
  the real table or schematic are marked *verify* or "assumed". Keep them marked.

## Pull requests

Use the `github-pr` skill (`.claude/skills/github-pr/SKILL.md`) for the full
steps. The essentials:

1. **Check before every push**, not just the first one: run `git fetch origin`
   and look up the branch's PR state. If the PR is merged or closed, never push
   to it or edit its description. Start a new branch from `origin/main`.
2. **One concern per branch**, each cut from `origin/main`. Don't commit to `main`.
   Stage files by name (not `git add -A`) and check `git status --short` before
   each commit.
3. **Run CI's checks first:** `cargo fmt --all --check`,
   `cargo clippy --all-targets --locked -- -D warnings` and `cargo test --locked`,
   plus `cargo fmt --check` and `cargo clippy --locked -- -D warnings` in
   `firmware/`.
4. **Write the description from `.github/pull_request_template.md`:**
   - Summary, then Notes: assumptions, what was tested on the real machine
     versus only in tests, and any coil timing changes.
   - Delete the template's comments.
   - Pass the body with `gh pr create --body-file`.
5. **Replace the template's placeholder GIF with one relevant to the change.**
   Check that its link loads as a GIF and, if possible, view it before using it.
   If none can be verified, keep the pinball placeholder.
6. **Watch CI** (`gh pr checks <n> --watch`) and report the PR link and the
   result.
7. **Get a fresh-context review** with the `review-pr` skill
   (`/review-pr <n>`). It runs in a separate subagent that hasn't seen the
   conversation that wrote the code. Relay its findings; don't post them to
   GitHub unless asked.

If `gh` acts as the wrong account, a `GITHUB_TOKEN` environment variable may be
overriding the stored login. Check with `gh auth status`.
