# Motion and sweep: one way for everything that moves to leave a streak

Date: 2026-11-16. Status: proposed, waiting for the owner's approval.

## Why

Many effects leave a streak or react to how they move: a flying sword's trail, a torch's flame, sparks, a slash, a
spinning blade. Each one works out its own velocity and draws its own streak, so they disagree. The owner wants one
formula, read the way a camera sees a burning thing waved in a long exposure:

- a point swept along a path leaves a line;
- a short column of pixels swept across the motion leaves a ribbon (the flying sword's trail today);
- a long bar that turns while it moves leaves an area (a fan, an arc smear, a disc).

So a streak is a **brush** (a column of pixels, each with a colour and a light) swept along **poses**, coloured by how
long ago it passed. The sword sweeps a small white-to-steel column; a fire skill sweeps a white, yellow, orange, red
column; another skill another column.

## Goals

- `pixfx_core::motion`: the derived quantities of a moving thing, one tested implementation.
- `pixfx_draw::sweep`: the swept area of a brush along poses, gap-free, coloured by age.
- A brush from code (cells built from a palette and a width, or written out) or read from a sprite's column
  (`Brush::from_column(frame, x)`), so a streak can take the object's exact colours.
- `projectiles/trail` rebuilt on these: a brush and a look as options.
- The fire family reads its motion: a flame carried fast bends back and its sparks blow back.

## Non-goals

- Stamping a mover's whole painted frame at every past pose (a full long exposure): it needs another effect's frames
  while painting in workers, and costs much more. The sprite-column brush covers what we need of it.
- A scene-wide tilt of finished frames.

## Anti-goals

- A port whose pixels change when it is not moving: parity and golden hashes stay as they are.
- A streak with holes when the brush moves fast or turns.

## Design

### Poses and motion (`pixfx-core`)

- `TracePoint` gains `angle` (radians): the mover's rotation when it was there. The scene already holds each
  instance's angle (`Placement::angle`, `crates/pixfx-core/src/effect.rs:459`); `Scene::follow` records it with
  the position, mirrored like x when the trail draws flipped.
- `Motion` (`pixfx_core::motion`), pure functions over a trace, newest first:
  - `at(age)`: position and angle at any age, interpolated;
  - `velocity(age, window)`, `speed`, `heading`: from the positions `window` seconds around `age`
    (a central difference, so one jittery sample does not swing it);
  - `turn(age, window)`: how fast the heading changes (radians a second); `spin`: how fast the angle changes;
  - `acceleration(age, window)`.
  A still or one-point trace gives zero velocity and no heading, never NaN.
- An effect computing its own path (a slash's arc, a spark's flight, a preview) builds the same pose list and uses
  the same functions.

### The brush (`pixfx-draw`)

- `Brush { cells: Vec<Cell { across: f64, color: Rgba, light: f64 }>, frame: BrushFrame }`:
  - `across`: the cell's offset in pixels from the path;
  - `frame`: `Heading` (the column stays square to the way it moves: a ribbon) or `Angle(base)` (the column turns
    with the mover's own angle: a bar fixed to a blade, sweeping arcs).
- Builders: `Brush::column(palette, width)` (centre the first colour, the edges the last), and
  `Brush::from_column(frame, x)` (the opaque pixels of column `x` of a painted frame, colour and light, centred on
  the frame's origin row).

### The sweep (`pixfx-draw`)

`sweep(painter, poses, brush, look)`:

- walks the poses oldest to newest, so the newest pass wins, as on film;
- between two poses fills the quad each pair of neighbouring cells sweeps (scanline, whole pixels), sub-stepping
  so no cell moves more than one pixel per step, including a long bar's far end when it turns: no holes;
- the look is a set of plain functions of age (0 newest to 1 oldest) and of the distance along the path:
  - `ramp`: how far along the brush's colours toward the last one the colour has moved with age;
  - `taper`: the brush's width with age (a ribbon thinning to one pixel);
  - `wave`: offset across the path, with the distance and the frame;
  - `breakup`: the age at which an ordered dither starts eating the streak, and how fast;
  - `light`: how much each cell adds to the light layer, falling with age.

### The trail on top

`projectiles/trail` becomes `sweep` with `Brush::column(palette, width)` and the look from its options (`secs`,
`width`, `wave`, `skip`, palette, plus `brush`: `heading` or `angle`). The flying sword's trail looks as it does now
(checked side by side before blessing). The sword formation can pass a brush read from the sword's blade column.

### Fire reads its motion

The fire family's `lean` and `wind` (`crates/pixfx-effects/src/fire/lib.rs:469`, `:486`) gain the motion's
velocity when the effect is moved (it asks for a short trace through `Effect::traces`): the flame bends against the
motion, sparks drift back. Standing still, the motion is zero and the pixels are the port's: parity and goldens
unchanged. The spark streak (`fire/lib.rs:351`) is a port and stays as it is; new effects draw sparks with `sweep`.

## Cost

- An effect that reads a live trace paints its own frames, not shared ones (as trails do now). Opt-in per effect;
  bench (`cargo run --release -p pixfx-effects --features <it> --example bench`) before and after.
- A sweep is linear in path length times brush cells: a 45-pixel trail of 3 cells is about 135 cell steps a frame.

## Tests

- `motion`: a straight constant-speed trace gives that speed and heading; a circle gives a constant turn; a still
  trace gives zero, no NaN.
- `sweep`: a point swept along a line covers exactly the line; a column swept straight covers a band; a bar turned
  about its end covers a fan with no hole (every pixel inside the fan's outline is painted); newest pass on top.
- `Brush::from_column` reads a known frame's column.
- Trail: the scene tests already there (straight, bending, longer when fast, on its own); its look beside today's.
- Fire: goldens unchanged with all effects built; a moved campfire leans against its motion.

## Left to the builder

The exact filling rule at quad edges, the sub-step bound, and the default window of `velocity`.
