//! A fire or a thruster the game carries along (`carried: 1`) bends back against the way it moves, and paints
//! exactly as its source while it is not carried or stands still.

#[cfg(any(
    feature = "fire-campfire",
    feature = "fire-brazier",
    feature = "fire-candle",
    feature = "scifi-jetpack"
))]
mod carried {
    use std::error::Error;
    use std::sync::Arc;

    use pixfx_core::{Layer, OptValue, Options, Trace, TracePoint};
    use pixfx_draw::frame::{Frame, frame_hash};
    use pixfx_draw::painter::Painter;

    type Res = Result<(), Box<dyn Error>>;

    fn opts(carried: f64) -> Options {
        let mut o = Options::default();
        o.extra
            .insert("carried".to_string(), OptValue::Num(carried));
        o
    }

    /// A mover that went along +x at `speed` pixels a second, now at the origin.
    fn moving(speed: f64) -> Trace {
        Trace {
            points: (0..=20)
                .map(|i| {
                    let age = f64::from(i) * 0.01;
                    TracePoint {
                        x: -speed * age,
                        y: 0.0,
                        angle: 0.0,
                        age,
                    }
                })
                .collect(),
        }
    }

    /// Effect `id` drawn at `t` with `trace`: the frame, and the mean x (from the origin) of its opaque pixels on the
    /// rows `rows` keeps (the flames), y counted down from its origin.
    fn drawn(
        id: &str,
        o: &Options,
        trace: Option<Trace>,
        rows: impl Fn(i32) -> bool,
    ) -> Result<(Frame, f64), Box<dyn Error>> {
        let fx = pixfx_effects::effect(id).ok_or("effect not built")?;
        let mut st = fx.spawn(o, 1);
        st.set_trace(trace.map(Arc::new));
        let mut f = Frame::new(240, 240, 0, 0);
        let mut p = Painter::new(&mut f);
        p.place(120.0, 180.0, false, 0.0);
        st.draw(&mut p, 0.5, Layer::Front);
        let (mut sum, mut n) = (0.0, 0.0);
        for (i, &c) in f.color.iter().enumerate() {
            let (x, y) = (i as i32 % 240 - 120, i as i32 / 240 - 180);
            if c >> 24 != 0 && rows(y) {
                sum += f64::from(x);
                n += 1.0;
            }
        }
        Ok((f, if n > 0.0 { sum / n } else { 0.0 }))
    }

    fn leans_back(id: &str, rows: impl Fn(i32) -> bool + Copy) -> Res {
        let fx = pixfx_effects::effect(id).ok_or("effect not built")?;
        assert_eq!(
            fx.traces(&Options::default()),
            None,
            "{id} asks for its motion unasked"
        );
        assert!(
            fx.traces(&opts(1.0)).is_some(),
            "{id} carried asks for no motion"
        );
        let (still, x0) = drawn(id, &Options::default(), None, rows)?;
        // carried but standing still, or never handed a trace: the source's pixels
        let (rest, _) = drawn(id, &opts(1.0), Some(moving(0.0)), rows)?;
        assert_eq!(
            frame_hash(&rest),
            frame_hash(&still),
            "{id} standing still changed"
        );
        let (_, right) = drawn(id, &opts(1.0), Some(moving(250.0)), rows)?;
        let (_, left) = drawn(id, &opts(1.0), Some(moving(-250.0)), rows)?;
        assert!(
            right < x0 - 1.0,
            "{id} carried right leans {right} against {x0} at rest"
        );
        assert!(
            left > x0 + 1.0,
            "{id} carried left leans {left} against {x0} at rest"
        );
        Ok(())
    }

    #[cfg(feature = "fire-campfire")]
    #[test]
    fn a_carried_campfire_leans_against_its_motion() -> Res {
        leans_back("fire/campfire", |y| y < -10)
    }

    #[cfg(feature = "fire-candle")]
    #[test]
    fn a_carried_candle_leans_against_its_motion() -> Res {
        leans_back("fire/candle", |y| y < -20)
    }

    #[cfg(feature = "fire-brazier")]
    #[test]
    fn a_carried_brazier_leans_against_its_motion() -> Res {
        leans_back("fire/brazier", |y| y < -40)
    }

    #[cfg(feature = "scifi-jetpack")]
    #[test]
    fn a_carried_jetpack_leans_its_plumes_against_its_motion() -> Res {
        // the plumes, below the nozzles
        leans_back("scifi/jetpack", |y| (10..30).contains(&y))
    }
}
