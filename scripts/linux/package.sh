#!/bin/sh
# Package circuitsimulator into a Linux AppDir, AppImage, and Debian (.deb) package.
#
#   ./scripts/linux/package.sh
#   ./scripts/linux/package.sh debug
#   QT_ROOT="$HOME/Qt/6.10.3/gcc_64" ./scripts/linux/package.sh release
#
# Uses linuxdeploy (and linuxdeploy-plugin-qt / appimagetool) when on PATH
# to bundle Qt and QML dependencies, and dpkg-deb when available to build a .deb.

set -eu

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

OUT_DIR="$ROOT/dist/linux"
APPDIR="$OUT_DIR/CircuitSimulator.AppDir"
DEBROOT="$OUT_DIR/deb-root"
STAGE_DESKTOP="$OUT_DIR/circuitsimulator.desktop"
DESKTOP_SOURCE="$ROOT/resources/circuitsimulator.desktop"
ICON="$ROOT/resources/icons/circuitsimulator.png"
MIME_SOURCE="$ROOT/resources/circuitsimulator-mime.xml"

rm -rf "$APPDIR" "$DEBROOT" "$STAGE_DESKTOP"
mkdir -p "$OUT_DIR" "$APPDIR/usr/bin"

# Prepare desktop entry outside AppDir so linuxdeploy can deploy it cleanly.
# Icon names in desktop entries must not carry a .png extension.
if [ -f "$DESKTOP_SOURCE" ]; then
    sed 's/^Icon=circuitsimulator\.png$/Icon=circuitsimulator/' "$DESKTOP_SOURCE" > "$STAGE_DESKTOP"
    if ! grep -q '^X-AppImage-Version=' "$STAGE_DESKTOP"; then
        printf '\nX-AppImage-Version=%s\n' "$VERSION" >> "$STAGE_DESKTOP"
    fi
else
    cat > "$STAGE_DESKTOP" << EOF
[Desktop Entry]
Type=Application
Name=Circuit Simulator
Exec=circuitsimulator %F
Icon=circuitsimulator
Categories=Education;Electronics;
X-AppImage-Version=${VERSION}
EOF
fi

cp "$BIN" "$APPDIR/usr/bin/circuitsimulator"
chmod +x "$APPDIR/usr/bin/circuitsimulator"

if [ "$CONFIG" = "release" ] && command -v strip >/dev/null 2>&1; then
    strip --strip-debug "$APPDIR/usr/bin/circuitsimulator"
fi

# Configure Qt discovery for linuxdeploy-plugin-qt when QT_ROOT or qmake is available.
if [ -n "${QT_ROOT:-}" ] && [ -x "$QT_ROOT/bin/qmake" ]; then
    PATH="$QT_ROOT/bin:$PATH"
    export PATH
    export QMAKE="$QT_ROOT/bin/qmake"
elif [ -z "${QMAKE:-}" ] && command -v qmake >/dev/null 2>&1; then
    QMAKE="$(command -v qmake)"
    export QMAKE
fi

if [ -n "${QMAKE:-}" ] && [ -x "$QMAKE" ]; then
    QT_LIBS="$("$QMAKE" -query QT_INSTALL_LIBS 2>/dev/null || true)"
    QT_QML="$("$QMAKE" -query QT_INSTALL_QML 2>/dev/null || true)"
    QT_PLUGINS="$("$QMAKE" -query QT_INSTALL_PLUGINS 2>/dev/null || true)"
    if [ -n "$QT_LIBS" ]; then
        LD_LIBRARY_PATH="$QT_LIBS${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
        export LD_LIBRARY_PATH
    fi
    if [ -n "$QT_QML" ] && [ -z "${QML_IMPORT_PATH:-}" ]; then
        export QML_IMPORT_PATH="$QT_QML"
    fi
    if [ -n "$QT_PLUGINS" ] && [ -z "${QT_PLUGIN_PATH:-}" ]; then
        export QT_PLUGIN_PATH="$QT_PLUGINS"
    fi
fi

export QT_VERSION_MAJOR=6
export QML_SOURCES_PATHS="$ROOT/qml"
export NO_STRIP=true
export APPIMAGE_EXTRACT_AND_RUN="${APPIMAGE_EXTRACT_AND_RUN:-1}"
export ARCH="${ARCH:-x86_64}"

write_fallback_layout() {
    mkdir -p "$APPDIR/usr/share/applications" "$APPDIR/usr/share/icons/hicolor/256x256/apps"
    cp "$STAGE_DESKTOP" "$APPDIR/circuitsimulator.desktop"
    cp "$STAGE_DESKTOP" "$APPDIR/usr/share/applications/circuitsimulator.desktop"
    if [ -f "$ICON" ]; then
        cp "$ICON" "$APPDIR/circuitsimulator.png"
        cp "$ICON" "$APPDIR/usr/share/icons/hicolor/256x256/apps/circuitsimulator.png"
    fi
    if [ ! -x "$APPDIR/AppRun" ]; then
        cat > "$APPDIR/AppRun" << 'EOF'
#!/bin/sh
HERE="$(dirname "$(readlink -f "$0")")"
exec "$HERE/usr/bin/circuitsimulator" "$@"
EOF
        chmod +x "$APPDIR/AppRun"
    fi
}

