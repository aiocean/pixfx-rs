---
name: pixfx-analyze
description: Analyze a pixel-art game effect from a GIF, video, sprite sheet, screenshot, or live reference. Use for frame-by-frame VFX breakdowns, object and movement analysis, color and lighting separation, and deciding between pixfx paint, sprite, and hybrid. Covers requests such as "phân tích effect" and "hiệu ứng này làm paint hay sprite".
---

# Analyze a pixfx effect

Explain what the player sees, when each component changes, and how the reference could map onto pixfx. Analysis requests end with the breakdown. For an explicit implementation request, use the breakdown as input to the repo's `pixfx-effect` skill.

## Read the reference

- Inspect the whole composition first. Identify overlays, UI, camera movement, actors, and background that help explain the shot. Crop a fixed region around the full effect for detail, with room for the widest flash, trail, or debris.
- For a GIF, reconstruct full frames before cropping. Delta frames may contain only an update rectangle. Read frame delays and build timestamps from their cumulative sum. Keep authored holds and blank sections in the timeline. For video, use actual frame timestamps where available. For a still or a sheet with missing timing metadata, label timing and playback order as unknown when they cannot be established.
- Review a chronological contact sheet, then inspect adjacent frames around reveals, peaks, and transitions at native scale or nearest-neighbor enlargement. Sample long steady stretches and inspect short flashes densely. When referring to frames, state the index convention. Keep probes and crops in temporary storage unless the user requests an artifact.
- With ImageMagick available, these commands show the basic GIF probe. Work in a temporary directory. Replace the input path. To crop, insert `-crop WIDTHxHEIGHT+X+Y` before `+repage`. The contact sheet is row-major, starting at zero.

```sh
magick identify -format '%s %T\n' input.gif
magick input.gif -coalesce +repage frame-%04d.png
magick montage +set label frame-*.png -filter point \
  -tile 8x -geometry '160x160>+2+2' contact.png
```

ImageMagick's GIF delays above are in centiseconds. If other tools are available, use their equivalent frame and timing probes. If access or frame evidence is limited, state what was available and base the analysis on that evidence.

## Separate what changes

| Track | What to inspect |
|---|---|
| Object | Primary silhouette or symbol, parts, fill and edges, pivot and anchor. Identify which forms persist across phases and which briefly overlay them. |
| Movement | Track each form relative to its anchor. Distinguish translation, rotation, scale, and deformation from an advancing reveal, a highlight sweeping over a fixed shape, and an outline expanding around it. Note direction, holds, speed changes, and what leads or follows. |
| Secondary VFX | Treat trails, streaks, rings, glyphs, and particles as separate tracks where the frames support it. Describe where they appear, their shapes and trajectories, and when they fade or break up. Distinguish a readable symbol from a generic spark. |
| Lighting | Separate the base fill and rim from highlights, the flash core, the soft halo, and any visible light on nearby objects. Compare brightness, hue, and reach through the sequence. A bright cross or star has a silhouette and a glow, which can be controlled independently. |
| Staging and feel | Record visible overlaps and the main accent. Mention cues like shake, hit-stop, or sound only when evidenced, or label them as proposals. |

GIFs and videos generally contain the final composite. Describe observed pixels separately from inferred layer boundaries, blend modes, source assets, and authoring methods. Similar silhouettes across frames can support a reconstruction hypothesis without proving that the source used a shared sprite.

## Deliver the breakdown

Lead with the primary object and the motion that gives the effect its character. For animated references, use a compact phase timeline with frames or timestamps. Describe what changes for each track and which layers overlap. Include a contact sheet when it materially helps someone inspect the timing.

For pixfx, map silhouette, fill, and crisp shape detail to `color`, and emissions or spill to additive `light`. Explain where a feature needs both. Treat this as a proposed reconstruction. Size any proposed effect against `pixfx_draw::RULER`; reference image pixels may be display-scaled.

- **Paint** fits shapes, masks, paths, palette ramps, and motions that can be described with a small set of authored keys or procedural rules. Point out the silhouettes and beats that need manual tuning.
- **Sprite** fits authored frame sequences whose irregular silhouettes or texture are important to preserve. Consider which controls the game needs at runtime, such as recoloring, following a moving actor, or fitting its pose.
- **Hybrid** fits an authored shape or sequence combined with independent procedural tracks. Name exactly what would be sprite data and what Rust would paint.

Recommend a render kind for the user's goal. Only discuss measured performance when measurements exist.

### Example judgment

In a shield activation, two separated side contours fold toward the center and settle into a closed shield. A diagonal glint then crosses the settled face. The large cross briefly overlaps the shield and dissolves while the original silhouette is still visible. Treat the shield, the contour motion, the glint, the cross, and the sparks as tracks with separate timing. A stable shield mask and a few contour poses support `paint`, with the cross geometry and its halo controlled independently. An authored shield face combined with those procedural tracks would be `hybrid`.
