#!/usr/bin/env bash
# Regression gate for skips.sh: #[ignore] is a decision, a SKIP: line is a hole.
# Run the same two cases CI relies on, with the same bash flags CI uses.
set -uo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

{
  for n in 1 2 3 4 5 6; do
    printf 'test bench_%s ... ignored\n' "$n"
  done
} >"$tmp/ignored.log"
printf 'test a_test ... ok\nSKIP: a test the build could not run\n' >"$tmp/skip.log"

failed=0
check() { # expected exit code, log, label
  local want=$1 log=$2 label=$3 got
  bash -e -o pipefail "$root/scripts/skips.sh" "$log" >"$tmp/out" 2>&1
  got=$?
  if [ "$got" -eq "$want" ]; then
    printf 'skips self-test: %-14s -> %s (expected %s) PASS\n' "$label" "$got" "$want"
  else
    printf 'skips self-test: %-14s -> %s (expected %s) FAIL\n' "$label" "$got" "$want" >&2
    sed -n '1,12p' "$tmp/out" >&2
    failed=1
  fi
}

check 0 "$tmp/ignored.log" 'ignored only'
check 1 "$tmp/skip.log" 'SKIP: marker'
exit "$failed"
