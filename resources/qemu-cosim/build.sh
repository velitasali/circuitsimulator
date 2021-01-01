#!/usr/bin/env bash
#
# Builds qemu-system-xtensa with the "esp32-cs" machine (Circuit Simulator's
# co-simulation fork of Espressif's public "esp32" machine) for the host
# platform. Clones espressif/qemu at a pinned commit, overlays this
# directory's patch/ tree, configures for xtensa-softmmu only, and builds.
#
# Usage: resources/qemu-cosim/build.sh [build-dir]
#   build-dir defaults to resources/qemu-cosim/build (gitignored, not
#   committed -- this script and the small patch/ tree are the source of
#   truth, not a vendored QEMU tree).
#
# Requires: git, meson, ninja, pkg-config, pixman, glib (brew install
# meson ninja pkg-config pixman -- glib ships with macOS dev tools already).

set -euo pipefail

QEMU_REPO="https://github.com/espressif/qemu.git"
QEMU_BRANCH="esp-develop"
QEMU_PINNED_COMMIT="febae182e132e4055529be423a818225ebddaa3a"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BUILD_DIR="${1:-$SCRIPT_DIR/build}"
SRC_DIR="$BUILD_DIR/src"
OUT_DIR="$BUILD_DIR/out"

mkdir -p "$BUILD_DIR"

if [ ! -d "$SRC_DIR/.git" ]; then
    echo "Cloning espressif/qemu ($QEMU_BRANCH) ..."
    git clone --branch "$QEMU_BRANCH" --depth 1 "$QEMU_REPO" "$SRC_DIR"
fi

cd "$SRC_DIR"
FETCHED_COMMIT="$(git rev-parse HEAD)"
if [ "$FETCHED_COMMIT" != "$QEMU_PINNED_COMMIT" ]; then
    echo "warning: HEAD ($FETCHED_COMMIT) does not match the pinned commit" \
         "($QEMU_PINNED_COMMIT) this patch tree was written against." \
         "Continuing, but check the patch still applies cleanly." >&2
fi

echo "Overlaying esp32-cs patch tree ..."
git apply --check "$SCRIPT_DIR/patch/hw-xtensa-register.patch" 2>/dev/null \
    || echo "  (register patch already applied, skipping)"
git apply "$SCRIPT_DIR/patch/hw-xtensa-register.patch" 2>/dev/null || true
cp "$SCRIPT_DIR/patch/hw/xtensa/esp32-cs.c" hw/xtensa/esp32-cs.c

NINJA_TARGET=qemu-system-xtensa
CONFIGURE_ARGS=(--target-list=xtensa-softmmu --disable-docs)
case "$(uname -s)" in
    MINGW*|MSYS*|CYGWIN*)
        NINJA_TARGET=qemu-system-xtensa.exe
        CONFIGURE_ARGS+=(--enable-gcrypt)
        # MinGW `ar T` (thin archives) fails; wrap ar so meson gets csrD.
        WRAP_DIR="$BUILD_DIR/win-ar-wrap"
        mkdir -p "$WRAP_DIR"
        cp "$SCRIPT_DIR/win-ar" "$WRAP_DIR/ar"
        chmod +x "$WRAP_DIR/ar"
        export PATH="$WRAP_DIR:$PATH"
        ;;
esac

echo "Configuring (xtensa-softmmu only) ..."
mkdir -p "$OUT_DIR"
( cd "$OUT_DIR" && "$SRC_DIR/configure" "${CONFIGURE_ARGS[@]}" )
# ninja.exe looks up ar.exe, so the shell wrapper above is not used.
# Rewrite meson thin-archive flags in the generated ninja file.
case "$(uname -s)" in
    MINGW*|MSYS*|CYGWIN*)
        ( cd "$OUT_DIR" && python -c "from pathlib import Path; p=Path('build.ninja'); t=p.read_text(encoding='utf-8'); p.write_text(t.replace('csrDT','csrD'), encoding='utf-8')" )
        ;;
esac

echo "Building $NINJA_TARGET ..."
ninja -C "$OUT_DIR" -j"$(sysctl -n hw.ncpu 2>/dev/null || nproc)" "$NINJA_TARGET"

if [ "$(uname -s)" = "Darwin" ]; then
    echo "Signing with JIT entitlements (macOS) ..."
    ENTITLEMENTS_FILE="$BUILD_DIR/entitlements.plist"
    cat << 'EOF' > "$ENTITLEMENTS_FILE"
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>com.apple.security.cs.allow-jit</key>
    <true/>
    <key>com.apple.security.cs.allow-unsigned-executable-memory</key>
    <true/>
    <key>com.apple.security.cs.disable-executable-page-protection</key>
    <true/>
</dict>
</plist>
EOF
    xattr -cr "$OUT_DIR/qemu-system-xtensa" 2>/dev/null || true
    codesign --force --sign - --entitlements "$ENTITLEMENTS_FILE" "$OUT_DIR/qemu-system-xtensa"
fi

echo
echo "Built: $OUT_DIR/$NINJA_TARGET"
"$OUT_DIR/$NINJA_TARGET" -M help | grep -E '^esp32'
