#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
OUT="$ROOT/dist/release/linux"
mkdir -p "$OUT"
echo "[package-linux] draft — AppImage/.deb via Tauri targets → $OUT"
echo "See distribute/flatpak/ for Flathub stub."
