#!/usr/bin/env bash
# Two lines on the disk under a path: a target to warn below, a hard floor to fail below.
#
#   scripts/disk.sh [hard_floor] [target] [root]      defaults: 30G 50G <repo>
#
# This drive is 451 GB. Our own footprint is 133 GB; the other 298 GB belongs to
# other projects and is not ours to delete. Multi-line work keeps a hot target dir
# per line (D:\cargo-target\<line>), and several of those building at once settle
# between 30 and 50 GB free. That 30-50 GB is the sustainable water level on this
# machine, not 100 GB: reaching 100 GB would mean deleting other people's data.
# So the defaults fail below 30 GB and only WARN below 50 GB. CI passes smaller
# numbers because a runner's disk is not this machine's disk.
set -euo pipefail

hard=${1:-30G}
target=${2:-50G}
root=${3:-$(cd "$(dirname "$0")/.." && pwd)}
[ -d "$root" ] || {
  echo "disk: no directory at $root" >&2
  exit 2
}
root=$(cd "$root" && pwd)

kb_of() {
  local size=${1%[GgMmKk]}
  case $size in
    '' | *[!0-9]*)
      echo "disk: $1 is not a size like 30G or 800M" >&2
      return 1
      ;;
  esac
  case $1 in
    *[Gg]) echo $((size * 1024 * 1024)) ;;
    *[Mm]) echo $((size * 1024)) ;;
    *[Kk]) echo "$size" ;;
    *) echo $((size * 1024 * 1024)) ;;
  esac
}

hard_kb=$(kb_of "$hard") || exit 2
target_kb=$(kb_of "$target") || exit 2
free_kb=$(df -Pk "$root" | awk 'NR == 2 { print $4 }')
[ -n "$free_kb" ] || {
  echo "disk: df gave no free space for $root" >&2
  exit 2
}

free_gb=$(awk -v k="$free_kb" 'BEGIN { printf "%.1f", k / 1048576 }')
line="disk: ${free_gb} GB free under $root; warn below $target, fail below $hard"
echo "$line"
[ -z "${GITHUB_STEP_SUMMARY:-}" ] || echo "$line" >>"$GITHUB_STEP_SUMMARY"

if [ "$free_kb" -lt "$target_kb" ]; then
  warn="disk: below the $target target ($free_gb GB free): collect build output soon, scripts/disk-prune says what can go"
  echo "$warn" >&2
  [ -z "${GITHUB_ACTIONS:-}" ] || echo "::warning::$warn"
fi
[ "$free_kb" -lt "$hard_kb" ] || exit 0

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

echo "disk: under the $hard floor; free space or build elsewhere (scripts/disk-prune lists what can go)" >&2
exit 1
