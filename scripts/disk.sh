#!/usr/bin/env bash
# Fails when the disk under a path is too full, and says what is eating it.
set -euo pipefail

threshold=${1:-4G}
root=${2:-$(cd "$(dirname "$0")/.." && pwd)}
[ -d "$root" ] || {
  echo "disk: no directory at $root" >&2
  exit 2
}
root=$(cd "$root" && pwd)

size=${threshold%[GgMmKk]}
case $size in
  '' | *[!0-9]*)
    echo "disk: threshold is a size like 4G or 800M, not $threshold" >&2
    exit 2
    ;;
esac
case $threshold in
  *[Gg]) floor_kb=$((size * 1024 * 1024)) ;;
  *[Mm]) floor_kb=$((size * 1024)) ;;
  *[Kk]) floor_kb=$size ;;
  *) floor_kb=$((size * 1024 * 1024)) ;;
esac

free_kb=$(df -Pk "$root" | awk 'NR == 2 { print $4 }')
[ -n "$free_kb" ] || {
  echo "disk: df gave no free space for $root" >&2
  exit 2
}
free_mb=$((free_kb / 1024))
line="disk: $free_mb MB free under $root, floor $threshold"
echo "$line"
[ -z "${GITHUB_STEP_SUMMARY:-}" ] || echo "$line" >>"$GITHUB_STEP_SUMMARY"

[ "$free_kb" -lt "$floor_kb" ] || exit 0

# The ten largest directories one level down: the point is to name the hog.
listing=$(du -sm "$root"/*/ 2>/dev/null | sort -nr | head -10 | while IFS=$'\t' read -r mb dir; do
  printf '  %6s MB  %s\n' "$mb" "${dir%/}"
done)
echo
echo "the ten largest directories under $root:"
printf '%s\n' "$listing"
if [ -n "${GITHUB_STEP_SUMMARY:-}" ]; then
  {
    echo '```'
    printf '%s\n' "$listing"
    echo '```'
  } >>"$GITHUB_STEP_SUMMARY"
fi

echo "disk: under the $threshold floor; free space or build elsewhere" >&2
exit 1
