#!/usr/bin/env bash
# Build the static site into site/dist: the pages as they are (with the fighters' sprite sheets in assets/sprites), and
# the engine in dist/pkg (the pixfx-canvas2d bundle, its painting worker and the pixfx-web wasm).
# Usage: site/build.sh            (from anywhere in the repo)
#        site/build.sh --serve    (build, then serve dist on http://127.0.0.1:8770/)
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
site="$root/site"
out="$site/dist"

(cd "$root/packages/pixfx-web" && bun install && bun run build:wasm)
(cd "$root/packages/pixfx-canvas2d" && bun install && bun run build)
(cd "$site" && bun install --frozen-lockfile)

rm -rf "$out"
mkdir -p "$out"
rsync -a --exclude dist --exclude node_modules --exclude build.sh --exclude bake-previews.js \
  --exclude package.json --exclude bun.lock --exclude preload.js --exclude tests "$site/" "$out/"
cp -R "$root/packages/pixfx-canvas2d/dist" "$out/pkg"
# Render the high-density versions from the same fighter poses and pivot.
for look in thunder frost; do
  (cd "$root" && cargo run --release -p pixfx-fighter --example poses -- "$look" --pixel-density 2 --out "$out/assets/sprites/dense")
done
bun "$site/bake-previews.js" "$out"
bun "$site/preload.js" "$out"
echo "site: $out"

if [[ "${1:-}" == "--serve" ]]; then
  cd "$out" && python3 -m http.server 8770 --bind 127.0.0.1
fi
