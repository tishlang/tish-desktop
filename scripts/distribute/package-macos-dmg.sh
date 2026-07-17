#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
SRC="${1:-$ROOT/dist/release/darwin}"
OUT="$ROOT/dist/release/darwin/TishDesktop.dmg"
echo "[package-macos-dmg] placeholder — use create-dmg or hdiutil on $SRC → $OUT"
mkdir -p "$(dirname "$OUT")"
# hdiutil create -volname TishDesktop -srcfolder "$SRC" -ov -format UDZO "$OUT"
echo "[package-macos-dmg] draft complete"
