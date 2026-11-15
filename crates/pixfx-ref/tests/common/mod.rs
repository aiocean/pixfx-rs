//! The hand-made sheet the tests read: the template's labels in near-black on a transparent page, and in one cell
//! an opaque block, checkered in two colours so its pixels show, with a see-through glow beside it.

use pixfx_pixel::sheet::cell_rect;
use pixfx_ref::template::{labels, layout};
use pixfx_ref::{RgbaImage, Spec};

pub const SPEC: &str = r#"{"id":"t/x","name":"X","look":"l","subject":"s","rows":[
  {"name":"INIT","caption":"forms","text":"a"},{"name":"HIT RIGHT","caption":"","text":"b","frames":2}]}"#;

pub const BLOCK: [u8; 4] = [200, 120, 40, 255];
pub const BLOCK_DARK: [u8; 4] = [90, 40, 10, 255];
pub const GLOW: [u8; 4] = [80, 160, 255, 128];

pub fn sheet(spec: &Spec) -> Result<RgbaImage, String> {
    let l = layout(spec);
    let (w, h) = pixfx_pixel::sheet::size(&l);
    let mut img = RgbaImage::new(w, h);
    let names = labels(spec);
    let texts: Vec<&str> = names.iter().map(String::as_str).collect();
    pixfx_pose::captions(&mut img, &l, &texts)?;
    // cell (1, 0): sprite pixels 40..56 a block, 60..70 glow, both on rows 40..56
    let [x0, y0, _, _] = cell_rect(&l, 1, 0);
    let s = l.scale;
    for sy in 40..56 {
        for sx in 40..70 {
            let checker = if (sx + sy) % 2 == 0 { BLOCK } else { BLOCK_DARK };
            let p = if sx < 56 { checker } else if sx >= 60 { GLOW } else { [0; 4] };
            for dy in 0..s {
                for dx in 0..s {
                    img.set(x0 + sx * s + dx, y0 + sy * s + dy, p);
                }
            }
        }
    }
    Ok(img)
}
