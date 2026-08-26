#!/usr/bin/env bash
# Assemble maxx.app autour d'un binaire déjà construit.
#
# Une icône ne s'attache pas à un exécutable : sur macOS, c'est le paquet qui la
# porte, avec le nom que le Dock et le menu Pomme affichent. Sans ça, `maxx`
# lancé depuis un terminal s'appelle « maxx » par chance — le nom du binaire —
# et n'a pas d'icône du tout.
#
#   scripts/bundle-macos.sh [chemin/du/binaire] [répertoire de sortie]
#
# Par défaut : target/release/maxx, et le paquet à côté de lui.

set -euo pipefail

binaire="${1:-target/release/maxx}"
sortie="${2:-target/release}"
racine="$(cd "$(dirname "$0")/.." && pwd)"

if [ ! -x "$binaire" ]; then
  echo "binaire introuvable : $binaire (cargo build --release ?)" >&2
  exit 1
fi

version="$(sed -n 's/^version = "\(.*\)"/\1/p' "$racine/Cargo.toml" | head -1)"
paquet="$sortie/maxx.app"

rm -rf "$paquet"
mkdir -p "$paquet/Contents/MacOS" "$paquet/Contents/Resources"
cp "$binaire" "$paquet/Contents/MacOS/maxx"
cp "$racine/assets/maxx.icns" "$paquet/Contents/Resources/maxx.icns"

cat > "$paquet/Contents/Info.plist" <<PLIST
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

# Le Finder relit le paquet quand sa date change ; sans ça l'icône peut rester
# celle d'un assemblage précédent, ce qui donne à croire que rien n'a marché.
touch "$paquet"

echo "$paquet"
