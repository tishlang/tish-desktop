#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
OUT="$ROOT/dist/release/macos-mas"
mkdir -p "$OUT"
echo "[package-macos-appstore] draft — produce .pkg for Transporter into $OUT"
echo "Use productbuild / xcodebuild archive with MAS entitlements when ready."
