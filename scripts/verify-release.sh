#!/usr/bin/env bash
# Proves a release's artifacts are what Gatekeeper will accept, before they are
# published — and afterwards, against whatever the relay is actually serving.
# Usage: ./scripts/verify-release.sh <zip> <dmg>; deps: codesign, spctl, stapler.

set -euo pipefail

ZIP="${1:?usage: verify-release.sh <zip> <dmg>}"
DMG="${2:?usage: verify-release.sh <zip> <dmg>}"

WORK="$(mktemp -d)"
MOUNT="$WORK/mnt"
FAILURES=0

# Unconditional and forgiving: an attach that failed still leaves the mountpoint
# behind, and a detach that fails must not stop the work directory being removed.
cleanup() {
  /usr/bin/hdiutil detach "$MOUNT" -quiet >/dev/null 2>&1 || true
  rm -rf "$WORK"
}
trap cleanup EXIT

fail() {
  printf '  FAIL  %s\n' "$1" >&2
  FAILURES=$((FAILURES + 1))
}

pass() {
  printf '  ok    %s\n' "$1"
}

# Gatekeeper's verdict on the app a person will actually launch. An ad-hoc or
# merely Developer ID signed build is rejected here; only a notarized one says
# "source=Notarized Developer ID", which is the string worth matching — plain
# "accepted" also comes back for a build notarization has never seen.
check_app() {
  local app="$1" origin="$2"
  if codesign --verify --deep --strict "$app" 2>/dev/null; then
    pass "$origin: signature is whole"
  else
    fail "$origin: codesign --verify --deep --strict rejected it"
  fi

  local assessment
  assessment="$(spctl --assess --type exec -vv "$app" 2>&1 || true)"
  case "$assessment" in
    *"source=Notarized Developer ID"*) pass "$origin: Gatekeeper accepts it as notarized" ;;
    *) fail "$origin: spctl says ${assessment##*$'\n'}" ;;
  esac

  # Stapled, so the first launch does not need to reach Apple. A notarized but
  # unstapled app fails closed on a machine that is offline.
  if xcrun stapler validate "$app" >/dev/null 2>&1; then
    pass "$origin: notarization ticket is stapled"
  else
    fail "$origin: no stapled ticket"
  fi
}

# Exactly one, not merely a first match: checking one app and shipping another
# beside it is the shape of hole this whole script exists to close.
only_app() {
  local root="$1" label="$2" apps
  apps="$(find "$root" -maxdepth 1 -name '*.app')"
  if [ "$(printf '%s\n' "$apps" | grep -c .)" != "1" ]; then
    printf '  FAIL  %s does not hold exactly one .app:\n%s\n' "$label" "${apps:-  (none)}" >&2
    exit 1
  fi
  printf '%s' "$apps"
}

printf 'Verifying %s\n' "$ZIP"
/usr/bin/ditto -x -k "$ZIP" "$WORK/zip"
ZIP_APP="$(only_app "$WORK/zip" "the zip")"
check_app "$ZIP_APP" "app from zip"

printf 'Verifying %s\n' "$DMG"
# The disk image is its own signed object: people download it, so Gatekeeper
# judges it before it ever judges the app inside.
if codesign --verify --strict "$DMG" 2>/dev/null; then
  pass "dmg image: signature is whole"
else
  fail "dmg image: codesign --verify --strict rejected it"
fi

dmg_assessment="$(spctl --assess --type open --context context:primary-signature -vv "$DMG" 2>&1 || true)"
case "$dmg_assessment" in
  *"source=Notarized Developer ID"*) pass "dmg image: Gatekeeper accepts it as notarized" ;;
  *) fail "dmg image: spctl says ${dmg_assessment##*$'\n'}" ;;
esac

if xcrun stapler validate "$DMG" >/dev/null 2>&1; then
  pass "dmg image: notarization ticket is stapled"
else
  fail "dmg image: no stapled ticket"
fi

mkdir -p "$MOUNT"
/usr/bin/hdiutil attach "$DMG" -nobrowse -readonly -mountpoint "$MOUNT" -quiet
DMG_APP="$(only_app "$MOUNT" "the dmg")"
check_app "$DMG_APP" "app in dmg"

if [ "$FAILURES" -ne 0 ]; then
  printf '\n%d check(s) failed; these artifacts are not fit to publish.\n' "$FAILURES" >&2
  exit 1
fi
printf '\nAll checks passed.\n'
