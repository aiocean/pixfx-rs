use pixfx_core::Rendered;
use pixfx_draw::{Frame, Rgba};
use pixfx_export::board::{busiest, still};

fn render(painted: &[usize]) -> Rendered {
    let frames = painted
        .iter()
        .map(|&n| {
            let mut f = Frame::new(4, 2, 0, 0);
            for c in f.color.iter_mut().take(n) {
                *c = Rgba(255, 0, 0, 255).pack();
            }
            f
        })
        .collect();
    Rendered {
        w: 4,
        h: 2,
        fps: 30,
        origin: (0, 0),
        margin: 0,
        frames,
    }
}

#[test]
fn the_busiest_frame_is_the_one_with_most_pixels_painted_first_on_a_tie() {
    assert_eq!(busiest(&render(&[1, 5, 3, 5])), Some(1));
    assert_eq!(busiest(&render(&[])), None);
}

#[test]
fn a_still_is_the_frame_over_the_backdrop_each_pixel_blown_up() -> Result<(), String> {
    let r = render(&[2]);
    let img = still(&r, 0, 3).ok_or("frame 0 exists")?;
    assert_eq!((img.w, img.h), (12, 6));
    assert_eq!(
        img.get(5, 2),
        Some([255, 0, 0, 255]),
        "pixel (1, 0), three times as big"
    );
    assert_ne!(
        img.get(6, 2),
        Some([255, 0, 0, 255]),
        "pixel (2, 0) is backdrop"
    );
    assert!(still(&r, 1, 3).is_none());
    Ok(())
}
