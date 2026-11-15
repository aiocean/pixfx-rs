#!/usr/bin/env bash
# The duel's skill icons: from a generated 5 x 5 sheet (skill-icons.prompt.txt: an icon per spell, left to right and
# top row first, a plain colour between the cells) to the atlases the hero draws, one per icon size:
# site/assets/skills-32.png and skills-24.png, the same 5 x 5 grid. site/assets/skill-icons.js names each cell.
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
grid=5
cells=$((grid * grid))
magick "$tmp/sheet.png" -crop ${grid}x${grid}@ +repage "$tmp/cell-%d.png"
for ((i = 0; i < cells; i++)); do
  cell="$tmp/cell-$i.png"
  # the gap left at the cell's edges, then a frame the model drew round the icon
  magick "$cell" -fuzz 8% -trim +repage -fuzz 8% -trim +repage "$cell"
  side="$(magick identify -format '%[fx:max(w,h)]' "$cell")"
  magick "$cell" -background "$field" -gravity center -extent "${side}x${side}" "$cell"
  "$snapper" "$cell" "$tmp/snap-$i.png" 32 > /dev/null
done
for size in 32 24; do
  for ((i = 0; i < cells; i++)); do
    magick "$tmp/snap-$i.png" -filter Box -resize "${size}x${size}!" -dither None -colors 32 "$tmp/icon-$size-$i.png"
  done
  rows=()
  for ((r = 0; r < grid; r++)); do
    row=()
    for ((c = 0; c < grid; c++)); do row+=("$tmp/icon-$size-$((r * grid + c)).png"); done
    rows+=("(" "${row[@]}" +append ")")
  done
  magick "${rows[@]}" -append -strip "PNG32:$out/skills-$size.png"
  echo "$out/skills-$size.png"
done
