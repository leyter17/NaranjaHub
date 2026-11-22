#!/bin/bash
# NaranjaHub — construcción de todos los entregables portables.
# Copyright (C) 2026 Leyter Vázquez. V — SPDX-License-Identifier: GPL-3.0-only
#
# Genera en download/:
#   NaranjaHub-<v>-x86_64.AppImage
#   naranjahub_<v>_amd64.deb
#   NaranjaHub-<v>-x86_64.run
#   NaranjaHub-<v>-codigo-fuente.zip
#   readme.txt + SHA256SUMS.txt
#
# Uso: scripts/build_release.sh

set -eu

VERSION="$(grep -m1 '^version' /home/z/my-project/naranjahub/Cargo.toml | sed 's/.*"\(.*\)".*/\1/')"
PROJ=/home/z/my-project/naranjahub
DIST="$PROJ/dist"
OUT=/home/z/my-project/download
TOOLS="$PROJ/.tools/squashfs-root/usr/bin"
ARCH=x86_64

export PATH="$HOME/.cargo/bin:$PATH"
# appimagetool busca mksquashfs en el PATH
export PATH="$TOOLS:$PATH"

echo "== NaranjaHub v$VERSION =="

# 1) Binario release
cd "$PROJ"
cargo build --release
BIN="$PROJ/target/release/naranjahub"

rm -rf "$DIST"
mkdir -p "$OUT"

# 2) AppImage
APPDIR="$DIST/AppDir"
mkdir -p "$APPDIR/usr/bin" "$APPDIR/usr/share/doc/naranjahub" \
         "$APPDIR/usr/share/icons/hicolor/256x256/apps" \
         "$APPDIR/usr/share/icons/hicolor/128x128/apps" \
         "$APPDIR/usr/share/icons/hicolor/64x64/apps" \
         "$APPDIR/usr/share/icons/hicolor/48x48/apps" \
         "$APPDIR/usr/share/icons/hicolor/32x32/apps" \
         "$APPDIR/usr/share/icons/hicolor/16x16/apps"
cp "$BIN" "$APPDIR/usr/bin/naranjahub"
cp "$PROJ/packaging/AppRun" "$APPDIR/AppRun"
chmod +x "$APPDIR/AppRun" "$APPDIR/usr/bin/naranjahub"
cp "$PROJ/packaging/naranjahub.desktop" "$APPDIR/naranjahub.desktop"
cp "$PROJ/assets/icon-256.png" "$APPDIR/naranjahub.png"
cp "$PROJ/assets/icon-256.png" "$APPDIR/.DirIcon"
cp "$PROJ/assets/icon-256.png" "$APPDIR/usr/share/icons/hicolor/256x256/apps/naranjahub.png"
cp "$PROJ/assets/icon-128.png" "$APPDIR/usr/share/icons/hicolor/128x128/apps/naranjahub.png"
cp "$PROJ/assets/icon-64.png"  "$APPDIR/usr/share/icons/hicolor/64x64/apps/naranjahub.png"
cp "$PROJ/assets/icon-48.png"  "$APPDIR/usr/share/icons/hicolor/48x48/apps/naranjahub.png"
cp "$PROJ/assets/icon-32.png"  "$APPDIR/usr/share/icons/hicolor/32x32/apps/naranjahub.png"
cp "$PROJ/assets/icon-16.png"  "$APPDIR/usr/share/icons/hicolor/16x16/apps/naranjahub.png"
cp "$PROJ/LICENSE" "$PROJ/readme.txt" "$APPDIR/usr/share/doc/naranjahub/"
"$TOOLS/appimagetool" "$APPDIR" "$OUT/NaranjaHub-$VERSION-$ARCH.AppImage" \
    2>&1 | tail -2

# 3) .deb
DEB="$DIST/deb-root"
mkdir -p "$DEB/DEBIAN" "$DEB/usr/bin" \
         "$DEB/usr/share/applications" \
         "$DEB/usr/share/icons/hicolor/256x256/apps" \
         "$DEB/usr/share/icons/hicolor/128x128/apps" \
         "$DEB/usr/share/icons/hicolor/64x64/apps" \
         "$DEB/usr/share/icons/hicolor/48x48/apps" \
         "$DEB/usr/share/icons/hicolor/32x32/apps" \
         "$DEB/usr/share/icons/hicolor/16x16/apps" \
         "$DEB/usr/share/doc/naranjahub"
