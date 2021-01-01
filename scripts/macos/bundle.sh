#!/bin/bash
set -e

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
BUILD_TYPE="${1:-debug}"

TARGET_DIR="$REPO_ROOT/target/$BUILD_TYPE"
BINARY="$TARGET_DIR/circuitsimulator"

if [ ! -f "$BINARY" ]; then
    echo "Binary not found at $BINARY. Building with cargo build..."
    if [ "$BUILD_TYPE" = "release" ]; then
        cargo build --release -p cs-app
    else
        cargo build -p cs-app
    fi
fi

APP_BUNDLE="$TARGET_DIR/Circuit Simulator.app"
CONTENTS="$APP_BUNDLE/Contents"
MACOS_DIR="$CONTENTS/MacOS"
RESOURCES_DIR="$CONTENTS/Resources"

echo "Creating bundle at: $APP_BUNDLE"
rm -rf "$APP_BUNDLE"
mkdir -p "$MACOS_DIR" "$RESOURCES_DIR"

cp "$BINARY" "$MACOS_DIR/circuitsimulator"
chmod +x "$MACOS_DIR/circuitsimulator"

if [ -f "$REPO_ROOT/resources/icons/circuitsimulator.icns" ]; then
    cp "$REPO_ROOT/resources/icons/circuitsimulator.icns" "$RESOURCES_DIR/"
fi
if [ -f "$REPO_ROOT/resources/icons/Assets.car" ]; then
    cp "$REPO_ROOT/resources/icons/Assets.car" "$RESOURCES_DIR/"
fi

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

# Touch the bundle so macOS LaunchServices picks up any changes
touch "$APP_BUNDLE"

echo "Successfully built bundle: $APP_BUNDLE"
