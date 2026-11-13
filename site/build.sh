#!/usr/bin/env bash
# Build the static site into site/dist: the pages as they are, plus the pixfx-canvas wasm engine in dist/pkg.
# Usage: site/build.sh            (from anywhere in the repo)
#        site/build.sh --serve    (build, then serve dist on http://127.0.0.1:8770/)
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
site="$root/site"
out="$site/dist"
pkg="$root/target/site-pkg"

wasm-pack build "$root/crates/pixfx-canvas" --release --target web --out-dir "$pkg"

rm -rf "$out"
mkdir -p "$out/pkg"
rsync -a --exclude dist --exclude build.sh "$site/" "$out/"
cp "$pkg/pixfx_canvas.js" "$pkg/pixfx_canvas_bg.wasm" "$out/pkg/"
echo "site: $out"

if [[ "${1:-}" == "--serve" ]]; then
  cd "$out" && python3 -m http.server 8770 --bind 127.0.0.1
fi
