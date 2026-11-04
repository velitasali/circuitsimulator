#!/bin/sh
# Lay out an AppDir for circuitsimulator and, when appimagetool is on PATH,
# turn it into an AppImage.
#
#   ./scripts/linux/package.sh
#   ./scripts/linux/package.sh debug
#
# Qt, ALSA, udev, and Vulkan still have to be on the target machine unless
# appimagetool bundles them. This script does not download those libraries.

set -e

ROOT="$(CDPATH= cd -- "$(dirname "$0")/../.." && pwd)"
CONFIG="${1:-release}"
BIN="$ROOT/target/$CONFIG/circuitsimulator"

VERSION="$(sed -n '/^\[workspace\.package\]/,/^\[/s/^version *= *"\([^"]*\)"/\1/p' "$ROOT/Cargo.toml" | head -n 1)"
VERSION="${VERSION:-0.1.0}"
export VERSION

if [ ! -f "$BIN" ]; then
    if [ "$CONFIG" = "release" ]; then
        echo "No binary at $BIN. Build it with: cargo build --release -p cs-app" >&2
    else
        echo "No binary at $BIN. Build it with: cargo build -p cs-app" >&2
    fi
    exit 1
fi

APPDIR="$ROOT/dist/linux/CircuitSimulator.AppDir"
rm -rf "$APPDIR"
mkdir -p "$APPDIR/usr/bin" "$APPDIR/usr/share/applications" "$APPDIR/usr/share/icons/hicolor/256x256/apps"

cp "$BIN" "$APPDIR/usr/bin/circuitsimulator"
chmod +x "$APPDIR/usr/bin/circuitsimulator"

if [ -f "$ROOT/resources/icons/circuitsimulator.png" ]; then
    cp "$ROOT/resources/icons/circuitsimulator.png" "$APPDIR/circuitsimulator.png"
    cp "$ROOT/resources/icons/circuitsimulator.png" "$APPDIR/usr/share/icons/hicolor/256x256/apps/circuitsimulator.png"
fi

cat > "$APPDIR/circuitsimulator.desktop" << EOF
[Desktop Entry]
Type=Application
Name=Circuit Simulator
Exec=circuitsimulator
Icon=circuitsimulator
Categories=Education;Electronics;
X-AppImage-Version=${VERSION}
EOF
cp "$APPDIR/circuitsimulator.desktop" "$APPDIR/usr/share/applications/circuitsimulator.desktop"

cat > "$APPDIR/AppRun" << 'EOF'
#!/bin/sh
HERE="$(dirname "$(readlink -f "$0")")"
exec "$HERE/usr/bin/circuitsimulator" "$@"
EOF
chmod +x "$APPDIR/AppRun"

echo "AppDir written to $APPDIR"

if command -v linuxdeploy >/dev/null 2>&1; then
    export QML_SOURCES_PATHS="$ROOT/qml"
    export NO_STRIP=true
    if command -v qmake >/dev/null 2>&1; then
        export QMAKE="$(command -v qmake)"
    fi
    if command -v linuxdeploy-plugin-qt >/dev/null 2>&1; then
        linuxdeploy --appdir "$APPDIR" --plugin qt || echo "warning: linuxdeploy qt plugin failed, continuing with raw AppDir" >&2
    else
        linuxdeploy --appdir "$APPDIR" || echo "warning: linuxdeploy failed, continuing with raw AppDir" >&2
    fi
fi

if command -v appimagetool >/dev/null 2>&1; then
    OUT="$ROOT/dist/linux/CircuitSimulator-x86_64.AppImage"
    ARCH="${ARCH:-x86_64}" appimagetool "$APPDIR" "$OUT"
    echo "AppImage written to $OUT"
else
    echo "appimagetool is not on PATH. The AppDir is ready; install appimagetool to pack it."
fi
