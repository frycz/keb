#!/bin/sh
# Recreates the WhatsApp folder that demo.tape and preview.tape rename. Touches playground/demo only.
set -eu

root=$(cd "$(dirname "$0")/.." && pwd)
dir="$root/playground/demo"

rm -rf "$dir"
mkdir -p "$dir"
cd "$dir"

touch \
  "WhatsApp Image 2026-09-28 at 21.14.03.jpeg" \
  "WhatsApp Image 2026-09-28 at 21.14.03 (1).jpeg" \
  "WhatsApp Image 2026-09-28 at 21.14.07.jpeg"
