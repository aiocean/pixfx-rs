#!/usr/bin/env bash
# The duel's skill icons: from a generated 4 x 2 sheet (skill-icons.prompt.txt: Thunder's slots 1, 2, 3 and ultimate
# on the top row, Frost's under it, a plain colour between the cells) to the atlases the hero draws, one per icon size:
# site/assets/skills-32.png and skills-24.png, a row per school and a column per slot.
# Each cell is trimmed of the gap round it and any frame inside that, squared on the slot's field colour, snapped to its own pixel grid by the pixel
# snapper, then reduced to the icon size with a small palette and no dithering.
# Usage: site/art/skill-icons.sh <sheet.png> [out dir, default site/assets]
set -euo pipefail

sheet="$1"
site="$(cd "$(dirname "$0")/.." && pwd)"
out="${2:-$site/assets}"
snapper=spritefusion-pixel-snapper
field="#1a1424"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

magick "$sheet" -alpha off -fuzz 5% -trim +repage -fuzz 5% -trim +repage "$tmp/sheet.png"
magick "$tmp/sheet.png" -crop 4x2@ +repage "$tmp/cell-%d.png"
for i in 0 1 2 3 4 5 6 7; do
  cell="$tmp/cell-$i.png"
  # the gap left at the cell's edges, then a frame the model drew round the icon
  magick "$cell" -fuzz 8% -trim +repage -fuzz 8% -trim +repage "$cell"
  side="$(magick identify -format '%[fx:max(w,h)]' "$cell")"
  magick "$cell" -background "$field" -gravity center -extent "${side}x${side}" "$cell"
  "$snapper" "$cell" "$tmp/snap-$i.png" 32 > /dev/null
done
for size in 32 24; do
  for i in 0 1 2 3 4 5 6 7; do
    magick "$tmp/snap-$i.png" -filter Box -resize "${size}x${size}!" -dither None -colors 32 "$tmp/icon-$size-$i.png"
  done
  magick \( "$tmp"/icon-"$size"-{0,1,2,3}.png +append \) \( "$tmp"/icon-"$size"-{4,5,6,7}.png +append \) -append \
    -strip "PNG32:$out/skills-$size.png"
  echo "$out/skills-$size.png"
done
