#!/usr/bin/env bash
# vendor's gpui is crates.io's release plus patches/gpui, in order.
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
mode=${1:-check}
case $mode in
  check | write) ;;
  *)
    echo "usage: vendor-gpui.sh [check|write]" >&2
    exit 2
    ;;
esac

# The version lives in Cargo.toml alone; a bump lands here as a hash mismatch.
version=$(sed -n 's/^gpui = { package = "gpui-pre", version = "=\([^"]*\)".*/\1/p' "$root/Cargo.toml")
[ -n "$version" ] || {
  echo "vendor-gpui: Cargo.toml pins no gpui-pre version" >&2
  exit 1
}

# Each crate with the hash crates.io's index gives it, at that version.
crates="gpui-pre ab22c23c1304a0b3f34f64568f59a4ce2abbfab3727273a90c66b37924d31089
gpui-pre-linux 9515d52d8378c5dda95b3b2536d3d05ea2431bc25d1d8bb4cdeb32f21090f4a1
gpui-pre-macos 23eeb64494000782e17b0ae0ec2c402ba7d76e5918a6af4759e60094fc8d6dcf
gpui-pre-windows 8badbaa4129a88e4d2fdaa1615fc61b2a59d4ebb2ac36e5de51f75598f0dd992"

sha256() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | cut -d' ' -f1
  else
    shasum -a 256 "$1" | cut -d' ' -f1
  fi
}

# A series that is missing is not a series that applies; say which.
patches=$(ls "$root"/patches/gpui/*.patch 2>/dev/null || true)
[ -n "$patches" ] || {
  echo "vendor-gpui: patches/gpui holds no patch" >&2
  exit 1
}
for patch in $patches; do
  head -n 1 "$patch" | grep -qF "gpui-pre =$version" || {
    echo "vendor-gpui: ${patch##*/} does not name gpui-pre =$version as its baseline" >&2
    exit 1
  }
done

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
mkdir "$work/vendor"
# Inside another repository git apply skips every path.
git init -q "$work"
# Patched files keep upstream's line endings.
git -C "$work" config core.autocrlf false
while read -r name sum; do
  crate=$work/$name.crate
  curl -fsSL -o "$crate" "https://static.crates.io/crates/$name/$name-$version.crate"
  got=$(sha256 "$crate")
  if [ "$got" != "$sum" ]; then
    echo "vendor-gpui: $name $version hashes to $got, not the pin's $sum; a version bump needs new hashes" >&2
    exit 1
  fi
  tar -xzf "$crate" -C "$work/vendor"
  mv "$work/vendor/$name-$version" "$work/vendor/$name"
done <<<"$crates"
for patch in $patches; do
  (cd "$work" && git apply "$patch") || {
    echo "vendor-gpui: ${patch##*/} does not apply to $version" >&2
    exit 1
  }
done

failed=0
while read -r name _; do
  if [ "$mode" = write ]; then
    mkdir -p "$root/vendor"
    rm -rf "$root/vendor/$name"
    cp -R "$work/vendor/$name" "$root/vendor/$name"
  elif ! diff -r --brief "$work/vendor/$name" "$root/vendor/$name"; then
    echo "vendor-gpui: vendor/$name is not $version plus patches/gpui" >&2
    failed=1
  fi
done <<<"$crates"
[ "$failed" = 0 ] || exit 1
count=$(printf '%s\n' "$patches" | wc -l | tr -d ' ')
echo "vendor-gpui: $mode: four crates at $version plus $count patches"
