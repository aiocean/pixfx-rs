//! A window whose frames are painted on native threads shows them once they come back, shares them like frames
//! painted on the spot, and spends less of its own thread per frame. `bench` measures it:
//! `cargo test --release --manifest-path crates/pixfx-gpui/Cargo.toml --test threads -- --ignored --nocapture`.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use gpui::{Styled, TestAppContext, VisualTestContext, canvas, point, px, size};
use pixfx_core::{Layer, Options};
use pixfx_gpui::{FxHandle, FxScene};

struct Site {
    effects: Vec<FxHandle>,
    scene: FxScene,
}

/// Paint every effect into the window once, as a window frame does; returns the bytes the frames hold.
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

/// `n` effects of `ids` in turn, seeded `seed(i)`.
fn site(
    mut scene: FxScene,
    ids: &[&str],
    n: usize,
    seed: impl Fn(usize) -> Option<u64>,
) -> Rc<RefCell<Site>> {
    let effects = (0..n)
        .filter_map(|i| {
            let fx = pixfx_effects::effect(ids[i % ids.len()])?;
            let opts = Options {
                seed: seed(i),
                ..Options::default()
            };
            scene.play(fx, opts).ok()
        })
        .collect::<Vec<_>>();
    assert_eq!(effects.len(), n, "every effect exists");
    Rc::new(RefCell::new(Site { effects, scene }))
}

fn threads(n: usize) -> FxScene {
    let scene = FxScene::with_threads(n);
    assert!(scene.is_ok(), "threads start");
    scene.unwrap_or_default()
}

#[gpui::test]
fn frames_painted_on_threads_show_once_they_come_back_and_are_shared(cx: &mut TestAppContext) {
    let cx = cx.add_empty_window();
    let one = paint(
        cx,
        &site(FxScene::default(), &["feedback/power-up"], 1, |_| None),
    );
    assert!(one > 0);
    let site = site(threads(2), &["feedback/power-up"], 20, |_| None);
    assert_eq!(paint(cx, &site), 0, "nothing came back yet: nothing shows");
    let until = Instant::now() + Duration::from_secs(2);
    let mut held = 0;
    while held == 0 && Instant::now() < until {
        std::thread::sleep(Duration::from_millis(2));
        held = paint(cx, &site);
    }
    assert_eq!(
        held, one,
        "twenty power-ups hold the frame of one, as painted on the spot"
    );
}

/// Main-thread milliseconds per window frame (median) for `n` effects of their own seeds over `frames` frames of
/// 1/60 s, painted by `scene`.
fn per_frame(cx: &mut VisualTestContext, scene: FxScene, n: usize, frames: usize) -> f64 {
    let site = site(
        scene,
        &["feedback/power-up", "explosions/body-burst", "projectiles/trail"],
        n,
        |i| Some(i as u64 + 1),
    );
    let mut ms = Vec::with_capacity(frames);
    for _ in 0..frames {
        let t = Instant::now();
        assert!(site.borrow_mut().scene.tick(1.0 / 60.0).is_ok());
        paint(cx, &site);
        ms.push(t.elapsed().as_secs_f64() * 1000.0);
        // the rest of a 60 fps frame, which the threads use
        std::thread::sleep(Duration::from_secs_f64(
            (1.0 / 60.0 - ms.last().copied().unwrap_or(0.0) / 1000.0).max(0.0),
        ));
    }
    ms.sort_by(f64::total_cmp);
    ms.get(ms.len() / 2).copied().unwrap_or(0.0)
}

#[gpui::test]
#[ignore = "a measurement: run in release with --ignored --nocapture"]
fn bench(cx: &mut TestAppContext) {
    let cx = cx.add_empty_window();
    let cores = std::thread::available_parallelism().map_or(4, |n| n.get());
    let n_threads = (cores / 2).max(1);
    println!("| effects | on the spot, ms/frame | {n_threads} threads, ms/frame |");
    println!("|---:|---:|---:|");
    for n in [10, 25, 50, 100] {
        let inline = per_frame(cx, FxScene::default(), n, 120);
        let threaded = per_frame(cx, threads(n_threads), n, 120);
        println!("| {n} | {inline:.2} | {threaded:.2} |");
    }
}
