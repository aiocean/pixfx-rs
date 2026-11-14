# pixfx-cli

Layer: **Shell** (command line). Read the root `AGENTS.md` ("The boundary" and "Layers and where code goes") before
changing anything here; this file only says what this crate holds.

## Job

The `pixfx` binary: `parity` (an effect against pixelfx), `sheet`, `render` (PNG sheets, GIF), `poses` (every pose
of a fighter look on one sheet, `--blank` cells left empty; `--pose P --states` one pose in every state), `pixel`
(the agent's pixel tool: `clean`, `read`, `draw`, `view`, `critique`, `sheet`, `cut`, `parts`; see
`skills/pixfx-pixel/SKILL.md`), `diff` (pixels two images differ by, with a mask; `--anchor feet`). `src/lib.rs`
holds the parts worth testing on their own (`opts`, `parity`, `pixel`, `poses`).

## Belongs here

Argument parsing, choosing what to run, reading and writing files, printing results.

## Does not belong here

| If the code... | It goes to |
|---|---|
| renders, composites or outlines pixels | `pixfx-draw` / `pixfx-core` |
| a file format (sheet layout, JSON, GIF, bake) | `pixfx-export` |
| an effect option's default or range | the effect (`pixfx-effects`) or `pixfx-core` |
| how a fighter is painted | `pixfx-fighter` |

If a second host (the site, a game) would want the same output, the logic is not CLI code.

## Dependencies

`pixfx-core`, `pixfx-draw`, `pixfx-effects`, `pixfx-export`, `pixfx-fighter`, `pixfx-pixel`, `gif`. Nothing
depends on this crate.

## Before you finish

```
cargo test -p pixfx-cli
cargo run -p pixfx-cli -- parity <id> --pixelfx /tmp/pixelfx --out target/parity   # when parity code changed
```
