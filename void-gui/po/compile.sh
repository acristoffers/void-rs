#!/bin/sh
# Compile .po files into .mo files for use by gettext at runtime.
# Usage: ./po/compile.sh [output_dir]
# Default output: void-gui/po/locale/

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
OUTPUT_DIR="${1:-$SCRIPT_DIR/locale}"

for po in "$SCRIPT_DIR"/*.po; do
    lang="$(basename "$po" .po)"
    mkdir -p "$OUTPUT_DIR/$lang/LC_MESSAGES"
    msgfmt "$po" -o "$OUTPUT_DIR/$lang/LC_MESSAGES/void.mo"
    echo "Compiled $lang.po -> $OUTPUT_DIR/$lang/LC_MESSAGES/void.mo"
done
