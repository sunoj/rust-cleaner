#!/usr/bin/env bash
# Builds, signs, optionally notarizes, and publishes one Sparkle release of WD-40.
# Usage: UPLOAD_SECRET=... ./scripts/release.sh <version> [release notes].

set -euo pipefail

VERSION="${1:?usage: release.sh <version> [notes]}"
NOTES="${2:-Bug fixes and improvements.}"
: "${UPLOAD_SECRET:?set UPLOAD_SECRET for the WD-40 release Worker}"

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT_DIR"

RELAY="https://wd40-release.sunoj-mings.workers.dev"
SIGN_UPDATE=".sparkle/bin/sign_update"
MIN_OS="13.0"
BUILD="$(date +%Y%m%d%H%M)"
OUT_DIR="dist"
ZIP="wd40-$VERSION.zip"
APP="dist/WD-40.app"

CARGO_VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)"
if [[ "$CARGO_VERSION" != "$VERSION" ]]; then
  printf 'ERROR: Cargo.toml is at %s but you asked to release %s\n' "$CARGO_VERSION" "$VERSION" >&2
  exit 1
fi

WD40_VERSION="$VERSION" WD40_BUILD="$BUILD" ./scripts/bundle.sh >/dev/null

if [[ -n "${NOTARY_PROFILE:-}" && -n "${SIGN_IDENTITY:-}" ]]; then
  /usr/bin/ditto -c -k --keepParent "$APP" "$OUT_DIR/notarize-$VERSION.zip"
  printf 'Submitting to Apple notary (profile: %s)…\n' "$NOTARY_PROFILE"
  xcrun notarytool submit "$OUT_DIR/notarize-$VERSION.zip" \
    --keychain-profile "$NOTARY_PROFILE" --wait
  xcrun stapler staple "$APP"
  printf 'Notarized and stapled.\n'
elif [[ -n "${SIGN_IDENTITY:-}" ]]; then
  printf 'NOTE: Developer ID signed but not notarized; set NOTARY_PROFILE to notarize.\n'
else
  printf 'NOTE: Ad-hoc signed; Gatekeeper may warn on first launch.\n'
fi

/usr/bin/ditto -c -k --keepParent "$APP" "$OUT_DIR/$ZIP"
SIG_LINE="$("$SIGN_UPDATE" "$OUT_DIR/$ZIP")"
LENGTH="$(printf '%s' "$SIG_LINE" | sed -n 's/.*length="\([0-9]*\)".*/\1/p')"

# A "]]>" inside the notes would close the CDATA section early and let the rest
# of the text be parsed as markup. Split the terminator across two sections.
NOTES_XML="${NOTES//]]>/]]]]><![CDATA[>}"

cat > "$OUT_DIR/appcast.xml" <<XML
<?xml version="1.0" encoding="utf-8"?>
<rss version="2.0" xmlns:sparkle="http://www.andymatuschak.org/xml-namespaces/sparkle">
  <channel>
    <title>WD-40</title>
    <item>
      <title>Version $VERSION</title>
      <sparkle:shortVersionString>$VERSION</sparkle:shortVersionString>
      <sparkle:version>$BUILD</sparkle:version>
      <sparkle:minimumSystemVersion>$MIN_OS</sparkle:minimumSystemVersion>
      <description><![CDATA[ $NOTES_XML ]]></description>
      <enclosure url="$RELAY/$ZIP" $SIG_LINE type="application/zip" />
    </item>
  </channel>
</rss>
XML

# Never publish a feed that Sparkle cannot parse: a broken appcast breaks
# updates for every installed copy, and the upload is not transactional.
if ! xmllint --noout "$OUT_DIR/appcast.xml" 2>/dev/null; then
  printf 'ERROR: generated appcast.xml is not well-formed XML; nothing uploaded\n' >&2
  exit 1
fi

curl -fsS -X PUT "$RELAY/$ZIP" \
  -H "authorization: Bearer $UPLOAD_SECRET" \
  -H "content-type: application/zip" \
  --data-binary "@$OUT_DIR/$ZIP" >/dev/null
curl -fsS -X PUT "$RELAY/appcast.xml" \
  -H "authorization: Bearer $UPLOAD_SECRET" \
  -H "content-type: application/xml" \
  --data-binary "@$OUT_DIR/appcast.xml" >/dev/null

printf 'Published WD-40 %s (build %s, %s bytes)\n' "$VERSION" "$BUILD" "$LENGTH"
printf '  feed: %s/appcast.xml\n  zip:  %s/%s\n' "$RELAY" "$RELAY" "$ZIP"
