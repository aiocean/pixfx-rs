#!/usr/bin/env bash
# Build the arena site into games/arena/site/dist: its pages, the player docs rendered from games/arena/docs/*.md,
# the duel's page code from the pixfx site (site/assets: duel*.js, skill-icons.js, the icon atlases, site.css), and the
# engine in dist/pkg: the pixfx-canvas2d bundle, its painting worker, the pixfx-web wasm and the trained policies.
# Usage: games/arena/site/build.sh            (from anywhere in the repo)
#        games/arena/site/build.sh --serve    (build, then serve dist on http://127.0.0.1:8771/)
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
root="$(cd "$here/../../.." && pwd)"
out="$here/dist"

(cd "$root/packages/pixfx-web" && bun install && bun run build:wasm)
(cd "$root/packages/pixfx-canvas2d" && bun install && bun run build)

rm -rf "$out"
mkdir -p "$out/assets"
cp "$here"/*.html "$out/"
cp "$here"/assets/* "$out/assets/"
cp "$root"/site/assets/duel*.js "$root"/site/assets/skill-icons.js "$root"/site/assets/skills-*.png "$root"/site/assets/site.css "$out/assets/"
cp -R "$root/packages/pixfx-canvas2d/dist" "$out/pkg"
cp "$root"/games/arena/crates/pixfx-duel/weights/*.bin "$out/pkg/"
bun "$here/docs.js" "$root/games/arena/docs" "$out/docs"
bun "$root/site/preload.js" "$out"
echo "arena site: $out"

if [[ "${1:-}" == "--serve" ]]; then
  cd "$out" && python3 -m http.server 8771 --bind 127.0.0.1
fi
