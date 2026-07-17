#!/usr/bin/env bash
# Notarize with notarytool. Prefers App Store Connect API key env vars.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
TARGET="${1:-$ROOT/dist/release/darwin}"

if [[ -z "${APP_STORE_CONNECT_API_KEY_ID:-}" ]]; then
  echo "[notarize-macos] App Store Connect API key not set — skipping (draft)"
  exit 0
fi

KEYDIR="$(mktemp -d)"
echo "${APP_STORE_CONNECT_API_KEY_P8}" > "$KEYDIR/AuthKey_${APP_STORE_CONNECT_API_KEY_ID}.p8"
xcrun notarytool submit "$TARGET" \
  --key "$KEYDIR/AuthKey_${APP_STORE_CONNECT_API_KEY_ID}.p8" \
  --key-id "$APP_STORE_CONNECT_API_KEY_ID" \
  --issuer "$APP_STORE_CONNECT_ISSUER_ID" \
  --wait
echo "[notarize-macos] staple if .app/.dmg present"
find "$TARGET" \( -name "*.app" -o -name "*.dmg" \) -print0 | while IFS= read -r -d '' f; do
  xcrun stapler staple "$f" || true
done
