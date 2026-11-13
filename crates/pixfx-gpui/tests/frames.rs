//! Views painted into one window upload a frame once and share it; a frame is rasterised again only when an
//! effect reaches a new one.

use std::cell::RefCell;
use std::rc::Rc;

use gpui::{Styled, TestAppContext, VisualTestContext, canvas, point, px, size};
use pixfx_core::{Layer, Options};
use pixfx_gpui::{FxFrames, FxView, paint_fx};

struct Site {
    views: Vec<FxView>,
    frames: FxFrames,
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
                    let Site { views, frames } = &mut *site;
                    for view in views.iter_mut() {
                        assert!(
                            paint_fx(
                                view,
                                frames,
                                Layer::Both,
                                point(px(100.), px(100.)),
                                4,
                                window
                            )
                            .is_ok()
                        );
                    }
                    assert!(frames.end_frame(window).is_ok());
                },
            )
            .size_full()
        },
    );
    site.borrow().frames.bytes()
}

fn site(ids: &[&str]) -> Rc<RefCell<Site>> {
    let views = ids
        .iter()
        .filter_map(|id| pixfx_effects::effect(id))
        .map(|fx| FxView::new(fx, Options::default()))
        .collect::<Vec<_>>();
    assert_eq!(views.len(), ids.len(), "every effect exists");
    Rc::new(RefCell::new(Site {
        views,
        frames: FxFrames::default(),
    }))
}

#[gpui::test]
fn identical_views_share_one_uploaded_frame(cx: &mut TestAppContext) {
    let cx = cx.add_empty_window();
    let one = paint(cx, &site(&["fire/campfire"]));
    assert!(one > 0, "the campfire draws something on its first frame");
    let many = paint(cx, &site(&["fire/campfire"; 20]));
    assert_eq!(many, one, "twenty campfires hold the bytes of one");
    let two = paint(cx, &site(&["fire/campfire", "fire/candle"]));
    assert!(two > one, "another effect holds its own frame");
}

#[gpui::test]
fn a_frame_is_uploaded_only_when_the_clock_reaches_a_new_one(cx: &mut TestAppContext) {
    let cx = cx.add_empty_window();
    let site = site(&["fire/campfire"]);
    let first = paint(cx, &site);
    // well within one frame of 1/30 s: the same frame, nothing new uploaded
    site.borrow_mut()
        .views
        .iter_mut()
        .for_each(|v| v.tick(0.001));
    assert_eq!(paint(cx, &site), first);
    site.borrow_mut()
        .views
        .iter_mut()
        .for_each(|v| v.tick(0.04));
    assert!(
        paint(cx, &site) > first,
        "the next frame is uploaded beside the first"
    );
}

#[gpui::test]
fn frames_past_the_budget_leave_the_atlas_and_a_fading_view_holds_its_own(cx: &mut TestAppContext) {
    let cx = cx.add_empty_window();
    let site = site(&["fire/campfire"]);
    site.borrow_mut().frames.set_budget(0);
    for _ in 0..5 {
        paint(cx, &site);
        site.borrow_mut()
            .views
            .iter_mut()
            .for_each(|v| v.tick(0.04));
    }
    // over a zero budget only the frame painted last stays: the next draw drops it
    let shown = paint(cx, &site);
    assert!(shown > 0);
    site.borrow_mut().views.iter_mut().for_each(|v| v.stop());
    site.borrow_mut()
        .views
        .iter_mut()
        .for_each(|v| v.tick(0.04));
    assert_eq!(
        paint(cx, &site),
        0,
        "a fading view paints its own images, not shared ones"
    );
}
