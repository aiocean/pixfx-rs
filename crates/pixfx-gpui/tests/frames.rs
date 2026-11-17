//! Views painted into one window upload a frame once and share it; a frame is rasterised again only when an
//! effect reaches a new one.

use std::cell::RefCell;
use std::rc::Rc;

use gpui::{Styled, TestAppContext, VisualTestContext, canvas, point, px, size};
use pixfx_core::{Layer, Options};
use pixfx_gpui::{FxHandle, FxScene};

struct Site {
    effects: Vec<FxHandle>,
    scene: FxScene,
}

/// Paint every view into the window once, as a window frame does, and return the bytes the frames hold.
fn paint(cx: &mut VisualTestContext, site: &Rc<RefCell<Site>>) -> u64 {
    let painting = site.clone();
    cx.draw(
        point(px(0.), px(0.)),
        size(px(400.), px(300.)),
        move |_, _| {
            canvas(
                |_, _, _| {},
                move |_, _, window, _| {
                    let mut site = painting.borrow_mut();
                    let Site { effects, scene } = &mut *site;
                    for &fx in effects.iter() {
                        assert!(
                            scene
                                .paint(fx, Layer::Both, point(px(100.), px(100.)), 4, window)
                                .is_ok()
                        );
                    }
                    assert!(scene.end_frame(window).is_ok());
                },
            )
            .size_full()
        },
    );
    site.borrow().scene.bytes()
}

fn site(ids: &[&str]) -> Rc<RefCell<Site>> {
    let mut scene = FxScene::default();
    let effects = ids
        .iter()
        .filter_map(|id| pixfx_effects::effect(id))
        .filter_map(|fx| scene.play(fx, Options::default()).ok())
        .collect::<Vec<_>>();
    assert_eq!(effects.len(), ids.len(), "every effect exists");
    Rc::new(RefCell::new(Site { effects, scene }))
}

/// Let `dt` seconds pass for every effect of `site`.
fn tick(site: &Rc<RefCell<Site>>, dt: f64) {
    assert!(site.borrow_mut().scene.tick(dt).is_ok());
}

#[gpui::test]
fn identical_views_share_one_uploaded_frame(cx: &mut TestAppContext) {
    let cx = cx.add_empty_window();
    let one = paint(cx, &site(&["feedback/power-up"]));
    assert!(one > 0, "the power-up draws something on its first frame");
    let many = paint(cx, &site(&["feedback/power-up"; 20]));
    assert_eq!(many, one, "twenty power-ups hold the bytes of one");
    let two = paint(cx, &site(&["feedback/power-up", "explosions/body-burst"]));
    assert!(two > one, "another effect holds its own frame");
}

#[gpui::test]
fn a_frame_is_uploaded_only_when_the_clock_reaches_a_new_one(cx: &mut TestAppContext) {
    let cx = cx.add_empty_window();
    let site = site(&["feedback/power-up"]);
    let first = paint(cx, &site);
    // well within one frame of 1/60 s: the same frame, nothing new uploaded
    tick(&site, 0.001);
    assert_eq!(paint(cx, &site), first);
    tick(&site, 0.04);
    assert!(
        paint(cx, &site) > first,
        "the next frame is uploaded beside the first"
    );
}

#[gpui::test]
fn frames_past_the_budget_leave_the_atlas_and_a_fading_view_holds_its_own(cx: &mut TestAppContext) {
    let cx = cx.add_empty_window();
    let site = site(&["feedback/power-up"]);
    site.borrow_mut().scene.set_budget(0);
    for _ in 0..5 {
        paint(cx, &site);
        tick(&site, 0.04);
    }
    // over a zero budget only the frame painted last stays: the next draw drops it
    let shown = paint(cx, &site);
    assert!(shown > 0);
    {
        let mut site = site.borrow_mut();
        let Site { effects, scene } = &mut *site;
        effects.iter().for_each(|&fx| {
            scene.stop(fx);
        });
    }
    tick(&site, 0.04);
    assert_eq!(
        paint(cx, &site),
        0,
        "a fading view paints its own images, not shared ones"
    );
}

#[gpui::test]
fn a_draw_budget_uploads_that_many_new_frames_per_window_frame(cx: &mut TestAppContext) {
    let cx = cx.add_empty_window();
    let mut scene = FxScene::default();
    let effects = (0..3)
        .filter_map(|seed| {
            let fx = pixfx_effects::effect("feedback/power-up")?;
            let opts = Options {
                seed: Some(100 + seed),
                ..Options::default()
            };
            scene.play(fx, opts).ok()
        })
        .collect::<Vec<_>>();
    assert_eq!(effects.len(), 3);
    scene.set_draw_budget(std::num::NonZeroU32::new(1));
    let site = Rc::new(RefCell::new(Site { effects, scene }));
    // three power-ups with their own seeds share nothing: one new frame per window frame until all three show
    let bytes: Vec<u64> = (0..4).map(|_| paint(cx, &site)).collect();
    assert!(
        bytes[0] > 0 && bytes[1] > bytes[0] && bytes[2] > bytes[1],
        "{bytes:?}"
    );
    assert_eq!(bytes[3], bytes[2], "nothing left to draw: {bytes:?}");
}
