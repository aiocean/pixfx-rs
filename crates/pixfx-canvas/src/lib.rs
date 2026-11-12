//! pixfx as a wasm renderer: mount a [`Stage`] on a canvas, say which effects play where, and it runs its own
//! `requestAnimationFrame` loop and draws them (or a host that owns the loop calls [`Stage::tick`]).
//!
//! Canvas 2D: each frame clears the canvas (or fills the background), then draws the back layer of every
//! effect, then the front layer of every effect. Each layer is its colour drawn normally and its light drawn
//! with `globalCompositeOperation = "lighter"` (additive, saturating), upscaled by an integer scale with
//! image smoothing off. The decisions live in [`stage`] (plain Rust, tested natively); this file only blits.
#![forbid(unsafe_code)]

pub mod stage;

use std::cell::{Cell, RefCell};
use std::num::NonZeroU32;
use std::rc::{Rc, Weak};

use js_sys::Uint8ClampedArray;
use pixfx_core::Frame;
use pixfx_effects::effect;
use pixfx_wasm::{Player, parse_options};
use stage::{Pass, Scene, color_rgba, light_rgba};
use wasm_bindgen::prelude::*;
use web_sys::{
    CanvasRenderingContext2d, HtmlCanvasElement, ImageData, OffscreenCanvas,
    OffscreenCanvasRenderingContext2d, Window,
};

/// Effects drawing onto one canvas, in game pixels upscaled by an integer `scale`.
#[wasm_bindgen]
pub struct Stage {
    /// Shared with the animation-frame closure, which must reach the renderer from a JS callback after this
    /// call returns: `Rc<RefCell<..>>` is the standard wasm-bindgen ownership for that (single thread, no
    /// re-entrant borrow: the closure and the methods below never run inside each other).
    renderer: Rc<RefCell<Renderer>>,
    running: Option<Loop>,
}

#[wasm_bindgen]
impl Stage {
    /// Draw onto `canvas` with one game pixel as `scale` x `scale` canvas pixels (`scale` at least 1).
    #[wasm_bindgen(constructor)]
    pub fn new(canvas: HtmlCanvasElement, scale: u32) -> Result<Stage, JsError> {
        let ctx = canvas
            .get_context("2d")
            .map_err(|_| JsError::new("could not get the canvas 2d context"))?
            .ok_or_else(|| {
                JsError::new("the canvas has no 2d context (already used for another?)")
            })?
            .dyn_into::<CanvasRenderingContext2d>()
            .map_err(|_| JsError::new("the canvas 2d context has an unexpected type"))?;
        Ok(Stage {
            renderer: Rc::new(RefCell::new(Renderer {
                ctx,
                scene: Scene::new(nonzero_scale(scale)?),
                background: None,
            })),
            running: None,
        })
    }

    /// Play effect `id` with its origin at game pixel (`x`, `y`), rounded to a whole pixel; returns its
    /// handle. `opts`: `undefined` or `{seed?, flip?, angle?, light?, palette?: string[], ...extra}`.
    pub fn play(&mut self, id: &str, x: f64, y: f64, opts: JsValue) -> Result<u32, JsError> {
        let fx = effect(id).ok_or_else(|| JsError::new(&format!("unknown effect {id:?}")))?;
        let player = Player::new(fx, parse_options(&opts)?);
        let surface = Surface::new(player.frame())?;
        let mut r = self.renderer.borrow_mut();
        Ok(r.scene.play(player, x, y, surface)?)
    }

    /// Let the effect end: loops fade out, one-shots are cut short; it is removed once done. Unknown handles
    /// are ignored.
    pub fn stop(&mut self, handle: u32) {
        self.renderer.borrow_mut().scene.stop(handle);
    }

    /// Drop the effect now. Unknown handles are ignored.
    pub fn remove(&mut self, handle: u32) {
        self.renderer.borrow_mut().scene.remove(handle);
    }

    /// Run the stage's own animation-frame loop with real elapsed time. Calling it while running does
    /// nothing; stop it with `pause()` or by freeing the stage.
    pub fn start(&mut self) -> Result<(), JsError> {
        if self.running.is_none() {
            self.running = Some(Loop::start(Rc::clone(&self.renderer))?);
        }
        Ok(())
    }

