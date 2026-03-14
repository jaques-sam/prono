#!/bin/bash
set -euo pipefail

# Build a Synology SPK package from a pre-built backend binary.
#
# Usage: ./build-spk.sh <path-to-binary> <output-dir>
#   e.g.: ./build-spk.sh target/x86_64-unknown-linux-gnu/release/prono-backend dist/

BINARY="${1:?Usage: build-spk.sh <binary-path> <output-dir>}"
OUTPUT_DIR="${2:?Usage: build-spk.sh <binary-path> <output-dir>}"

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
SPK_DIR="$SCRIPT_DIR/spk"
WORK_DIR="$(mktemp -d)"

trap 'rm -rf "$WORK_DIR"' EXIT

# Create package.tgz containing the binary
mkdir -p "$WORK_DIR/package"
cp "$BINARY" "$WORK_DIR/package/prono-backend"
chmod 755 "$WORK_DIR/package/prono-backend"
tar czf "$WORK_DIR/package.tgz" -C "$WORK_DIR/package" .

# Copy INFO
cp "$SPK_DIR/INFO" "$WORK_DIR/INFO"

# Copy scripts
mkdir -p "$WORK_DIR/scripts"
cp "$SPK_DIR/scripts/"* "$WORK_DIR/scripts/"
chmod 755 "$WORK_DIR/scripts/"*

# Copy icons (resize from app assets)
ICON_SRC="$SCRIPT_DIR/../../app/assets/icon-256.png"
if [ -f "$ICON_SRC" ]; then
    cp "$ICON_SRC" "$WORK_DIR/PACKAGE_ICON.PNG"
    cp "$ICON_SRC" "$WORK_DIR/PACKAGE_ICON_256.PNG"
fi

# Assemble SPK (which is just a tar archive)
mkdir -p "$OUTPUT_DIR"
VERSION=$(grep '^version=' "$SPK_DIR/INFO" | cut -d'"' -f2)
SPK_NAME="prono-backend-${VERSION}.spk"
tar cf "$OUTPUT_DIR/$SPK_NAME" -C "$WORK_DIR" INFO package.tgz scripts PACKAGE_ICON.PNG PACKAGE_ICON_256.PNG

echo "Built: $OUTPUT_DIR/$SPK_NAME"
