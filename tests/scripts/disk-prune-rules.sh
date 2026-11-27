#!/usr/bin/env bash
# disk-prune must stay inside Green's own root, and a top-level profile must
# never be an automatic candidate.
set -euo pipefail

here=$(cd "$(dirname "$0")/../.." && pwd)
prune="$here/scripts/disk-prune"

default=$(sed -n 's/^root=${DISK_PRUNE_ROOT:-\([^}]*\)}.*/\1/p' "$prune" | head -1)
if [ "$default" != /d/green-targets ]; then
  echo "FAIL: default root is '$default', want /d/green-targets" >&2
  exit 1
fi
for bad in /d/cargo-target /d/Amp/targets; do
  if grep -q "DISK_PRUNE_ROOT:-$bad" "$prune"; then
    echo "FAIL: $bad appears as a default root" >&2
    exit 1
  fi
done

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
mkdir -p "$tmp/debug"
touch -d '1 hour ago' "$tmp/debug"

out=$(DISK_PRUNE_ROOT="$tmp" bash "$prune" 2>&1)
if printf '%s\n' "$out" | grep -Eq '^debug[[:space:]].*DELETE'; then
  echo "FAIL: top-level debug was an automatic candidate" >&2
  printf '%s\n' "$out" >&2
  exit 1
fi

out=$(DISK_PRUNE_ROOT="$tmp" DISK_PRUNE_ONLY=debug bash "$prune" 2>&1)
if ! printf '%s\n' "$out" | grep -Eq '^debug[[:space:]].*DELETE'; then
  echo "FAIL: an explicit DISK_PRUNE_ONLY=debug did not consider debug" >&2
  printf '%s\n' "$out" >&2
  exit 1
fi

echo 'disk-prune: default root isolated; top-level debug needs an explicit name'
