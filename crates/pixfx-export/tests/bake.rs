//! A baked clip holds exactly the frames live play shows, under the key live play shows them by.

use std::path::PathBuf;

use pixfx_core::{Frame, Layer, Options, Player};
use pixfx_export::bake::{BakeIndex, decode};
use pixfx_export::{bake, write_bake};

fn out_dir(test: &str) -> PathBuf {
    std::env::temp_dir().join(format!("pixfx-bake-{test}-{}", std::process::id()))
}

/// Equal sizes, colour and light; a light word's top byte is unused (`composite_px` masks it off).
fn same_picture(a: &Frame, b: &Frame) -> bool {
    let rgb = |f: &Frame| f.light.iter().map(|l| l & 0x00ff_ffff).collect::<Vec<_>>();
    (a.w, a.h, a.ox, a.oy) == (b.w, b.h, b.ox, b.oy) && a.color == b.color && rgb(a) == rgb(b)
}

/// Bake `id`, write it, read the files back and decode them.
fn round_trip(id: &str, test: &str) -> Result<(), String> {
    let fx = pixfx_effects::effect(id).ok_or(format!("no effect {id}"))?;
    let clip = bake(fx, Options::default())?;
    let dir = out_dir(test);
    let files = write_bake(&clip, &dir, "clip").map_err(|e| e.to_string())?;
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

    // live play, stepped by real-looking uneven steps, shows the baked frame for every index it reaches
    let mut live = Player::live(fx, Options::default());
    let place = live.placement();
    let mut checked = 0;
    for step in [0.0, 0.013, 0.021, 0.034, 0.008].iter().cycle().take(400) {
        live.update(*step);
        let Some(shown) = live.shown(place) else {
            break;
        };
        for (side, layer) in [Layer::Back, Layer::Front].into_iter().enumerate() {
            let baked = usize::try_from(shown.index)
                .ok()
                .and_then(|k| layers.get(k * 2 + side))
                .ok_or(format!("{id}: no baked frame {}", shown.index))?;
            if baked.id != shown || baked.layer != layer {
                return Err(format!(
                    "{id}: baked frame {} is not keyed as live play keys it",
                    shown.index
                ));
            }
            live.draw(layer, place);
            if !same_picture(&baked.frame, live.frame()) {
                return Err(format!(
                    "{id}: baked frame {} {layer:?} differs from live",
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
fn a_looping_effect_bakes_to_the_frames_live_play_shows() -> Result<(), String> {
    round_trip("fire/campfire", "loop")
}

#[test]
fn a_one_shot_bakes_to_the_frames_live_play_shows() -> Result<(), String> {
    round_trip("explosions/small-explosion", "shot")
}

#[test]
fn a_bake_index_of_another_version_or_missing_pages_is_refused() -> Result<(), String> {
    let fx = pixfx_effects::effect("fire/candle").ok_or("no candle")?;
    let clip = bake(fx, Options::default())?;
    let (index, _) = pixfx_export::bake::pack(&clip, "candle");
    let json = serde_json::to_string(&index).map_err(|e| e.to_string())?;
    assert!(decode(&json, &[]).is_err(), "pages missing");
    let old = serde_json::to_string(&BakeIndex {
        version: 0,
        ..index
    })
    .map_err(|e| e.to_string())?;
    assert!(decode(&old, &[]).is_err(), "another version");
    Ok(())
}
