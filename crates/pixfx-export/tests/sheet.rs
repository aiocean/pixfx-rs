//! `write_sheet` on a real effect: the files it writes and the beanfx JSON an engine reads.

use std::fs;
use std::io::BufReader;
use std::path::{Path, PathBuf};

use pixfx_core::{Options, render_frames};
use pixfx_export::write_sheet;

type Test = Result<(), Box<dyn std::error::Error>>;

fn png_size(path: &Path) -> Result<(u32, u32), Box<dyn std::error::Error>> {
    let dec = png::Decoder::new(BufReader::new(fs::File::open(path)?));
    let reader = dec.read_info()?;
    let info = reader.info();
    Ok((info.width, info.height))
}

fn tmp(name: &str) -> Result<PathBuf, std::io::Error> {
    let dir = std::env::temp_dir().join(format!("pixfx-export-{name}-{}", std::process::id()));
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

#[test]
fn candle_sheet_layout() -> Test {
    let effect = pixfx_effects::effect("fire/candle").ok_or("no candle")?;
    let r = render_frames(effect, &Options::default(), 0);
    let dir = tmp("candle")?;
    let files = write_sheet(&r, effect.meta(), &dir, "candle")?;

    let json: serde_json::Value = serde_json::from_str(&fs::read_to_string(&files.json)?)?;
    assert_eq!(json["frames"].as_array().ok_or("frames")?.len(), 120);
    assert_eq!(
        json["frames"][9]["frame"],
        serde_json::json!({"x": 56, "y": 76, "w": 56, "h": 76})
    );
    assert_eq!(json["frames"][0]["duration"], 33);
    assert_eq!(json["meta"]["image"], "candle.png");
    assert_eq!(
        json["meta"]["size"],
        serde_json::json!({"w": 448, "h": 1140})
    );
    let bfx = &json["meta"]["beanfx"];
    assert_eq!(bfx["columns"], 8);
    assert_eq!(bfx["rows"], 1);
    assert_eq!(bfx["frames"], 120);
    assert_eq!(bfx["frameSize"], serde_json::json!([56, 76]));
    assert_eq!(bfx["pivot"], serde_json::json!([28, 64]));
    assert_eq!(bfx["end"], "loop");
    assert_eq!(bfx["loop"], true);
    assert_eq!(bfx["blend_light"], "add");
    assert_eq!(bfx["light_gain"], 1.0);
    assert_eq!(bfx["sheet"], "candle.png");
    assert_eq!(bfx["color"], "candle_color.png");
    assert_eq!(bfx["light"], "candle_light.png");

    for p in [&files.sheet, &files.color, &files.light] {
        assert_eq!(png_size(p)?, (448, 1140), "{}", p.display());
    }
    fs::remove_dir_all(dir)?;
    Ok(())
}

#[test]
fn margin_moves_the_pivot() -> Test {
    let effect = pixfx_effects::effect("fire/candle").ok_or("no candle")?;
    let r = render_frames(effect, &Options::default(), 6);
    let dir = tmp("margin")?;
    let files = write_sheet(&r, effect.meta(), &dir, "candle")?;
    let json: serde_json::Value = serde_json::from_str(&fs::read_to_string(&files.json)?)?;
    assert_eq!(json["meta"]["beanfx"]["pivot"], serde_json::json!([34, 70]));
    assert_eq!(
        json["meta"]["beanfx"]["frameSize"],
        serde_json::json!([68, 88])
    );
    fs::remove_dir_all(dir)?;
    Ok(())
}
