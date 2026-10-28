#!/usr/bin/env bash
# Build docs/spec/spec.html from the spec parts (README.md, then 01…06), in one page.
# Requires pandoc 3.x. Run from anywhere:
#   mise exec pandoc@3.1.11.1 -- ./scripts/spec/build.sh
set -euo pipefail
cd "$(dirname "$0")/../.."
PANDOC="${PANDOC:-pandoc}"

parts=(docs/spec/README.md docs/spec/[0-9][0-9]-*.md)

"$PANDOC" "${parts[@]}" -f markdown -o docs/spec/spec.html \
  --standalone --embed-resources --toc --toc-depth=2 \
  --metadata pagetitle="Tadidy — Specification" --metadata lang=en \
  --lua-filter scripts/spec/filters.lua --css scripts/spec/spec.css \
  --no-highlight --metadata document-css=false
echo "docs/spec/spec.html generated from ${#parts[@]} parts"
