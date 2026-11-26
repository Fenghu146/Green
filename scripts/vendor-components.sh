#!/usr/bin/env bash
# vendor's component library is the pinned commit, its own repo's extras pruned,
# plus patches/ely-gpui-components.
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
mode=${1:-check}
case $mode in
  check | write) ;;
  *)
    echo "usage: vendor-components.sh [check|write]" >&2
    exit 2
    ;;
esac

# The pin lives in Cargo.toml alone.
line=$(grep '^ely-gpui-component = { git = ' "$root/Cargo.toml" || true)
[ -n "$line" ] || {
  echo "vendor-components: Cargo.toml pins no ely-gpui-component" >&2
  exit 1
}
url=$(sed -n 's/.*git = "\([^"]*\)".*/\1/p' <<<"$line")
rev=$(sed -n 's/.*rev = "\([^"]*\)".*/\1/p' <<<"$line")
[ -n "$url" ] && [ -n "$rev" ] || {
  echo "vendor-components: the ely-gpui-component pin needs a git URL and a rev" >&2
  exit 1
}

prune=$root/patches/ely-gpui-components/fc71e96-prune.txt
[ -f "$prune" ] || {
  echo "vendor-components: no prune list at patches/ely-gpui-components/fc71e96-prune.txt" >&2
  exit 1
}
patches=$(ls "$root"/patches/ely-gpui-components/*.patch 2>/dev/null || true)
for patch in $patches; do
  head -n 1 "$patch" | grep -qF "$rev" || {
    echo "vendor-components: ${patch##*/} does not name $rev as its baseline" >&2
    exit 1
  }
done

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
name=ely-gpui-components
repo=$work/checkout
stage=$work/stage
git init -q "$repo"
git -C "$repo" config core.autocrlf false
git -C "$repo" remote add origin "$url"
# One commit is all this needs; a server that will not hand over a bare sha gets
# the history instead.
git -C "$repo" fetch -q --depth 1 origin "$rev" ||
  git -C "$repo" fetch -q origin "$rev"
git -C "$repo" checkout -q FETCH_HEAD
rm -rf "$repo/.git"
mkdir -p "$stage/vendor"
mv "$repo" "$stage/vendor/$name"
# git apply wants a repository; the prune and the patches happen inside it.
git init -q "$stage"
git -C "$stage" config core.autocrlf false
while read -r path; do
  [ -n "$path" ] || continue
  case $path in
    /* | ../* | */../*)
      echo "vendor-components: prune list has a path outside the tree: $path" >&2
      exit 1
      ;;
  esac
  rm -rf "$stage/vendor/$name/$path"
done <"$prune"
# Git tracks no empty directory, so a pruned tree drops the ones left behind.
find "$stage/vendor/$name" -depth -type d -empty -delete
for patch in $patches; do
  (cd "$stage" && git apply "$patch") || {
    echo "vendor-components: ${patch##*/} does not apply to ${rev:0:7}" >&2
    exit 1
  }
done

if [ "$mode" = write ]; then
  rm -rf "$root/vendor/$name"
  cp -R "$stage/vendor/$name" "$root/vendor/$name"
elif ! diff -r --brief "$stage/vendor/$name" "$root/vendor/$name"; then
  echo "vendor-components: vendor/$name is not ${rev:0:7} plus patches" >&2
  exit 1
fi
count=$(printf '%s\n' "$patches" | grep -c . || true)
echo "vendor-components: $mode: ${rev:0:7} pruned by $(grep -c . "$prune") paths, $count patches"
