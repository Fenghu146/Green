#!/usr/bin/env bash
# Surfaces skipped tests: one that skips quietly is one that does not exist.
set -euo pipefail

log=${1:?usage: skips.sh TEST_LOG}
[ -f "$log" ] || {
  echo "skips: no log at $log" >&2
  exit 2
}

# `SKIP:` is this repository's own marker; `... ignored` is the test harness's.
skips=$(grep -inE '^SKIP|skipped:|\.\.\. ignored' "$log" || true)
if [ -z "$skips" ]; then
  echo "skips: none in $log"
  exit 0
fi

echo "skipped tests:"
printf '%s\n' "$skips"
if [ -n "${GITHUB_STEP_SUMMARY:-}" ]; then
  {
    echo "**skipped tests** (\`$log\`)"
    echo '```'
    printf '%s\n' "$skips"
    echo '```'
  } >>"$GITHUB_STEP_SUMMARY"
fi

# The conversion test only proves anything when it compiles what it converts.
if printf '%s\n' "$skips" | grep -q 'GREEN_TYPST'; then
  echo "skips: a test skipped for GREEN_TYPST; install typst at Cargo.toml's version and export the path instead" >&2
  exit 1
fi
