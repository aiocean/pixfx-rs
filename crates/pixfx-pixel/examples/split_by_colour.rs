//! A script an agent writes when no verb fits: recover a picture's pixel grid, then list its connected regions
//! (box, cells, how many colours) to start a parts spec from.
//!
//!   cargo run -p pixfx-pixel --example split_by_colour -- <picture.png>

use pixfx_pixel::parts::components;
use pixfx_pixel::recover::recover;
use pixfx_pixel::reduce::reduce;
use pixfx_pixel::{colors, nearest, shrink};

fn main() -> Result<(), String> {
    let path = std::env::args()
        .nth(1)
        .ok_or("usage: split_by_colour <picture.png>")?;
    let bytes = std::fs::read(&path).map_err(|e| format!("{path}: {e}"))?;
    let found = recover(&pixfx_export::decode_png_rgba(&bytes)?);
    let sprite = reduce(&pixfx_pixel::cut(&found.picture, 40), 32, 0.03);
    let palette = colors(&sprite);
    let map = shrink(&nearest(&sprite, &palette), sprite.w, 1.0);
    println!(
        "pitch {:.2}, {}x{} cells, {} colours",
        found.pitch,
        map.w,
        map.h,
        palette.len()
    );
    for c in components(&map) {
        let [x, y, w, h] = c.bbox;
        println!(
            "{x},{y},{w},{h}  {} cells  {} colours",
            c.cells,
            c.colors.len()
        );
    }
    Ok(())
}
