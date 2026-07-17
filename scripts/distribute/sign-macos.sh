#!/usr/bin/env bash
# Sign macOS app bundle. Requires APPLE_CERTIFICATE (base64), APPLE_CERTIFICATE_PASSWORD, APPLE_TEAM_ID.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
APP="${1:-$ROOT/dist/release/darwin}"
ENTITLEMENTS="$ROOT/crates/tish_desktop/entitlements/app.entitlements"

if [[ -z "${APPLE_CERTIFICATE:-}" ]]; then
  echo "[sign-macos] APPLE_CERTIFICATE not set — skipping (draft)"
  exit 0
fi

TMP="$(mktemp -d)"
echo "$APPLE_CERTIFICATE" | base64 -d > "$TMP/cert.p12"
security create-keychain -p "" "$TMP/build.keychain"
security import "$TMP/cert.p12" -k "$TMP/build.keychain" -P "${APPLE_CERTIFICATE_PASSWORD:-}" -T /usr/bin/codesign
IDENTITY=$(security find-identity -v -p codesigning "$TMP/build.keychain" | head -1 | awk -F'"' '{print $2}')
echo "[sign-macos] identity=$IDENTITY"
find "$APP" -name "*.app" -print0 | while IFS= read -r -d '' bundle; do
  codesign --force --deep --options runtime --entitlements "$ENTITLEMENTS" --sign "$IDENTITY" "$bundle"
done
echo "[sign-macos] done"
