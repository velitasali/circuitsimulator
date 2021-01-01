#!/bin/bash
set -e

BIN_PATH="$1"
shift

BIN_NAME="$(basename "$BIN_PATH")"

# Only bundle the main GUI application binary 'circuitsimulator'
if [ "$BIN_NAME" != "circuitsimulator" ]; then
    exec "$BIN_PATH" "$@"
fi

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
BIN_DIR="$(dirname "$BIN_PATH")"
APP_BUNDLE="$BIN_DIR/Circuit Simulator.app"
CONTENTS="$APP_BUNDLE/Contents"
MACOS_DIR="$CONTENTS/MacOS"
RESOURCES_DIR="$CONTENTS/Resources"

mkdir -p "$MACOS_DIR" "$RESOURCES_DIR"

# Copy the binary into the bundle
cp -f "$BIN_PATH" "$MACOS_DIR/circuitsimulator"
chmod +x "$MACOS_DIR/circuitsimulator"

# Copy icons and assets
if [ -f "$REPO_ROOT/resources/icons/circuitsimulator.icns" ]; then
    cp -f "$REPO_ROOT/resources/icons/circuitsimulator.icns" "$RESOURCES_DIR/"
fi
if [ -f "$REPO_ROOT/resources/icons/Assets.car" ]; then
    cp -f "$REPO_ROOT/resources/icons/Assets.car" "$RESOURCES_DIR/"
fi

# Ensure Info.plist is in place
cat << 'PLIST' > "$CONTENTS/Info.plist"
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>CFBundleAllowMixedLocalizations</key>
	<true/>
	<key>CFBundleDevelopmentRegion</key>
	<string>en</string>
	<key>CFBundleDisplayName</key>
	<string>Circuit Simulator</string>
	<key>CFBundleExecutable</key>
	<string>circuitsimulator</string>
	<key>CFBundleIconFile</key>
	<string>circuitsimulator</string>
	<key>CFBundleIconName</key>
	<string>circuitsimulator</string>
	<key>CFBundleIdentifier</key>
	<string>com.circuitsimulator.app</string>
	<key>CFBundleName</key>
	<string>Circuit Simulator</string>
	<key>CFBundlePackageType</key>
	<string>APPL</string>
	<key>CFBundleSignature</key>
	<string>????</string>
	<key>LSMinimumSystemVersion</key>
	<string>13</string>
	<key>NSPrincipalClass</key>
	<string>NSApplication</string>
	<key>NSSupportsAutomaticGraphicsSwitching</key>
	<true/>
</dict>
</plist>
PLIST

echo -n 'APPL????' > "$CONTENTS/PkgInfo"

# Execute from inside the bundle so macOS LaunchServices associates the process with it
exec "$MACOS_DIR/circuitsimulator" "$@"
