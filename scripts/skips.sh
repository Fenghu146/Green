#!/usr/bin/env bash
# Surfaces what did not run: a test nobody sees is a test that does not exist.
set -euo pipefail

log=${1:?usage: skips.sh TEST_LOG}
[ -f "$log" ] || {
  echo "skips: no log at $log" >&2
  exit 2
}

# `#[ignore]` is a decision, not a hole: those run by hand with --ignored.
ignored=$(grep -iE '\.\.\. ignored' "$log" || true)
# `SKIP:` is this repository's own marker for a test a build could not run.
skipped=$(grep -inE '^SKIP|skipped:' "$log" || true)

if [ -z "$ignored" ] && [ -z "$skipped" ]; then
  echo "skips: none in $log"
  exit 0
fi

if [ -n "$ignored" ]; then
  echo "ignored by design (run with --ignored):"
  printf '%s\n' "$ignored"
fi
if [ -n "$skipped" ]; then
  echo "skipped:"
  printf '%s\n' "$skipped"
fi
if [ -n "${GITHUB_STEP_SUMMARY:-}" ]; then
  {
    echo "**not run** (\`$log\`)"
    echo '```'
    [ -z "$ignored" ] || printf 'ignored by design:\n%s\n' "$ignored"
    [ -z "$skipped" ] || printf 'skipped:\n%s\n' "$skipped"
    echo '```'
  } >>"$GITHUB_STEP_SUMMARY"
fi

# A `SKIP:` line is a hole, not a decision: `#[ignore]` is the decision.
# Exit explicitly on every path: a trailing grep's status must never decide
# whether the job is green (bash -e -o pipefail runs this script from CI).
if printf '%s\n' "$skipped" | grep -q 'GREEN_TYPST'; then
  echo "skips: a test skipped for GREEN_TYPST; install typst at Cargo.toml's version and export the path instead" >&2
  exit 1
fi
if [ -n "$skipped" ]; then
  echo "skips: $(printf '%s\n' "$skipped" | grep -c .) skipped test(s) are holes; see the list above" >&2
  exit 1
fi
exit 0
