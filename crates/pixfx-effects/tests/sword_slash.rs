//! The sword slash cuts where it is told: its impact cue fires as the blade passes through `hitX`, `hitY`, and the
//! blade leaves a crescent behind it only once the cut has begun.

#[cfg(feature = "arcane-sword-slash")]
mod sword_slash {
    use std::error::Error;

    use pixfx_core::{Layer, OptValue, Options};
    use pixfx_draw::frame::Frame;
    use pixfx_draw::painter::Painter;

    type Res = Result<(), Box<dyn Error>>;

    const ID: &str = "arcane/sword-slash";

    fn opts(hit: (f64, f64)) -> Options {
        let mut o = Options::default();
        o.extra.insert("hitX".to_string(), OptValue::Num(hit.0));
        o.extra.insert("hitY".to_string(), OptValue::Num(hit.1));
        o
    }

    /// The frame at `t`, with the origin at (120, 160).
    fn at(o: &Options, t: f64) -> Result<Frame, Box<dyn Error>> {
        let fx = pixfx_effects::effect(ID).ok_or("effect not built")?;
        let st = fx.spawn(o, 1);
        let mut f = Frame::new(240, 240, 0, 0);
        let mut p = Painter::new(&mut f);
        p.place(120.0, 160.0, false, 0.0);
        st.draw(&mut p, t, Layer::Front);
        Ok(f)
    }

    /// Opaque pixels within `r` of local point (x, y).
    fn near(f: &Frame, x: f64, y: f64, r: i32) -> usize {
        let (cx, cy) = (120 + x as i32, 160 + y as i32);
        let mut n = 0;
        for dy in -r..=r {
            for dx in -r..=r {
                let i = ((cy + dy) * 240 + cx + dx) as usize;
                if f.color.get(i).is_some_and(|c| c >> 24 != 0) {
                    n += 1;
                }
            }
        }
        n
    }

    fn cue_time(o: &Options) -> Result<f64, Box<dyn Error>> {
        let fx = pixfx_effects::effect(ID).ok_or("effect not built")?;
        Ok(pixfx_core::cue::cues_of(fx, o).first().ok_or("no cue")?.t)
    }

    #[test]
    fn the_cue_fires_as_the_blade_passes_through_the_hit_point() -> Res {
        for hit in [(0.0, -22.0), (-20.0, -40.0), (25.0, -10.0)] {
            let o = opts(hit);
            let t = cue_time(&o)?;
            assert!(
                near(&at(&o, t)?, hit.0, hit.1, 1) > 0,
                "nothing drawn at {hit:?} as its cue fires"
            );
            // long before the cut the blade is up behind the guard, nowhere near the hit point
            assert_eq!(
                near(&at(&o, 0.2)?, hit.0, hit.1, 1),
                0,
                "something at {hit:?} before the cut"
            );
        }
        Ok(())
    }

    #[test]
    fn the_blade_leaves_a_crescent_only_once_the_cut_has_begun() -> Res {
        let o = opts((0.0, -22.0));
        let t = cue_time(&o)?;
        let count = |f: &Frame| f.color.iter().filter(|&&c| c >> 24 != 0).count();
        // the blade alone while winding back, much more once it sweeps
        let still = count(&at(&o, 0.28)?);
        let cutting = count(&at(&o, t)?);
        assert!(
            cutting > still * 2,
            "{cutting} pixels as it cuts against {still} winding back"
        );
        Ok(())
    }
}