    /// Stop the loop; effects keep their state and resume from there on `start()`.
    pub fn pause(&mut self) {
        // dropping the loop cancels its pending frame and frees its closure
        self.running = None;
    }

    /// Game pixel to canvas pixels (at least 1).
    #[wasm_bindgen(js_name = setScale)]
    pub fn set_scale(&mut self, scale: u32) -> Result<(), JsError> {
        self.renderer
            .borrow_mut()
            .scene
            .set_scale(nonzero_scale(scale)?);
        Ok(())
    }

    /// Fill the canvas with this CSS colour before drawing; `""` clears to transparent instead.
    pub fn background(&mut self, css_color: &str) {
        self.renderer.borrow_mut().background =
            (!css_color.is_empty()).then(|| css_color.to_owned());
    }

    /// Advance by `dt` seconds (finite, 0 or more) and draw once, for hosts that own the loop.
    pub fn tick(&mut self, dt: f64) -> Result<(), JsError> {
        self.renderer
            .borrow_mut()
            .tick(dt)
            .map_err(|e| JsError::new(&e))
    }

    /// The playing handles, oldest first (finished effects are gone).
    pub fn handles(&self) -> Vec<u32> {
        self.renderer.borrow().scene.handles().collect()
    }
}

fn nonzero_scale(scale: u32) -> Result<NonZeroU32, JsError> {
    NonZeroU32::new(scale).ok_or_else(|| JsError::new("scale must be at least 1"))
}

struct Renderer {
    ctx: CanvasRenderingContext2d,
    scene: Scene<Surface>,
    background: Option<String>,
}

impl Renderer {
    fn tick(&mut self, dt: f64) -> Result<(), String> {
        self.scene.update(dt).map_err(|e| e.to_string())?;
        let ctx = &self.ctx;
        let canvas = ctx.canvas().ok_or("the 2d context has no canvas")?;
        let (w, h) = (f64::from(canvas.width()), f64::from(canvas.height()));
        ctx.set_global_composite_operation("source-over")
            .map_err(|e| js_message("could not reset compositing", &e))?;
        match &self.background {
            Some(css) => {
                ctx.set_fill_style_str(css);
                ctx.fill_rect(0.0, 0.0, w, h);
            }
            None => ctx.clear_rect(0.0, 0.0, w, h),
        }
        // resizing the canvas resets this, so set it every frame
        ctx.set_image_smoothing_enabled(false);
        self.scene
            .draw(|pass, frame, surface| surface.blit(ctx, pass, frame))
    }
}

/// Frame to RGBA bytes for one half of a layer; true if anything shows.
type Convert = fn(&Frame, &mut [u8]) -> bool;

/// One effect's scratch canvas and the pixel buffer each layer goes through on its way to the stage. Built
/// once per effect, reused every frame: drawing a layer allocates nothing on either side.
struct Surface {
    canvas: OffscreenCanvas,
    ctx: OffscreenCanvasRenderingContext2d,
    /// The layer as RGBA bytes, converted in Rust ...
    bytes: Vec<u8>,
    /// ... copied into this JS array, which `image` views.
    js: Uint8ClampedArray,
    image: ImageData,
}

impl Surface {
    fn new(frame: &Frame) -> Result<Self, JsError> {
        let (w, h) = (frame.w, frame.h);
        let canvas = OffscreenCanvas::new(w, h)
            .map_err(|_| JsError::new("could not create an OffscreenCanvas"))?;
        let ctx = canvas
            .get_context("2d")
            .map_err(|_| JsError::new("could not get an OffscreenCanvas 2d context"))?
            .ok_or_else(|| JsError::new("OffscreenCanvas has no 2d context"))?
            .dyn_into::<OffscreenCanvasRenderingContext2d>()
            .map_err(|_| JsError::new("OffscreenCanvas 2d context has an unexpected type"))?;
        let len = w as usize * h as usize * 4;
        let js = Uint8ClampedArray::new_with_length(len as u32);
        let image = ImageData::new_with_js_u8_clamped_array_and_sh(&js, w, h)
            .map_err(|_| JsError::new("could not create ImageData"))?;
        Ok(Self {
            canvas,
            ctx,
            bytes: vec![0; len],
            js,
            image,
        })
    }

