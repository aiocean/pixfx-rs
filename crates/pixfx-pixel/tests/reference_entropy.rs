use pixfx_pixel::reference::RgbaImage;
use pixfx_pixel::reference::entropy::{change, steps, symbols};

const W: u32 = 32;

/// A frame with the pixels `on` returns lit in grey `v`, the rest empty.
fn frame(on: impl Fn(u32, u32) -> Option<u8>) -> RgbaImage {
    let mut img = RgbaImage::new(W, W);
    for y in 0..W {
        for x in 0..W {
            let p = on(x, y).map_or([0, 0, 0, 255], |v| [v, v, v, 255]);
            img.set(x, y, p);
        }
    }
    img
}

fn square(side: u32, v: u8) -> RgbaImage {
    frame(move |x, y| (x < side && y < side).then_some(v))
}

#[test]
fn a_frame_that_follows_from_the_last_pixel_for_pixel_changes_nothing() -> Result<(), String> {
    let a = square(10, 200);
    assert_eq!(change(&symbols(&a), &symbols(&a))?, 0.0);
    // every bright pixel turned dim: still known from the first frame, place by place
    assert!(change(&symbols(&a), &symbols(&square(10, 60)))? < 1e-9);
    Ok(())
}

#[test]
fn change_grows_with_the_pixels_a_step_touches() -> Result<(), String> {
    let s = steps(&[square(8, 200), square(10, 200), square(20, 200)])?;
    assert_eq!(s.len(), 2);
    assert!(
        s[0] > 0.0 && s[1] > s[0],
        "growing by 36 pixels changes less than growing by 300: {s:?}"
    );
    Ok(())
}

#[test]
fn an_unrelated_frame_is_nearly_all_change() -> Result<(), String> {
    let noise = |seed: u32| {
        frame(move |x, y| {
            let h = (x.wrapping_mul(73_856_093)
                ^ y.wrapping_mul(19_349_663)
                ^ seed.wrapping_mul(83_492_791))
                % 5;
            (h > 0).then_some((h * 60) as u8)
        })
    };
    let c = change(&symbols(&noise(1)), &symbols(&noise(2)))?;
    assert!(c > 0.8, "{c}");
    Ok(())
}

#[test]
fn frames_of_different_sizes_are_refused() {
    let small = RgbaImage::new(4, 4);
    assert!(
        change(&symbols(&small), &symbols(&square(4, 200)))
            .is_err_and(|e| e.contains("cannot be compared"))
    );
}