cp "$BIN" "$DEB/usr/bin/naranjahub"
cp "$PROJ/packaging/naranjahub.desktop" "$DEB/usr/share/applications/"
for size in 256x256 128x128 64x64 48x48 32x32 16x16; do
    short="${size%x*}"
    cp "$PROJ/assets/icon-$short.png" "$DEB/usr/share/icons/hicolor/$size/apps/naranjahub.png"
done
cp "$PROJ/LICENSE" "$PROJ/readme.txt" "$PROJ/packaging/copyright" "$DEB/usr/share/doc/naranjahub/"
INSTALLED_SIZE="$(du -sk "$DEB/usr" | cut -f1)"
sed -e "s/@ARCH@/amd64/" -e "s/@INSTALLED_SIZE@/$INSTALLED_SIZE/" -e "s/@VERSION@/$VERSION/" \
    "$PROJ/packaging/control" > "$DEB/DEBIAN/control"
dpkg-deb --build --root-owner-group "$DEB" "$OUT/naranjahub_${VERSION}_amd64.deb" >/dev/null

# 4) .run autoextraíble
RUN_ROOT="$DIST/run-root"
mkdir -p "$RUN_ROOT/NaranjaHub-$VERSION/icons"
cp "$BIN" "$RUN_ROOT/NaranjaHub-$VERSION/naranjahub"
cp "$PROJ/assets/icon-256.png" "$RUN_ROOT/NaranjaHub-$VERSION/icons/naranjahub-256.png"
cp "$PROJ/packaging/naranjahub.desktop.template" "$RUN_ROOT/NaranjaHub-$VERSION/"
cp "$PROJ/LICENSE" "$PROJ/readme.txt" "$RUN_ROOT/NaranjaHub-$VERSION/"
chmod +x "$RUN_ROOT/NaranjaHub-$VERSION/naranjahub"
# La versión real se inyecta en el instalador (evita números desactualizados).
sed -e "s/@VERSION@/$VERSION/" "$PROJ/packaging/installer_header.sh" > "$DIST/installer.sh"
chmod +x "$DIST/installer.sh"
tar -czf "$DIST/payload.tar.gz" -C "$RUN_ROOT" "NaranjaHub-$VERSION"
base64 -w 76 "$DIST/payload.tar.gz" >> "$DIST/installer.sh"
mv "$DIST/installer.sh" "$OUT/NaranjaHub-$VERSION-$ARCH.run"
chmod +x "$OUT/NaranjaHub-$VERSION-$ARCH.run"

# 5) Código fuente (.zip) — completo para reconstruir todos los entregables
ZIP_ROOT="$DIST/naranjahub"
mkdir -p "$ZIP_ROOT"
cp -r "$PROJ/src" "$PROJ/assets" "$PROJ/packaging" "$ZIP_ROOT/"
cp "$PROJ/Cargo.toml" "$PROJ/LICENSE" "$PROJ/readme.txt" "$PROJ/.gitignore" "$ZIP_ROOT/"
[ -f "$PROJ/Cargo.lock" ] && cp "$PROJ/Cargo.lock" "$ZIP_ROOT/"
mkdir -p "$ZIP_ROOT/scripts"
# Script maestro de construcción y utilidades (para regenerar los entregables)
cp /home/z/my-project/scripts/build_release.sh "$ZIP_ROOT/scripts/" 2>/dev/null || true
cp /home/z/my-project/scripts/make_icons.py "$ZIP_ROOT/scripts/" 2>/dev/null || true
cp /home/z/my-project/scripts/debug_fake_appimage.py "$ZIP_ROOT/scripts/" 2>/dev/null || true
find "$ZIP_ROOT" -name "*.rs" -exec chmod 644 {} \;
find "$ZIP_ROOT/scripts" -name "*.sh" -exec chmod 755 {} \;
(cd "$DIST" && zip -qr "$OUT/NaranjaHub-$VERSION-codigo-fuente.zip" naranjahub)

# 6) readme + sumas
cp "$PROJ/readme.txt" "$OUT/readme.txt"
rm -f "$OUT/SHA256SUMS.txt"
cd "$OUT"
sha256sum \
    "NaranjaHub-$VERSION-$ARCH.AppImage" \
    "naranjahub_${VERSION}_amd64.deb" \
    "NaranjaHub-$VERSION-$ARCH.run" \
    "NaranjaHub-$VERSION-codigo-fuente.zip" \
    > SHA256SUMS.txt

echo "== entregables =="
ls -la "$OUT"
