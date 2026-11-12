//! A candle and a campfire burning live in gpui, on a dark ground the campfire lights up.
//!
//! `trunk serve --release` in this directory, then open http://127.0.0.1:8090 (`?backend=webgl` or
//! `?backend=webgpu` forces a renderer). The console logs the frame rate every two seconds.
#![cfg_attr(target_family = "wasm", no_main)]
#![forbid(unsafe_code)]

use gpui::{
    App, Application, Bounds, Context, Pixels, Window, WindowBounds, WindowOptions, canvas, fill,
    point, prelude::*, px, rgb, size,
};
use pixfx_core::{Effect, Layer, Options};
use pixfx_gpui::{FxView, paint_fx};
use web_time::Instant;

/// Logical pixels per effect pixel.
const SCALE: u32 = 4;
const SKY: u32 = 0x0b0a10;
const GROUND: u32 = 0x1e1a24;
/// Seconds between frame-rate log lines.
const FPS_EVERY: f64 = 2.0;

struct Campsite {
    candle: FxView,
    campfire: FxView,
    last: Option<Instant>,
    frames: u32,
    since: Option<Instant>,
}

impl Campsite {
    fn new(candle: &'static dyn Effect, campfire: &'static dyn Effect) -> Self {
        Self {
            candle: FxView::new(candle, Options::default()),
            campfire: FxView::new(campfire, Options::default()),
            last: None,
            frames: 0,
            since: None,
        }
    }

    /// Advance both effects by the real time since the last frame.
    fn tick(&mut self, now: Instant) {
        if let Some(last) = self.last {
            let dt = now.duration_since(last).as_secs_f64();
            self.candle.tick(dt);
            self.campfire.tick(dt);
        }
        self.last = Some(now);
        self.count_frame(now);
    }

    fn count_frame(&mut self, now: Instant) {
        let since = *self.since.get_or_insert(now);
        self.frames += 1;
        let elapsed = now.duration_since(since).as_secs_f64();
        if elapsed >= FPS_EVERY {
            log::info!(
                "pixfx: {:.1} fps ({:.1} ms/frame)",
                f64::from(self.frames) / elapsed,
                elapsed * 1000.0 / f64::from(self.frames)
            );
            self.frames = 0;
            self.since = Some(now);
        }
    }

    /// Sky, ground, and the two effects standing on the ground: candle left of centre, campfire right.
    fn paint(&mut self, bounds: Bounds<Pixels>, window: &mut Window) {
        window.paint_quad(fill(bounds, rgb(SKY)));
        let ground_y = (bounds.origin.y + bounds.size.height * 0.72).round();
        let ground = Bounds::new(
            point(bounds.origin.x, ground_y),
            size(
                bounds.size.width,
                bounds.origin.y + bounds.size.height - ground_y,
            ),
        );
        window.paint_quad(fill(ground, rgb(GROUND)));
        let at = |fraction: f32| {
            point(
                (bounds.origin.x + bounds.size.width * fraction).round(),
                ground_y,
            )
        };
        for (view, x) in [(&mut self.candle, 0.3), (&mut self.campfire, 0.62)] {
            if let Err(error) = paint_fx(view, Layer::Both, at(x), SCALE, window) {
                log::error!("pixfx: failed to paint an effect: {error:#}");
            }
        }
    }
}

impl Render for Campsite {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.tick(Instant::now());
        window.request_animation_frame();
        let campsite = cx.entity();
        canvas(
            |_, _, _| {},
            move |bounds, _, window, cx| {
                campsite.update(cx, |campsite, _| campsite.paint(bounds, window));
            },
        )
        .size_full()
    }
}

fn run(application: Application) {
    application.run(|cx: &mut App| {
        let (Some(candle), Some(campfire)) = (
            pixfx_effects::effect("fire/candle"),
            pixfx_effects::effect("fire/campfire"),
        ) else {
            log::error!("pixfx: fire/candle or fire/campfire is missing from pixfx-effects");
            return;
        };
        let bounds = Bounds::centered(None, size(px(1280.), px(720.)), cx);
        let opened = cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |_, cx| cx.new(|_| Campsite::new(candle, campfire)),
        );
        if let Err(error) = opened {
            log::error!("pixfx: failed to open the window: {error:#}");
            return;
        }
        cx.activate(true);
    });
}

#[cfg(not(target_family = "wasm"))]
fn main() {
    run(gpui_platform::application());
}

/// `?backend=webgl` or `?backend=webgpu` forces a renderer so both paths can be checked.
#[cfg(target_family = "wasm")]
fn requested_backend() -> gpui_platform::WebBackendPreference {
    let search = web_sys::window()
        .and_then(|window| window.location().search().ok())
        .unwrap_or_default();
    let requested = |value: &str| {
        search
            .trim_start_matches('?')
            .split('&')
            .any(|p| p == value)
    };
    if requested("backend=webgpu") {
        gpui_platform::WebBackendPreference::WebGpu
    } else if requested("backend=webgl") {
        gpui_platform::WebBackendPreference::WebGl
    } else {
        gpui_platform::WebBackendPreference::Auto
    }
}

#[cfg(target_family = "wasm")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    gpui_platform::web_init();
    run(gpui_platform::application_with_web_backend(
        requested_backend(),
    ));
}
