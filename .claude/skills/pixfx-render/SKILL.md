---
name: pixfx-render
description: Use when making files from pixfx - sprite sheets with Aseprite-style JSON, gifs, rendered clips for a game to play without painting, fighter pose sheets, videos (mp4), or a new kind of output such as a frame strip or a report - or when an export script's flags do not cover an option you need.
---

# Make files from pixfx

There is no command-line tool. Each output is a short script in the `examples/` of the crate that owns its logic,
run with `cargo run -p <crate> --example <script> -- <args>`. A script reads arguments, calls the library and
writes files. Need something the flags don't cover? Edit or copy the script; logic worth testing moves into the
library.

## The scripts

| Output | Command | Writes (`<name>` = id with `/` as `_`) |
|---|---|---|
| Sprite sheets + beanfx JSON | `cargo run -p pixfx-export --example sprite_sheet -- <id> [--seed N] [--palette WORD] [--num NAME=V] [--out DIR]` | `<name>.png`, `_color.png`, `_light.png`, `.json` |
| Looping gif | `cargo run -p pixfx-export --example gif -- <id> [--scale N] [--seed N] [--palette WORD] [--num NAME=V] [--actor PNG,PNG... [--actor-every N] [--show-actor]] [--out DIR]` | `<name>.gif` (over a dark backdrop); `--actor` plays an effect that reads its sprite on these sprite frames, feet on the origin; `--show-actor` draws the sprite too, between the back and front layers |
| An effect in play, on a stage | `cargo run -p pixfx-export --example stage -- <id> [--actor PNG] [--target DX,DY] [--order-at SECS] [--aim converge\|pierce] [--feel] [--secs N] [--scale N] [--out DIR]` | `<name>.stage.gif`: the effect on a caster with a target facing them, through `pixfx-scene`; `--order-at` orders a volley at the target, `--feel` shows the hit-stop and screen shake its cues ask for |
| Video (ffmpeg on `PATH`) | `cargo run -p pixfx-export --example mp4 -- <id> [--scale N] [--loops N] [--seed N] [--palette WORD] [--num NAME=V] [--out DIR]` | `<name>.mp4`; `write_video` takes `.webm` and others by extension |
| Rendered clip for `loadClip` | `cargo run -p pixfx-export --example render_clip -- <id> [--seed N] [--palette WORD] [--num NAME=V] [--out DIR]` | `<name>.clip.json`, `<name>.clip.<n>.png` |
| Mannequin clip sheet | `cargo run -p pixfx-body --example clip_sheet -- <clip> [--camera side\|iso\|topdown] [--out DIR]`; `--example catalog [--camera ...]` for every clip | `target/body/<clip>/`: `composite.png`, `hull.png`, `bones.png`, `parts.png`, `<clip>.gif`, `<clip>.json` (skill `pixfx-body`) |
| Fighter pose sheet | `cargo run -p pixfx-fighter --example poses -- <thunder\|frost\|alchemist> [--blank POSE,...] [--pose POSE [--states]] [--out DIR]` | `<look>-poses.png`, or `<look>-<POSE>.png` and its cells |
| Parity side-by-side | `cargo run -p pixfx-export --example parity -- <id> [--pixelfx DIR]` (a pixelfx port needs a pixelfx checkout: `--pixelfx`, else `$PIXELFX`, else `./pixelfx`) | `target/parity/<id>.side.gif` (see `pixfx-effect`) |

- `--palette` takes a preset: `fire`, `ice`, `poison`, `holy`, `shadow`, `arcane`, `blood`, `electric`, `nature`,
  `gold`.
- Run from the repo root. `--out` defaults to `target/out` (`out` for `poses`). A bad flag exits 2 with the usage.
- Effect ids: `pixfx_effects::EFFECTS`, or `rg -n 'id: "' crates/pixfx-effects/src`.
- An effect's own number option takes `--num NAME=V`, once per option (`--num hitX=-20 --num hitY=-48`); a ramp,
  a colour or a glow margin has no flag: set it in the script's `Options` or copy the script.
- A clip written into a game: `--out <game>/public/fx`, then `await loadClip("fx/<name>.clip.json")`.

## A new output (a frame strip, a report, another format)

Put it where the existing outputs live: the format's logic as a library function in `crates/pixfx-export/src/`
(pure layout in plain functions, files only in a `write_*`), a script in `crates/pixfx-export/examples/`, and a
test in `crates/pixfx-export/tests/`. Never a binary crate or a CLI.

Frames come from `pixfx_core::render_frames(effect, &opts, margin) -> Rendered` (`w`, `h`, `fps`, `frames`).
Turn each `Frame` into RGBA with `pixfx_draw::composite(frame, &mut rgba, Some(&backdrop))`, the way
`src/gif.rs` does.

`src/video.rs` (`write_video`, piping frames to ffmpeg) is the example to copy for a format that needs an outside
tool: nearest-neighbour upscale padded to even sides, stdin closed then `wait()` with the exit status checked, a
clear error when the tool is missing, and its test `#[ignore]`d (`-- --ignored`) because it needs ffmpeg.

Check: `cargo clippy -p pixfx-export --all-targets -- -D warnings`, `cargo test -p pixfx-export`, run the script
once, and open what it wrote.
