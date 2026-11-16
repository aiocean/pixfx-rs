#!/usr/bin/env bash
# Build the static site into site/dist: the pages as they are, the engine in dist/pkg (the pixfx-canvas2d bundle, its
# painting worker and the pixfx-web wasm), and the arena's duel, which the landing and effect pages show: its page
# kit (page modules, its own wasm, the trained policies), written into site/arena by the arena's site/embed.sh from
# the checkout at ../../projects/arena (or $ARENA).
# Usage: site/build.sh            (from anywhere in the repo)
#        site/build.sh --serve    (build, then serve dist on http://127.0.0.1:8770/)
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
site="$root/site"
out="$site/dist"

(cd "$root/packages/pixfx-web" && bun install && bun run build:wasm)
(cd "$root/packages/pixfx-canvas2d" && bun install && bun run build)

arena="${ARENA:-$root/../../projects/arena}"
rm -rf "$site/arena"
"$arena/site/embed.sh" "$site/arena"

rm -rf "$out"
mkdir -p "$out"
rsync -a --exclude dist --exclude build.sh --exclude preload.js --exclude tests "$site/" "$out/"
cp -R "$root/packages/pixfx-canvas2d/dist" "$out/pkg"
bun "$site/preload.js" "$out"
echo "site: $out"

if [[ "${1:-}" == "--serve" ]]; then
  cd "$out" && python3 -m http.server 8770 --bind 127.0.0.1
fi