    /// Draw `frame` (one layer of one effect) into `pass.rect`: colour normally, then light with "lighter".
    /// Empty colour or light is skipped.
    fn blit(
        &mut self,
        stage: &CanvasRenderingContext2d,
        pass: Pass,
        frame: &Frame,
    ) -> Result<(), String> {
        let r = pass.rect;
        let (x, y, w, h) = (r.x as f64, r.y as f64, r.w as f64, r.h as f64);
        let steps: [(Convert, &str); 2] = [(color_rgba, "source-over"), (light_rgba, "lighter")];
        for (convert, op) in steps {
            if !convert(frame, &mut self.bytes) {
                continue;
            }
            self.js.copy_from(&self.bytes);
            self.ctx
                .put_image_data(&self.image, 0.0, 0.0)
                .map_err(|e| js_message("putImageData failed", &e))?;
            stage
                .set_global_composite_operation(op)
                .map_err(|e| js_message("could not set compositing", &e))?;
            stage
                .draw_image_with_offscreen_canvas_and_dw_and_dh(&self.canvas, x, y, w, h)
                .map_err(|e| js_message("drawImage failed", &e))?;
        }
        Ok(())
    }
}

fn js_message(what: &str, e: &JsValue) -> String {
    match e.dyn_ref::<js_sys::Error>() {
        Some(err) => format!("{what}: {}", String::from(err.message())),
        None => format!("{what}: {e:?}"),
    }
}

type FrameCallback = Closure<dyn FnMut(f64)>;
type Slot = Rc<RefCell<Option<FrameCallback>>>;

/// The running animation-frame loop. Dropping it cancels the pending frame and frees the closure, so neither
/// `pause()` nor freeing the stage leaks it.
struct Loop {
    window: Window,
    /// The pending `requestAnimationFrame` id, updated by the closure each frame.
    pending: Rc<Cell<i32>>,
    /// Owns the closure. The closure holds only a `Weak` to this slot (to schedule itself again), so there is
    /// no reference cycle: dropping the `Loop` drops the closure.
    closure: Slot,
}

impl Loop {
    fn start(renderer: Rc<RefCell<Renderer>>) -> Result<Self, JsError> {
        let window =
            web_sys::window().ok_or_else(|| JsError::new("no window: start() needs a browser"))?;
        let pending = Rc::new(Cell::new(0));
        let closure: Slot = Rc::new(RefCell::new(None));
        let slot: Weak<RefCell<Option<FrameCallback>>> = Rc::downgrade(&closure);
        let (win, ids) = (window.clone(), Rc::clone(&pending));
        let mut last: Option<f64> = None;
        let frame = move |now: f64| {
            // `now` is the frame's timestamp on the performance.now() clock. The first frame after start()
            // steps 0; a clock that runs backwards steps 0; a non-finite stamp is skipped.
            let dt = match last {
                Some(prev) if now.is_finite() => (now - prev).max(0.0) / 1000.0,
                _ => 0.0,
            };
            if now.is_finite() {
                last = Some(now);
            }
            if let Err(e) = renderer.borrow_mut().tick(dt) {
                // stop scheduling rather than repeat the error every frame; pause()/start() retries
                web_sys::console::error_1(&JsValue::from_str(&format!("pixfx Stage stopped: {e}")));
                return;
            }
            let Some(slot) = slot.upgrade() else { return };
            let next = slot
                .borrow()
                .as_ref()
                .map(|c| win.request_animation_frame(c.as_ref().unchecked_ref()));
            match next {
                Some(Ok(id)) => ids.set(id),
                Some(Err(e)) => web_sys::console::error_1(&e),
                None => {}
            }
        };
        let cb = Closure::<dyn FnMut(f64)>::new(frame);
        let id = window
            .request_animation_frame(cb.as_ref().unchecked_ref())
            .map_err(|_| JsError::new("requestAnimationFrame failed"))?;
        pending.set(id);
        *closure.borrow_mut() = Some(cb);
        Ok(Self {
            window,
            pending,
            closure,
        })
    }
}

impl Drop for Loop {
    fn drop(&mut self) {
        // cancel first: a frame firing into a freed closure would throw
        let _ = self.window.cancel_animation_frame(self.pending.get());
        self.closure.borrow_mut().take();
    }
}
