//! A trail streaking and a power-up rising on its stand-in body, live in gpui, on a dark ground. The power-up plays
//! again each time it ends.
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
use pixfx_gpui::{FxHandle, FxScene};
use web_time::Instant;

/// Logical pixels per effect pixel.
const SCALE: u32 = 4;
const SKY: u32 = 0x0b0a10;
const GROUND: u32 = 0x1e1a24;
/// Seconds between frame-rate log lines.
const FPS_EVERY: f64 = 2.0;

struct Stage {
    /// The effects and the frames they paint from.
    fx: FxScene,
    trail: FxHandle,
    power_up: &'static dyn Effect,
    buff: FxHandle,
    last: Option<Instant>,
    frames: u32,
    since: Option<Instant>,
}

impl Stage {
    fn new(
        trail: &'static dyn Effect,
        power_up: &'static dyn Effect,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let mut fx = FxScene::default();
        let trail = fx.play(trail, Options::default())?;
        let buff = fx.play(power_up, Options::default())?;
        Ok(Self {
            fx,
            trail,
            power_up,
            buff,
            last: None,
            frames: 0,
            since: None,
        })
    }

    /// Advance both effects by the real time since the last frame, and play the power-up again once it ended.
    fn tick(&mut self, now: Instant) {
        if let Some(last) = self.last {
            let dt = now.duration_since(last).as_secs_f64();
            if let Err(error) = self.fx.tick(dt) {
                log::error!("pixfx: failed to advance the effects: {error:#}");
            }
        }
        if self.fx.done(self.buff) {
            match self.fx.play(self.power_up, Options::default()) {
                Ok(buff) => self.buff = buff,
                Err(error) => log::error!("pixfx: failed to play the power-up again: {error:#}"),
            }
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

    /// Sky, ground, and the two effects standing on the ground: the trail left of centre, the power-up right.
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
        for (fx, x) in [(self.trail, 0.3), (self.buff, 0.62)] {
            if let Err(error) = self.fx.paint(fx, Layer::Both, at(x), SCALE, window) {
                log::error!("pixfx: failed to paint an effect: {error:#}");
            }
        }
        if let Err(error) = self.fx.end_frame(window) {
            log::error!("pixfx: failed to drop old effect frames: {error:#}");
        }
    }
}

impl Render for Stage {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.tick(Instant::now());
        window.request_animation_frame();
        let stage = cx.entity();
        canvas(
            |_, _, _| {},
            move |bounds, _, window, cx| {
                stage.update(cx, |stage, _| stage.paint(bounds, window));
            },
        )
        .size_full()
    }
}

fn run(application: Application) {
    application.run(|cx: &mut App| {
        let (Some(trail), Some(power_up)) = (
            pixfx_effects::effect("projectiles/trail"),
            pixfx_effects::effect("feedback/power-up"),
        ) else {
            log::error!("pixfx: projectiles/trail or feedback/power-up is missing from pixfx-effects");
            return;
        };
        let stage = match Stage::new(trail, power_up) {
            Ok(stage) => stage,
            Err(error) => {
                log::error!("pixfx: failed to start the effects: {error}");
                return;
            }
        };
        let bounds = Bounds::centered(None, size(px(1280.), px(720.)), cx);
        let opened = cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |_, cx| cx.new(|_| stage),
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
