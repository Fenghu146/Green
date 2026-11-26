#!/usr/bin/env bash
# vendor's crates.io crates: each release, patches/crates-io's patch on it, and
# the vendor workspace manifest.
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
mode=${1:-check}
case $mode in
  check | write) ;;
  *)
    echo "usage: vendor-crates.sh [check|write]" >&2
    exit 2
    ;;
esac

# Each crate at the version the lock resolves, with its tarball's sha256.
crates="accesskit_atspi_common 0.19.1 023da0e5097f46df7092d5280b02efb9bbf8d93298daeced42652463e357d636
accesskit_unix 0.22.1 03e156ed3802e35eefe894ef2671bc6c889303d8a7e110b5e1b48f504b91362f
mail-parser 0.11.9 4ec00bda90c6e645a54506c630c2820cd6b1890cfd2b0a169b50f74b2b8c7c86
pulldown-cmark 0.13.6 3db2be46835f013dd3f4fa7f5f7402c537787b54ebb6fe2b94bbdd7e2669bb7b
zbus 5.19.0 5db4be7c075cb421e4b7ee645541604239bd243ba7c357511f4ff3a74b555907"

# The four gpui crates come from vendor-gpui.sh; the manifest lists all nine.
members="accesskit_atspi_common accesskit_unix gpui-pre gpui-pre-linux gpui-pre-macos gpui-pre-windows mail-parser pulldown-cmark zbus"

sha256() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | cut -d' ' -f1
  else
    shasum -a 256 "$1" | cut -d' ' -f1
  fi
}

patches=$(ls "$root"/patches/crates-io/*.patch 2>/dev/null || true)
[ -n "$patches" ] || {
  echo "vendor-crates: patches/crates-io holds no patch" >&2
  exit 1
}
# Every patch names its crate and version in the file name and on its first line.
while read -r name version _; do
  found=
  for patch in $patches; do
    case ${patch##*/} in
      "$name-$version-"*) found=$patch ;;
    esac
  done
  [ -n "$found" ] || {
    echo "vendor-crates: no patch for $name $version" >&2
    exit 1
  }
  head -n 1 "$found" | grep -qF "$name $version" || {
    echo "vendor-crates: ${found##*/} does not name $name $version as its baseline" >&2
    exit 1
  }
done <<<"$crates"

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
mkdir "$work/vendor"
# Inside another repository git apply skips every path.
git init -q "$work"
git -C "$work" config core.autocrlf false
while read -r name version sum; do
  crate=$work/$name.crate
  curl -fsSL -o "$crate" "https://static.crates.io/crates/$name/$name-$version.crate"
  got=$(sha256 "$crate")
  if [ "$got" != "$sum" ]; then
    echo "vendor-crates: $name $version hashes to $got, not the pin's $sum; a version bump needs a new hash and a rebased patch" >&2
    exit 1
  fi
  mkdir -p "$work/vendor/$name"
  tar -xzf "$crate" -C "$work/vendor/$name" --strip-components=1
  # Publish metadata, the crate's own lock and its CI are not source we vendor.
  rm -f "$work/vendor/$name/.cargo_vcs_info.json" "$work/vendor/$name/Cargo.lock"
  rm -rf "$work/vendor/$name/.github"
done <<<"$crates"
for patch in $patches; do
  (cd "$work" && git apply "$patch") || {
    echo "vendor-crates: ${patch##*/} does not apply" >&2
    exit 1
  }
done

{
  echo "# Tested apart: dev-dependencies stay out of the apps' lock."
  echo "[workspace]"
  echo 'resolver = "3"'
  echo -n 'members = ['
  sep=
  for name in $members; do
    echo -n "$sep\"$name\""
    sep=', '
  done
  echo "]"
  echo
  echo "[patch.crates-io]"
  for name in $members; do
    echo "$name = { path = \"$name\" }"
  done
} >"$work/vendor/Cargo.toml"

if [ "$mode" = write ]; then
  mkdir -p "$root/vendor"
  while read -r name _; do
    rm -rf "$root/vendor/$name"
    cp -R "$work/vendor/$name" "$root/vendor/$name"
  done <<<"$crates"
  cp "$work/vendor/Cargo.toml" "$root/vendor/Cargo.toml"
else
  failed=0
  while read -r name _; do
    diff -r --brief "$work/vendor/$name" "$root/vendor/$name" || failed=1
  done <<<"$crates"
  diff --brief "$work/vendor/Cargo.toml" "$root/vendor/Cargo.toml" || failed=1
  [ "$failed" = 0 ] || {
    echo "vendor-crates: vendor/ is not these releases plus patches/crates-io" >&2
    exit 1
  }
fi
echo "vendor-crates: $mode: $(printf '%s\n' "$crates" | grep -c .) crates plus $(printf '%s\n' "$patches" | grep -c .) patches"
