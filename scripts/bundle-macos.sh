#!/usr/bin/env bash
# Assembles maxx.app around an already-built binary.
#
# An icon does not attach to an executable: on macOS the bundle carries it,
# along with the name the Dock and the Apple menu display. Without one, `maxx`
# launched from a terminal is called "maxx" by luck — the binary's name — and
# has no icon at all.
#
#   scripts/bundle-macos.sh [path/to/binary] [output directory]
#
# Defaults: target/release/maxx, and the bundle beside it.

set -euo pipefail

binary="${1:-target/release/maxx}"
output="${2:-target/release}"
root="$(cd "$(dirname "$0")/.." && pwd)"

if [ ! -x "$binary" ]; then
  echo "binary not found: $binary (cargo build --release ?)" >&2
  exit 1
fi

version="$(sed -n 's/^version = "\(.*\)"/\1/p' "$root/Cargo.toml" | head -1)"
bundle="$output/maxx.app"

rm -rf "$bundle"
mkdir -p "$bundle/Contents/MacOS" "$bundle/Contents/Resources"
cp "$binary" "$bundle/Contents/MacOS/maxx"
cp "$root/assets/maxx.icns" "$bundle/Contents/Resources/maxx.icns"

cat > "$bundle/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>maxx</string>
  <key>CFBundleDisplayName</key><string>maxx</string>
  <key>CFBundleIdentifier</key><string>rs.maxx.app</string>
  <key>CFBundleExecutable</key><string>maxx</string>
  <key>CFBundleIconFile</key><string>maxx</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>${version}</string>
  <key>CFBundleVersion</key><string>${version}</string>
  <key>LSMinimumSystemVersion</key><string>11.0</string>
  <key>NSHighResolutionCapable</key><true/>
</dict>
</plist>
PLIST

# The Finder re-reads the bundle when its date changes; without this the icon
# can stay the one from an earlier assembly, which looks like nothing worked.
touch "$bundle"

echo "$bundle"
