use pixfx_core::{Layer, Options, PixelBox, Player};
use pixfx_draw::PixelDensity;
use pixfx_effects::EFFECTS;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test;

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn the_catalog_can_paint_on_a_dense_raster_at_the_same_game_size() {
    for &effect in EFFECTS {
        let options = Options {
            pixel_density: PixelDensity::One,
            ..Options::default()
        };
        let normal = Player::new(effect, options, 0);
        let mut dense = Player::new(effect, Options::default(), 0);
        assert_eq!(normal.canvas(), dense.canvas());
        assert_eq!(normal.frame().density(), PixelDensity::One);
        assert_eq!(dense.frame().density(), PixelDensity::Two);
        assert_eq!(dense.frame().w, normal.frame().w * 2);
        assert_eq!(dense.frame().h, normal.frame().h * 2);
        assert_ne!(
            normal.shown(normal.placement()),
            dense.shown(dense.placement())
        );

        let mut visible = false;
        for _ in 0..8 {
            dense.draw(Layer::Both, dense.placement());
            visible |= PixelBox::lit(dense.frame()).is_some();
            dense.update((effect.meta().duration / 8.0).min(0.25));
        }
        assert!(
            visible || effect.wraps_actor() || effect.meta().id == "projectiles/trail",
            "{} drew no dense pixels",
            effect.meta().id
        );
    }
}
