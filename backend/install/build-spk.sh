#!/bin/bash
# Build a Synology SPK package from a pre-built backend binary.
#
# Usage: ./build-spk.sh <path-to-binary> <output-dir>
#   e.g.: ./build-spk.sh target/x86_64-unknown-linux-gnu/release/prono-backend dist/

set -euo pipefail
validate_inputs() {
    BINARY="${1:?Usage: build-spk.sh <binary-path> <output-dir>}"
    OUTPUT_DIR="${2:?Usage: build-spk.sh <binary-path> <output-dir>}"
}

setup_directories() {
    SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
    SPK_DIR="$SCRIPT_DIR/spk"
    WORK_DIR="$(mktemp -d)"
    trap 'rm -rf "$WORK_DIR"' EXIT
}

package_binary() {
    mkdir -p "$WORK_DIR/package"
    cp "$BINARY" "$WORK_DIR/package/prono-backend"
    chmod 755 "$WORK_DIR/package/prono-backend"
    tar czf "$WORK_DIR/package.tgz" -C "$WORK_DIR/package" .
}

copy_metadata() {
    cp "$SPK_DIR/INFO" "$WORK_DIR/INFO"
    mkdir -p "$WORK_DIR/conf"
    cp "$SPK_DIR/privilege" "$WORK_DIR/conf/privilege"
    mkdir -p "$WORK_DIR/scripts"
    cp "$SPK_DIR/scripts/"* "$WORK_DIR/scripts/"
    chmod 755 "$WORK_DIR/scripts/"*
}

copy_icons() {
    local icon_src="$SCRIPT_DIR/../../app/assets/icon-256.png"
    if [ -f "$icon_src" ]; then
        cp "$icon_src" "$WORK_DIR/PACKAGE_ICON.PNG"
        cp "$icon_src" "$WORK_DIR/PACKAGE_ICON_256.PNG"
    fi
}

assemble_spk() {
    mkdir -p "$OUTPUT_DIR"
    local version
    version=$(grep '^version=' "$SPK_DIR/INFO" | cut -d'"' -f2)
    local spk_name="prono-backend-${version}.spk"
    tar cf "$OUTPUT_DIR/$spk_name" -C "$WORK_DIR" INFO conf package.tgz scripts PACKAGE_ICON.PNG PACKAGE_ICON_256.PNG
    echo "Built: $OUTPUT_DIR/$spk_name"
}

validate_inputs "$@"
setup_directories
package_binary
copy_metadata
copy_icons
assemble_spk
