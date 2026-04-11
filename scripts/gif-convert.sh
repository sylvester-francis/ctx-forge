#!/usr/bin/env bash
# Convert an asciinema .cast file to a GIF using agg.
# Prerequisites: agg (https://github.com/asciinema/agg)
#
# Usage: ./scripts/gif-convert.sh demo.cast [output.gif]

set -euo pipefail

INPUT="${1:?Usage: gif-convert.sh <input.cast> [output.gif]}"
OUTPUT="${2:-${INPUT%.cast}.gif}"

agg \
  --font-family "JetBrains Mono,Menlo,monospace" \
  --font-size 14 \
  --theme monokai \
  --speed 1.5 \
  --cols 120 \
  --rows 35 \
  "${INPUT}" \
  "${OUTPUT}"

echo "GIF saved: ${OUTPUT} ($(du -h "${OUTPUT}" | cut -f1))"
