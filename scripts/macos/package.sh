#!/bin/sh
# Package a release build of Circuit Simulator into a distributable .dmg.
#
# Usage:
#   scripts/macos/package.sh [path/to/circuitsimulator.app]
#
# If no path is given, the newest release build under
# build_template/executables/*/release/*.app is used.
#
# Requires: qmake's macdeployqt (found automatically via `qmake -query QT_INSTALL_BINS`).

set -e

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"

APP_PATH="$1"
if [ -z "$APP_PATH" ]; then
    APP_PATH="$(find "$REPO_ROOT/build_template/executables" -maxdepth 3 -type d -name '*.app' -path '*/release/*' 2>/dev/null | sort | tail -n 1)"
fi

if [ -z "$APP_PATH" ] || [ ! -d "$APP_PATH" ]; then
    echo "error: could not find a release .app bundle. Build with 'make' first, or pass the .app path explicitly." >&2
    exit 1
fi

QMAKE_BIN="$(command -v qmake || true)"
if [ -z "$QMAKE_BIN" ]; then
    echo "error: qmake not found in PATH" >&2
    exit 1
fi

QT_BINS="$("$QMAKE_BIN" -query QT_INSTALL_BINS)"
MACDEPLOYQT="$QT_BINS/macdeployqt"
if [ ! -x "$MACDEPLOYQT" ]; then
    echo "error: macdeployqt not found at $MACDEPLOYQT" >&2
    exit 1
fi

echo "Packaging: $APP_PATH"
echo "Using:     $MACDEPLOYQT"

# -qmldir lets macdeployqt scan the app's QML for imports so the right
# QtQuick/Controls/Layouts plugins get bundled. It has to cover all of src/:
# the .qml files sit next to the C++ they back, not in one directory.
# -codesign=- ad-hoc signs the bundle (no Developer ID certificate is installed
# on this machine); replace with a Developer ID identity for notarized releases.
"$MACDEPLOYQT" "$APP_PATH" \
    -qmldir="$REPO_ROOT/qml" \
    -codesign=- \
    -dmg \
    -verbose=1

DMG_PATH="${APP_PATH%.app}.dmg"
if [ -f "$DMG_PATH" ]; then
    echo ""
    echo "Created: $DMG_PATH"
else
    echo "warning: expected dmg at $DMG_PATH but it was not found" >&2
fi
