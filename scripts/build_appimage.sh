#!/usr/bin/env bash
set -e

APP_NAME="StarPSX"
BIN_NAME="starpsx"

APPDIR="${APP_NAME}.AppDir"
BUILD_DIR="target/${BUILD_TARGET:+$BUILD_TARGET/}release"

echo "[1/3] Preparing AppDir..."
rm -rf "$APPDIR"
mkdir -p "$APPDIR/usr/bin"
mkdir -p "$APPDIR/usr/lib"

echo "[2/3] Copying binary..."
cp "$BUILD_DIR/$BIN_NAME" "$APPDIR/usr/bin/"
chmod +x "$APPDIR/usr/bin/$BIN_NAME"

echo "[3/3] Creating AppRun..."
cat > "$APPDIR/AppRun" << EOF
#!/bin/bash
HERE="\$(dirname "\$(readlink -f "\$0")")"
export LD_LIBRARY_PATH="\$HERE/usr/lib:\$LD_LIBRARY_PATH"
exec "\$HERE/usr/bin/$BIN_NAME" "\$@"
EOF
chmod +x "$APPDIR/AppRun"

cat > "$APPDIR/$BIN_NAME.desktop" << EOF
[Desktop Entry]
Name=$APP_NAME
Exec=$BIN_NAME
Icon=$BIN_NAME
Type=Application
Categories=Game;
EOF

# TODO: Add an actual icon here
printf '\x89PNG\r\n\x1a\n' > "$APPDIR/$BIN_NAME.png"

echo "[*] Checking for shared libraries..."
ldd "$BUILD_DIR/$BIN_NAME" || true

echo "[*] Building AppImage..."
appimagetool "$APPDIR"

echo "[✓] Done:"
ls -lh *.AppImage
