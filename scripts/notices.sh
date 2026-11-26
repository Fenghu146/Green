#!/usr/bin/env bash
# Regenerates THIRD-PARTY-NOTICES.txt with cargo-about; CI reruns it and diffs.
set -euo pipefail

root=${1:-$(cd "$(dirname "$0")/.." && pwd)}
cd "$root"
cargo about generate --config about.toml about.hbs -o THIRD-PARTY-NOTICES.txt
# The same bytes on every platform: cargo-about writes CRLF on Windows, and a
# platform-dependent file would make the CI diff fail for no reason.
tr -d '\r' <THIRD-PARTY-NOTICES.txt >THIRD-PARTY-NOTICES.txt.lf
mv THIRD-PARTY-NOTICES.txt.lf THIRD-PARTY-NOTICES.txt
echo "notices: wrote THIRD-PARTY-NOTICES.txt"
