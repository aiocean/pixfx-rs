//! A rendered clip holds exactly the frames live play shows, under the key live play shows them by.

use std::path::PathBuf;

use pixfx_core::{Layer, Options, Player};
use pixfx_draw::{Frame, PixelDensity};
use pixfx_export::clip::{ClipIndex, decode};
use pixfx_export::{render_clip, write_clip};

fn out_dir(test: &str) -> PathBuf {
    std::env::temp_dir().join(format!("pixfx-clip-{test}-{}", std::process::id()))
}

/// Equal sizes, colour and light; a light word's top byte is unused (`composite_px` masks it off).
fn same_picture(a: &Frame, b: &Frame) -> bool {
    let rgb = |f: &Frame| f.light.iter().map(|l| l & 0x00ff_ffff).collect::<Vec<_>>();
    (a.w, a.h, a.ox, a.oy) == (b.w, b.h, b.ox, b.oy)
        && a.density() == b.density()
        && a.color == b.color
        && rgb(a) == rgb(b)
}

/// Render `id`'s clip, write it, read the files back and decode them.
fn round_trip(id: &str, test: &str, pixel_density: PixelDensity) -> Result<(), String> {
    let fx = pixfx_effects::effect(id).ok_or(format!("no effect {id}"))?;
    let options = Options {
        pixel_density,
        ..Options::default()
    };
    let clip = render_clip(fx, options.clone())?;
    let dir = out_dir(test);
    let files = write_clip(&clip, &dir, "clip").map_err(|e| e.to_string())?;
    let json = std::fs::read_to_string(&files.json).map_err(|e| e.to_string())?;
    let pages = files
        .pages
        .iter()
        .map(std::fs::read)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    let page_refs: Vec<&[u8]> = pages.iter().map(Vec::as_slice).collect();
    let layers = decode(&json, &page_refs)?;
    std::fs::remove_dir_all(&dir).map_err(|e| e.to_string())?;

    // live play, stepped by real-looking uneven steps, shows the rendered frame for every index it reaches
    let mut live = Player::live(fx, options);
    let place = live.placement();
    let mut checked = 0;
    for step in [0.0, 0.013, 0.021, 0.034, 0.008].iter().cycle().take(400) {
        live.update(*step);
        let Some(shown) = live.shown(place) else {
            break;
        };
        for (side, layer) in [Layer::Back, Layer::Front].into_iter().enumerate() {
            let rendered = usize::try_from(shown.index)
                .ok()
                .and_then(|k| layers.get(k * 2 + side))
                .ok_or(format!("{id}: no rendered frame {}", shown.index))?;
            if rendered.id != shown || rendered.layer != layer {
                return Err(format!(
                    "{id}: rendered frame {} is not keyed as live play keys it",
                    shown.index
                ));
            }
            live.draw(layer, place);
            if !same_picture(&rendered.frame, live.frame()) {
                return Err(format!(
                    "{id}: rendered frame {} {layer:?} differs from live",
                    shown.index
                ));
            }
            checked += 1;
        }
        if live.done() {
            break;
        }
    }
    if checked < 20 {
        return Err(format!("{id}: only {checked} frames checked"));
    }
    Ok(())
}

#[test]
fn a_looping_effect_renders_to_the_frames_live_play_shows() -> Result<(), String> {
    round_trip("projectiles/trail", "loop", PixelDensity::One)
}

#[test]
fn a_one_shot_renders_to_the_frames_live_play_shows() -> Result<(), String> {
    round_trip("explosions/body-burst", "shot", PixelDensity::One)
}

#[test]
fn a_dense_clip_keeps_its_raster_and_live_cache_keys() -> Result<(), String> {
    round_trip("feedback/violet-portal", "dense", PixelDensity::Two)
}

#[test]
fn a_clip_index_of_another_version_or_missing_pages_is_refused() -> Result<(), String> {
    let fx = pixfx_effects::effect("projectiles/trail").ok_or("no trail")?;
    let clip = render_clip(fx, Options::default())?;
    let (index, _) = pixfx_export::clip::pack(&clip, "trail");
    let json = serde_json::to_string(&index).map_err(|e| e.to_string())?;
    assert!(decode(&json, &[]).is_err(), "pages missing");
    let old = serde_json::to_string(&ClipIndex {
        version: 0,
        ..index
    })
    .map_err(|e| e.to_string())?;
    assert!(decode(&old, &[]).is_err(), "another version");
    Ok(())
}
