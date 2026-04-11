#!/usr/bin/env bash
# Record the ctxforge hero demo GIF.
# Prerequisites: asciinema, a built ctxforge binary in PATH.
#
# Usage: ./scripts/record-demo.sh [output.cast]

set -euo pipefail

OUTPUT="${1:-demo.cast}"
COLS=120
ROWS=35

echo "Recording to ${OUTPUT} (${COLS}x${ROWS})"
echo "Run the demo interactively. Press Ctrl-D or type 'exit' when done."

asciinema rec \
  --cols "${COLS}" \
  --rows "${ROWS}" \
  --title "ctxforge — the missing TUI for AI coding" \
  "${OUTPUT}"

echo "Saved: ${OUTPUT}"
echo "Convert to GIF: ./scripts/gif-convert.sh ${OUTPUT}"