if command -v linuxdeploy >/dev/null 2>&1; then
    set -- \
        --appdir "$APPDIR" \
        --executable "$APPDIR/usr/bin/circuitsimulator" \
        --desktop-file "$STAGE_DESKTOP"
    if [ -f "$ICON" ]; then
        set -- "$@" --icon-file "$ICON"
    fi
    if command -v linuxdeploy-plugin-qt >/dev/null 2>&1; then
        linuxdeploy "$@" --plugin qt || {
            echo "warning: linuxdeploy qt plugin failed, falling back to raw AppDir layout" >&2
            write_fallback_layout
        }
    else
        linuxdeploy "$@" || {
            echo "warning: linuxdeploy failed, falling back to raw AppDir layout" >&2
            write_fallback_layout
        }
    fi
    if [ ! -x "$APPDIR/AppRun" ]; then
        write_fallback_layout
    fi
else
    write_fallback_layout
fi

echo "AppDir written to $APPDIR"

APPIMAGE_OUT="$OUT_DIR/CircuitSimulator-${ARCH}.AppImage"
if command -v appimagetool >/dev/null 2>&1; then
    rm -f "$APPIMAGE_OUT"
    appimagetool "$APPDIR" "$APPIMAGE_OUT"
    echo "AppImage written to $APPIMAGE_OUT"
elif command -v linuxdeploy >/dev/null 2>&1 && command -v linuxdeploy-plugin-appimage >/dev/null 2>&1; then
    rm -f "$OUT_DIR"/*.AppImage
    (
        cd "$OUT_DIR"
        linuxdeploy --appdir "$APPDIR" --output appimage
    )
    for generated in "$OUT_DIR"/*.AppImage; do
        if [ -f "$generated" ]; then
            if [ "$generated" != "$APPIMAGE_OUT" ]; then
                mv "$generated" "$APPIMAGE_OUT"
            fi
            echo "AppImage written to $APPIMAGE_OUT"
            break
        fi
    done
else
    echo "appimagetool is not on PATH. The AppDir is ready; install appimagetool to pack it."
fi

if command -v dpkg-deb >/dev/null 2>&1; then
    if command -v dpkg >/dev/null 2>&1; then
        DEB_ARCH="$(dpkg --print-architecture)"
    elif command -v dpkg-architecture >/dev/null 2>&1; then
        DEB_ARCH="$(dpkg-architecture -qDEB_HOST_ARCH)"
    else
        DEB_ARCH="amd64"
    fi

    mkdir -p \
        "$DEBROOT/opt/circuitsimulator" \
        "$DEBROOT/usr/bin" \
        "$DEBROOT/usr/share/applications" \
        "$DEBROOT/usr/share/icons/hicolor/256x256/apps" \
        "$DEBROOT/DEBIAN"

    cp -a "$APPDIR/." "$DEBROOT/opt/circuitsimulator/"

    cat > "$DEBROOT/usr/bin/circuitsimulator" << 'LAUNCHER'
#!/bin/sh
exec /opt/circuitsimulator/AppRun "$@"
LAUNCHER
    chmod 755 "$DEBROOT/usr/bin/circuitsimulator"

    cp "$STAGE_DESKTOP" "$DEBROOT/usr/share/applications/circuitsimulator.desktop"
    chmod 644 "$DEBROOT/usr/share/applications/circuitsimulator.desktop"

    if [ -f "$ICON" ]; then
        cp "$ICON" "$DEBROOT/usr/share/icons/hicolor/256x256/apps/circuitsimulator.png"
        chmod 644 "$DEBROOT/usr/share/icons/hicolor/256x256/apps/circuitsimulator.png"
    fi

    if [ -f "$MIME_SOURCE" ]; then
        mkdir -p "$DEBROOT/usr/share/mime/packages"
        cp "$MIME_SOURCE" "$DEBROOT/usr/share/mime/packages/circuitsimulator.xml"
        chmod 644 "$DEBROOT/usr/share/mime/packages/circuitsimulator.xml"
    fi

    cat > "$DEBROOT/DEBIAN/control" << CONTROL
Package: circuitsimulator
Version: $VERSION
Section: electronics
Priority: optional
Architecture: $DEB_ARCH
Maintainer: Circuit Simulator <https://github.com/velitasali/CircuitSimulator>
Homepage: https://github.com/velitasali/CircuitSimulator
Depends: libc6, libstdc++6, libasound2t64 | libasound2, libudev1, libvulkan1, libgl1, libxkbcommon0
Description: Real-time electronic circuit simulator
 Circuit Simulator is an electronic circuit schematic capture and
 simulation suite with integrated microcontroller emulation, code
 editing, and debugging toolchains.
CONTROL
    chmod 755 "$DEBROOT/DEBIAN"
    chmod 644 "$DEBROOT/DEBIAN/control"

    DEB_OUT="$OUT_DIR/circuitsimulator_${VERSION}_${DEB_ARCH}.deb"
    rm -f "$DEB_OUT"
    dpkg-deb --build --root-owner-group "$DEBROOT" "$DEB_OUT"
    rm -rf "$DEBROOT"
    echo "Debian package written to $DEB_OUT"
fi

rm -f "$STAGE_DESKTOP"
