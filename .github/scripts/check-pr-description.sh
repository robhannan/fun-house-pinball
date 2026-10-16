#!/usr/bin/env bash
# Checks a pull request description against .github/pull_request_template.md
# and the PR rules in CLAUDE.md. Run by .github/workflows/pr-checks.yml.
#
# Inputs (environment):
#   PR_BODY   the PR description
#   BASE_SHA  base commit of the PR
#   HEAD_SHA  head commit of the PR
set -euo pipefail

body="${PR_BODY:-}"
failed=0
problem() {
  echo "::error::$1"
  failed=1
}

grep -q '^## Summary' <<<"$body" || problem "Missing a '## Summary' section (see .github/pull_request_template.md)."
grep -q '^## Notes' <<<"$body" || problem "Missing a '## Notes' section (see .github/pull_request_template.md)."
if grep -q '<!--' <<<"$body"; then
  problem "Template comments (<!-- ... -->) are still in the description; delete them."
fi

# The last image in the description is the GIF.
gif=$(grep -oE '!\[[^]]*\]\(https?://[^)[:space:]]+\)' <<<"$body" | tail -n 1 | sed -E 's/^.*\((.*)\)$/\1/' || true)
if [[ -z "$gif" ]]; then
  problem "No GIF at the bottom of the description (see .github/pull_request_template.md)."
else
  if grep -q 'xULW8hDrQXqSTkbMju' <<<"$gif"; then
    problem "The GIF is still the template's placeholder; replace it with one relevant to this change."
  fi
  headers=$(curl -sIL --max-time 20 "$gif" | tr -d '\r' || true)
  status=$(grep -E '^HTTP/' <<<"$headers" | tail -n 1 | awk '{print $2}' || true)
  type=$(grep -i '^content-type:' <<<"$headers" | tail -n 1 | awk '{print $2}' || true)
  if [[ "$status" != "200" || "$type" != image/* ]]; then
    problem "The GIF doesn't load as an image: $gif (HTTP ${status:-none}, ${type:-no content type})."
  fi
fi

# Coil safety (CLAUDE.md): any change to COIL_KINDS must be called out with
# the coil and its rating.
if [[ -n "${BASE_SHA:-}" && -n "${HEAD_SHA:-}" ]]; then
  if git diff "$BASE_SHA...$HEAD_SHA" -- crates/pinball-core/src/playfield.rs |
    grep -qE '^[+-][[:space:]]*k\[coil::'; then
    if ! grep -qi 'rating' <<<"$body"; then
      problem "COIL_KINDS changed, so the description must name each changed coil and its rating (CLAUDE.md, coil safety)."
    fi
  fi
fi

if [[ "$failed" -eq 0 ]]; then
  echo "PR description looks good."
fi
exit "$failed"
