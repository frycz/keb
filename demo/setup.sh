#!/bin/sh
# Recreates the messy folder that demo.tape renames. Touches playground/demo only.
set -eu

root=$(cd "$(dirname "$0")/.." && pwd)
dir="$root/playground/demo"

rm -rf "$dir"
mkdir -p "$dir"
cd "$dir"

touch \
  "Final Report (v2) FINAL.docx" \
  "Screenshot 2026-09-29 at 10.14.03.png" \
  "Ünïcödé Notes.md" \
  "IMG_0042.JPG" \
  "MyComponent.tsx" \
  "Copy of budget (1).xlsx" \
  "tax_return__2025.PDF" \
  "already-fine.md"
