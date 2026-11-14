//! The [`pixfx_scene`] scene for JavaScript shells (Phaser, Pixi, the DOM): Rust decides every effect's frame,
//! place and lifetime and hands the shell a draw list to follow, the pixels to upload, and, when frames are painted
//! on workers, the jobs to send them.
//!
//! One [`Frames`] per game holds the frames every [`Scene`] shares. Each host frame, per scene:
//! `scene.update(dt)`, then `scene.draw(frames)`, which returns the draw list; once every scene drew,
//! `frames.endFrame()`, which returns the surfaces to free. A draw list is an `Int32Array` of [`OP_STRIDE`] numbers
//! per op, in the order to carry them out:
//!
//! | `op[0]` | Fields | The shell... |
//! |---|---|---|
//! | [`OP_MAKE`] | `id, w, h, sides` | makes surface `id`, `w` × `h` pixels, and uploads its pixels |
//! | [`OP_REFILL`] | `id, w, h, sides` | uploads new pixels into `id`, now `w` × `h` |
//! | [`OP_SHOW`] | `id, x, y, w, h, handle, layer` | draws `id` at `(x, y)`, `w` × `h` canvas pixels: colour, then light added |
//! | [`OP_FREE`] | `id` | lets go of `id` |
//!
//! `sides` says which halves hold pixels ([`SIDE_COLOR`], [`SIDE_LIGHT`]). `layer` is 0 for the back layer, 1 for
//! the front.
//!
//! Painting on workers (`new Frames(true)`, what every JS shell does): a frame the scene needs is a job
//! (`frames.takeJobs()`). A worker's [`Painting`] paints it, cuts it to the box that holds anything and turns it into
//! RGBA; the worker transfers the bytes to the main thread, and the shell hands Rust only their [`Shape`]:
//! `scene.deliver(frames, job, shape)`. The make or refill ops that returns upload the bytes the worker sent, so
//! pixels reach the main thread once and never pass through wasm there. Until a frame comes back the effect shows its
//! last frame.
//!
//! Painting on the spot (`new Frames(false)`, for hosts without workers): the pixels of every surface a draw list
//! makes or refills are read with `frames.colorPtr(id)` and `frames.lightPtr(id)` (`w * h * 4` bytes in wasm memory)
//! before the next call on the frames.

use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::num::NonZeroU32;

use js_sys::{Int32Array, JSON, Uint8Array};
use pixfx_core::{Clip, Feel, Layer, PixelBox, Placement, Sim, color_rgba_in, light_rgba_in};
use pixfx_draw::Frame;
use pixfx_scene::{
    EffectKey, InlinePainter, Job, Painter, Pass, PlacementChange, Receive, Shell, StageError,
    Target, View,
};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

use crate::js::{parse, to_js};
use crate::{find, parse_options};

/// Numbers per op in a draw list.
pub const OP_STRIDE: usize = 8;
pub const OP_MAKE: i32 = 1;
pub const OP_REFILL: i32 = 2;
pub const OP_SHOW: i32 = 3;
pub const OP_FREE: i32 = 4;
/// A surface's colour half holds pixels.
pub const SIDE_COLOR: i32 = 1;
/// A surface's light half holds pixels.
pub const SIDE_LIGHT: i32 = 2;

/// Where a frame's pixels sit once cut to the box that holds anything, and which halves hold any: all Rust needs of
/// a surface to place it. A worker sends it with the pixels; the shell hands it to `scene.deliver`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, tsify::Tsify)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Shape {
    /// The whole frame's width: a pass's rect spans it.
    pub frame_w: u32,
    /// The cut's top-left on the frame, and its size: the surface is `w` × `h`.
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
    /// [`SIDE_COLOR`] and [`SIDE_LIGHT`], or'd.
    pub sides: i32,
}

impl Shape {
    fn crop(&self) -> PixelBox {
        PixelBox {
            x: self.x,
            y: self.y,
            w: self.w,
            h: self.h,
        }
    }
}

/// A frame cut to the box that holds anything, as RGBA bytes: colour with straight alpha, light as `r, g, b, 255`
/// where lit and `0, 0, 0, 0` elsewhere. What a shell uploads.
#[derive(Clone, Debug, PartialEq)]
pub struct Cut {
    pub shape: Shape,
    pub color: Vec<u8>,
    pub light: Vec<u8>,
}

impl Cut {
    pub fn of(frame: &Frame) -> Self {
        let crop = PixelBox::lit(frame).unwrap_or_default();
        let n = crop.w as usize * crop.h as usize * 4;
        let (mut color, mut light) = (vec![0; n], vec![0; n]);
        color_rgba_in(frame, crop, &mut color);
        light_rgba_in(frame, crop, &mut light);
        let mut sides = 0;
        if color.as_chunks::<4>().0.iter().any(|p| p[3] != 0) {
            sides |= SIDE_COLOR;
        }
        if light.as_chunks::<4>().0.iter().any(|p| p[3] != 0) {
            sides |= SIDE_LIGHT;
        }
        Self {
            shape: Shape {
                frame_w: frame.w,
                x: crop.x,
                y: crop.y,
                w: crop.w,
                h: crop.h,
                sides,
            },
            color,
            light,
        }
    }
}

/// The shell side of a JS scene: each call becomes an op, with the shape of each surface kept to place it. Pixels
/// painted on the spot are kept only until the shell has read them (the next [`Recorder::take`] but one).
#[derive(Default)]
pub struct Recorder {
    next: u32,
    ops: Vec<i32>,
    shapes: HashMap<u32, Shape>,
    /// Pixels of surfaces made or refilled on the spot since the last take.
    painted: HashMap<u32, Cut>,
    /// Pixels of the ops the last take handed out: what the shell reads now.
    read: HashMap<u32, Cut>,
}

impl Recorder {
    fn push(&mut self, op: [i32; OP_STRIDE]) {
        self.ops.extend_from_slice(&op);
    }

    /// The ops since the last call; the list is empty afterwards. The pixels of surfaces they make or refill on
    /// the spot can be read until the next call.
    pub fn take(&mut self) -> Vec<i32> {
        self.read = std::mem::take(&mut self.painted);
        std::mem::take(&mut self.ops)
    }

    /// The colour of surface `id`, painted on the spot and handed out by the last take, as RGBA bytes.
    pub fn color(&self, id: u32) -> Option<&[u8]> {
        self.read.get(&id).map(|c| c.color.as_slice())
    }

    /// As [`Recorder::color`] for the light.
    pub fn light(&self, id: u32) -> Option<&[u8]> {
        self.read.get(&id).map(|c| c.light.as_slice())
    }

    fn fill(&mut self, kind: i32, id: u32, shape: Shape) {
        self.push([
            kind,
            id as i32,
            shape.w as i32,
            shape.h as i32,
            shape.sides,
            0,
            0,
            0,
        ]);
        self.shapes.insert(id, shape);
    }

    fn new_id(&mut self) -> Result<u32, StageError> {
        let id = self.next;
        self.next = self.next.checked_add(1).ok_or(StageError::OutOfHandles)?;
        Ok(id)
    }
}

impl Shell for Recorder {
    type Surface = u32;
    type Error = StageError;

    fn make(&mut self, frame: &Frame) -> Result<u32, StageError> {
        let id = self.new_id()?;
        let cut = Cut::of(frame);
        self.fill(OP_MAKE, id, cut.shape);
        self.painted.insert(id, cut);
        Ok(id)
    }

    fn refill(&mut self, surface: &mut u32, frame: &Frame) -> Result<(), StageError> {
        let cut = Cut::of(frame);
        self.fill(OP_REFILL, *surface, cut.shape);
        self.painted.insert(*surface, cut);
        Ok(())
    }

    fn show(&mut self, pass: Pass, surface: &u32) -> Result<(), StageError> {
        let Some(shape) = self.shapes.get(surface) else {
            return Ok(());
        };
        if shape.sides == 0 {
            return Ok(());
        }
        // the rect spans the whole frame in canvas pixels; the surface holds the crop
        let unit = (pass.rect.w / u64::from(shape.frame_w.max(1))) as i64;
        let c = shape.crop();
        self.push([
            OP_SHOW,
            *surface as i32,
            clamp_i32(pass.rect.x + i64::from(c.x) * unit),
            clamp_i32(pass.rect.y + i64::from(c.y) * unit),
            clamp_i32(i64::from(c.w) * unit),
            clamp_i32(i64::from(c.h) * unit),
            pass.handle as i32,
            i32::from(pass.layer != Layer::Back),
        ]);
        Ok(())
    }

    fn free(&mut self, surface: u32) -> Result<(), StageError> {
        self.shapes.remove(&surface);
        self.painted.remove(&surface);
        self.push([OP_FREE, surface as i32, 0, 0, 0, 0, 0, 0]);
        Ok(())
    }
}

/// A frame a worker painted reaches the main thread as its shape: the shell uploads the bytes the worker sent.
impl Receive<Shape> for Recorder {
    fn make_from(&mut self, shape: &Shape) -> Result<u32, StageError> {
        let id = self.new_id()?;
        self.fill(OP_MAKE, id, *shape);
        Ok(id)
    }

    fn refill_from(&mut self, surface: &mut u32, shape: &Shape) -> Result<(), StageError> {
        self.fill(OP_REFILL, *surface, *shape);
        Ok(())
    }

    fn bytes_of(&self, shape: &Shape) -> u64 {
        u64::from(shape.w) * u64::from(shape.h) * 8
    }
}

fn clamp_i32(v: i64) -> i32 {
    v.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32
}

/// What a worker needs to make an effect's state: its id and its options as the game gave them (JSON).
#[derive(Clone, Debug)]
pub struct Spec {
    pub effect: String,
    pub opts: String,
}

/// A job for a worker's [`Painting`], as JS reads it.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, tsify::Tsify)]
#[serde(rename_all = "camelCase")]
pub struct PaintJob {
    /// What `scene.deliver` takes back with the shape.
    pub job: u32,
    pub scene: u32,
    pub handle: u32,
    #[tsify(type = "EffectId")]
    pub effect: String,
    /// The play options, as JSON (`""` for none).
    pub opts: String,
    pub frame: f64,
    pub fade: f64,
    pub x: f64,
    pub y: f64,
    pub flip: bool,
    pub angle: f64,
    pub light: f64,
    /// 0 back, 1 front, 2 both.
    pub layer: u8,
}

/// What a job was for, and the size of the frame it paints, to check the shape a worker sends back.
struct Asked {
    effect: EffectKey,
    layer: Layer,
    target: Target,
    w: u32,
    h: u32,
}

/// Paints the frames of a game's scenes: on the spot, or as jobs for workers.
pub enum Paint {
    Inline(InlinePainter),
    Workers(Workers),
}

/// The jobs for workers, and what the scenes need to take their frames back.
#[derive(Default)]
pub struct Workers {
    specs: HashMap<EffectKey, Spec>,
    next_job: u32,
    jobs: Vec<PaintJob>,
    asked: HashMap<u32, Asked>,
    ended: Vec<EffectKey>,
}

impl Workers {
    /// The jobs since the last call, oldest first, and the effects gone (their state can go); both lists are empty
    /// afterwards.
    pub fn take(&mut self) -> (Vec<PaintJob>, Vec<EffectKey>) {
        (
            std::mem::take(&mut self.jobs),
            std::mem::take(&mut self.ended),
        )
    }
}

fn layer_code(layer: Layer) -> u8 {
    match layer {
        Layer::Back => 0,
        Layer::Front => 1,
        Layer::Both => 2,
    }
}

impl Painter for Paint {
    fn paint(&mut self, job: &Job<'_>) -> Option<&Frame> {
        match self {
            Self::Inline(p) => p.paint(job),
            Self::Workers(w) => {
                let spec = w.specs.get(&job.effect)?;
                let n = w.next_job;
                w.next_job = w.next_job.wrapping_add(1);
                let (fw, fh) = job.clip.size();
                w.jobs.push(PaintJob {
                    job: n,
                    scene: job.effect.scene,
                    handle: job.effect.handle,
                    effect: spec.effect.clone(),
                    opts: spec.opts.clone(),
                    frame: job.frame as f64,
                    fade: job.fade,
                    x: job.place.x,
                    y: job.place.y,
                    flip: job.place.flip,
                    angle: job.place.angle,
                    light: job.place.light,
                    layer: layer_code(job.layer),
                });
                w.asked.insert(
                    n,
                    Asked {
                        effect: job.effect,
                        layer: job.layer,
                        target: job.target.clone(),
                        w: fw,
                        h: fh,
                    },
                );
                None
            }
        }
    }

    fn end(&mut self, effect: EffectKey) {
        match self {
            Self::Inline(p) => p.end(effect),
            Self::Workers(w) => {
                w.specs.remove(&effect);
                w.ended.push(effect);
            }
        }
    }
}

/// The frames every scene of a game shares, the painter and the shell's record: one per game.
pub struct Frames {
    inner: pixfx_scene::Frames<u32, Paint>,
    rec: Recorder,
}

impl Frames {
    /// Frames painted on the spot, or (`workers`) as jobs for workers.
    pub fn with(workers: bool) -> Self {
        let paint = if workers {
            Paint::Workers(Workers::default())
        } else {
            Paint::Inline(InlinePainter::default())
        };
        Self {
            inner: pixfx_scene::Frames::new(paint),
            rec: Recorder::default(),
        }
    }

    /// The shell's record: ops to take and pixels to read.
    pub fn recorder(&mut self) -> &mut Recorder {
        &mut self.rec
    }

    /// End the host frame once every scene drew; returns the ops (frees).
    pub fn end_frame_ops(&mut self) -> Result<Vec<i32>, StageError> {
        self.inner.end_frame(&mut self.rec)?;
        Ok(self.rec.take())
    }

    /// Hold the frames of a clip baked by `pixfx bake` (its index JSON and PNG pages) as shared frames, so effects
    /// reaching them paint nothing; returns how many were loaded and the ops (makes, their pixels read like those
    /// painted on the spot).
    pub fn load_baked_ops(
        &mut self,
        json: &str,
        pages: &[&[u8]],
    ) -> Result<(u32, Vec<i32>), String> {
        let layers = pixfx_export::decode(json, pages)?;
        for l in &layers {
            self.inner
                .deliver(&l.id, l.layer, &l.frame, &mut self.rec)
                .map_err(|e| e.to_string())?;
        }
        Ok((
            u32::try_from(layers.len()).unwrap_or(u32::MAX),
            self.rec.take(),
        ))
    }

    /// The jobs for workers since the last call and the effects gone; empty when painting on the spot.
    pub fn take_jobs(&mut self) -> (Vec<PaintJob>, Vec<EffectKey>) {
        match self.inner.painter_mut() {
            Paint::Workers(w) => w.take(),
            Paint::Inline(_) => (Vec::new(), Vec::new()),
        }
    }
}

/// One scene of a game, drawn from its [`Frames`].
pub struct Scene {
    inner: pixfx_scene::Scene<u32>,
}

impl Scene {
    pub fn new(frames: &mut Frames, scale: NonZeroU32) -> Self {
        Self {
            inner: frames.inner.scene(scale),
        }
    }

    /// The scene that decides.
    pub fn scene(&mut self) -> &mut pixfx_scene::Scene<u32> {
        &mut self.inner
    }

    /// Play `clip` at game pixel (`x`, `y`); `spec` is what a worker needs to paint it.
    pub fn play_clip(
        &mut self,
        frames: &mut Frames,
        clip: Clip,
        spec: Spec,
        x: f64,
        y: f64,
    ) -> Result<u32, StageError> {
        let handle = self.inner.play(clip, x, y)?;
        if let Paint::Workers(w) = frames.inner.painter_mut() {
            w.specs.insert(
                EffectKey {
                    scene: self.inner.id(),
                    handle,
                },
                spec,
            );
        }
        Ok(handle)
    }

    /// Draw both layers; returns the draw list.
    pub fn draw_ops(&mut self, frames: &mut Frames) -> Result<Vec<i32>, StageError> {
        self.inner.draw(&mut frames.inner, &mut frames.rec)?;
        Ok(frames.rec.take())
    }

    /// Take back the frame a worker painted for `job`, as the shape of the bytes it sent; returns the ops (a make
    /// or a refill, to fill with those bytes). A job no longer wanted, or of another scene, gives no op.
    pub fn deliver_ops(
        &mut self,
        frames: &mut Frames,
        job: u32,
        shape: Shape,
    ) -> Result<Vec<i32>, StageError> {
        let Paint::Workers(w) = frames.inner.painter_mut() else {
            return Ok(Vec::new());
        };
        let Some(a) = w.asked.remove(&job) else {
            return Ok(Vec::new());
        };
        let fits =
            |at: u32, len: u32, room: u32| at.checked_add(len).is_some_and(|end| end <= room);
        if shape.frame_w != a.w || !fits(shape.x, shape.w, a.w) || !fits(shape.y, shape.h, a.h) {
            return Err(StageError::BadFrame);
        }
        let frame = shape;
        match a.target {
            Target::Shared(id) => frames
                .inner
                .deliver(&id, a.layer, &frame, &mut frames.rec)?,
            Target::Own { clock, place } if a.effect.scene == self.inner.id() => {
                self.inner.deliver_own(
                    a.effect.handle,
                    a.layer,
                    clock,
                    place,
                    &frame,
                    &mut frames.rec,
                )?;
            }
            Target::Own { .. } => {}
        }
        Ok(frames.rec.take())
    }

    /// End every effect and free the scene's own surfaces; returns the ops.
    pub fn clear_ops(&mut self, frames: &mut Frames) -> Result<Vec<i32>, StageError> {
        self.inner.clear(&mut frames.inner, &mut frames.rec)?;
        Ok(frames.rec.take())
    }
}

/// Paints jobs on a worker: each effect's clip, state and frame, kept between jobs.
#[derive(Default)]
pub struct Painting {
    effects: HashMap<EffectKey, (Clip, Sim, Frame)>,
}

impl Painting {
    /// Paint `job`, making the effect's state the first time from `clip`; returns the frame, or none when the
    /// effect is new and `clip` gives none. [`Cut::of`] turns it into what the worker sends.
    pub fn paint_with(
        &mut self,
        job: &PaintJob,
        clip: impl FnOnce() -> Option<Clip>,
    ) -> Option<&Frame> {
        let key = EffectKey {
            scene: job.scene,
            handle: job.handle,
        };
        let (clip, sim, frame) = match self.effects.entry(key) {
            Entry::Occupied(e) => e.into_mut(),
            Entry::Vacant(e) => {
                let clip = clip()?;
                let (sim, frame) = (clip.sim(), clip.blank());
                e.insert((clip, sim, frame))
            }
        };
        let layer = match job.layer {
            0 => Layer::Back,
            1 => Layer::Front,
            _ => Layer::Both,
        };
        let place = Placement {
            x: job.x,
            y: job.y,
            flip: job.flip,
            angle: job.angle,
            light: job.light,
        };
        sim.seek(job.frame as u64);
        clip.draw(sim, frame, layer, place, job.fade);
        Some(frame)
    }

    /// The effect is gone: let go of its state.
    pub fn end(&mut self, key: EffectKey) {
        self.effects.remove(&key);
    }

    /// Effects whose state it keeps.
    pub fn len(&self) -> usize {
        self.effects.len()
    }

    pub fn is_empty(&self) -> bool {
        self.effects.is_empty()
    }
}

fn js_err(e: StageError) -> JsError {
    JsError::new(&e.to_string())
}

/// What `frames.takeJobs()` gives: frames to paint on workers, and effects whose state the workers can let go
/// of (`[scene, handle]`).
#[derive(Serialize, tsify::Tsify)]
pub struct Jobs {
    pub jobs: Vec<PaintJob>,
    pub ended: Vec<(u32, u32)>,
}

/// The frames every scene of a game shares: make one per game and a [`JsScene`] per stage from it.
#[wasm_bindgen(js_name = Frames)]
pub struct JsFrames {
    inner: Frames,
}

#[wasm_bindgen(js_class = Frames)]
impl JsFrames {
    /// Frames painted on the spot, or, with `workers`, as jobs to paint on workers (`takeJobs`).
    #[wasm_bindgen(constructor)]
    pub fn new(workers: bool) -> JsFrames {
        JsFrames {
            inner: Frames::with(workers),
        }
    }

    /// A new scene drawing from these frames, one game pixel as `scale` x `scale` canvas pixels.
    pub fn scene(&mut self, scale: u32) -> Result<JsScene, JsError> {
        let scale =
            NonZeroU32::new(scale).ok_or_else(|| JsError::new("scale must be at least 1"))?;
        Ok(JsScene {
            inner: Scene::new(&mut self.inner, scale),
        })
    }

    /// Bytes the shared frames may hold before the least recently shown go (64 MiB unless set).
    #[wasm_bindgen(js_name = setBudget)]
    pub fn set_budget(&mut self, bytes: f64) {
        // a JS number of bytes; past u64 is no budget at all
        self.inner.inner.set_budget(bytes.max(0.0) as u64);
    }

    /// Bytes the shared frames hold.
    pub fn bytes(&self) -> f64 {
        self.inner.inner.bytes() as f64
    }

    /// End the host frame, once every scene drew; returns a draw list (frees).
    #[wasm_bindgen(js_name = endFrame)]
    pub fn end_frame(&mut self) -> Result<Int32Array, JsError> {
        let ops = self.inner.end_frame_ops().map_err(js_err)?;
        Ok(Int32Array::from(ops.as_slice()))
    }

    /// Free every shared frame (a game shutting down); returns a draw list (frees).
    pub fn clear(&mut self) -> Result<Int32Array, JsError> {
        self.inner
            .inner
            .clear(&mut self.inner.rec)
            .map_err(js_err)?;
        Ok(Int32Array::from(self.inner.rec.take().as_slice()))
    }

    /// Where the colour of surface `id`, painted on the spot, sits in wasm memory (`w * h * 4` RGBA bytes,
    /// straight alpha), or 0; read it after the call whose draw list made or refilled `id`, before the next.
    #[wasm_bindgen(js_name = colorPtr)]
    pub fn color_ptr(&self, id: u32) -> usize {
        self.inner.rec.color(id).map_or(0, |b| b.as_ptr() as usize)
    }

    /// As `colorPtr` for the light: lit pixels `r, g, b, 255`, unlit `0, 0, 0, 0`, to draw added.
    #[wasm_bindgen(js_name = lightPtr)]
    pub fn light_ptr(&self, id: u32) -> usize {
        self.inner.rec.light(id).map_or(0, |b| b.as_ptr() as usize)
    }

    /// Hold the frames of a clip baked by `pixfx bake` (its index JSON and PNG pages): effects playing that effect
    /// with the same options show them instead of painting them. Returns a draw list (makes; read their pixels
    /// with `colorPtr` / `lightPtr`). How many frames loaded is the number of makes.
    #[wasm_bindgen(js_name = loadBaked)]
    pub fn load_baked(
        &mut self,
        json: &str,
        pages: Vec<Uint8Array>,
    ) -> Result<Int32Array, JsError> {
        let pages: Vec<Vec<u8>> = pages.iter().map(Uint8Array::to_vec).collect();
        let refs: Vec<&[u8]> = pages.iter().map(Vec::as_slice).collect();
        let (_, ops) = self
            .inner
            .load_baked_ops(json, &refs)
            .map_err(|e| JsError::new(&e))?;
        Ok(Int32Array::from(ops.as_slice()))
    }

    /// The frames to paint on workers since the last call, and the effects gone.
    #[wasm_bindgen(js_name = takeJobs, unchecked_return_type = "Jobs")]
    pub fn take_jobs(&mut self) -> Result<JsValue, JsError> {
        let (jobs, ended) = self.inner.take_jobs();
        to_js(&Jobs {
            jobs,
            ended: ended.into_iter().map(|k| (k.scene, k.handle)).collect(),
        })
    }
}

/// One stage of a game (a Phaser scene, a Pixi stage): the effects playing, where, and the answer to their cues.
#[wasm_bindgen(js_name = Scene)]
pub struct JsScene {
    inner: Scene,
}

#[wasm_bindgen(js_class = Scene)]
impl JsScene {
    /// Which scene of its frames this is (a job's `scene`).
    pub fn id(&mut self) -> u32 {
        self.inner.scene().id()
    }

    /// Play effect `id` with its origin at game pixel (`x`, `y`); returns its handle. `opts` as `new Fx` takes
    /// them, without `margin`: the scene gives each effect room for its glow. Any object is read strictly here, so
    /// a shell hands on the game's options as they came.
    pub fn play(
        &mut self,
        frames: &mut JsFrames,
        #[wasm_bindgen(unchecked_param_type = "EffectId")] id: &str,
        x: f64,
        y: f64,
        #[wasm_bindgen(unchecked_optional_param_type = "FxOptions | Record<string, unknown>")]
        opts: JsValue,
    ) -> Result<u32, JsError> {
        let fx = find(id)?;
        let (o, margin) = parse_options(fx.meta(), &opts)?;
        if margin.is_some() {
            return Err(JsError::new(
                "margin: the scene sizes each effect's margin itself",
            ));
        }
        let json = if opts.is_undefined() || opts.is_null() {
            String::new()
        } else {
            JSON::stringify(&opts)
                .ok()
                .and_then(|s| s.as_string())
                .unwrap_or_default()
        };
        let spec = Spec {
            effect: id.to_string(),
            opts: json,
        };
        self.inner
            .play_clip(&mut frames.inner, Clip::live(fx, o), spec, x, y)
            .map_err(js_err)
    }

    /// Put the effect's origin at game pixel (`x`, `y`), keeping its time. False for an unknown handle.
    #[wasm_bindgen(js_name = moveTo)]
    pub fn move_to(&mut self, handle: u32, x: f64, y: f64) -> Result<bool, JsError> {
        self.inner.scene().move_to(handle, x, y).map_err(js_err)
    }

    /// Change where and how the effect draws (`{x?, y?, flip?, angle?, light?}`). False for an unknown handle.
    pub fn set(
        &mut self,
        handle: u32,
        #[wasm_bindgen(unchecked_param_type = "PlacementChange")] change: JsValue,
    ) -> Result<bool, JsError> {
        let change: PlacementChange = parse("placement", &change)?;
        let scene = self.inner.scene();
        let Some(base) = scene.placement(handle) else {
            return Ok(false);
        };
        scene.set(handle, change.over(base)).map_err(js_err)
    }

    /// Let the effect end: loops fade out, one-shots are cut short. False for an unknown handle.
    pub fn stop(&mut self, handle: u32) -> bool {
        self.inner.scene().stop(handle)
    }

    /// Drop the effect now. False for an unknown handle.
    pub fn remove(&mut self, handle: u32) -> bool {
        self.inner.scene().remove(handle)
    }

    /// The handles playing, oldest first.
    pub fn handles(&mut self) -> Vec<u32> {
        self.inner.scene().handles().collect()
    }

    /// Let `dt` real seconds pass; returns the seconds the effects played (0 in a hit-stop).
    pub fn update(&mut self, dt: f64) -> Result<f64, JsError> {
        self.inner.scene().update(dt).map_err(js_err)
    }

    /// The events since the last call (`{type: "end", handle}`, `{type: "cue", handle, t, pan, cue}`).
    #[wasm_bindgen(unchecked_return_type = "StageEvent[]")]
    pub fn events(&mut self) -> Result<JsValue, JsError> {
        to_js(&self.inner.scene().drain_events())
    }

    /// Draw both layers of every effect; returns the draw list.
    pub fn draw(&mut self, frames: &mut JsFrames) -> Result<Int32Array, JsError> {
        let ops = self.inner.draw_ops(&mut frames.inner).map_err(js_err)?;
        Ok(Int32Array::from(ops.as_slice()))
    }

    /// Take back the frame a worker painted for `job`, as the shape the worker sent with its bytes. Returns the
    /// draw list: a make or refill there is filled with those bytes.
    pub fn deliver(
        &mut self,
        frames: &mut JsFrames,
        job: u32,
        #[wasm_bindgen(unchecked_param_type = "Shape")] shape: JsValue,
    ) -> Result<Int32Array, JsError> {
        let shape: Shape = parse("shape", &shape)?;
        let ops = self
            .inner
            .deliver_ops(&mut frames.inner, job, shape)
            .map_err(js_err)?;
        Ok(Int32Array::from(ops.as_slice()))
    }

    /// End every effect and free the scene's own surfaces (the scene shuts down); returns the draw list.
    pub fn clear(&mut self, frames: &mut JsFrames) -> Result<Int32Array, JsError> {
        let ops = self.inner.clear_ops(&mut frames.inner).map_err(js_err)?;
        Ok(Int32Array::from(ops.as_slice()))
    }

    #[wasm_bindgen(js_name = setScale)]
    pub fn set_scale(&mut self, scale: u32) -> Result<(), JsError> {
        let scale =
            NonZeroU32::new(scale).ok_or_else(|| JsError::new("scale must be at least 1"))?;
        self.inner.scene().set_scale(scale);
        Ok(())
    }

    /// The stage's width in game pixels, so a cue's pan follows where it fires.
    #[wasm_bindgen(js_name = setScreenWidth)]
    pub fn set_screen_width(&mut self, w: f64) {
        self.inner.scene().set_screen_w(w);
    }

    /// Draw only the effects whose frame overlaps the view (game pixels); the rest play on undrawn.
    #[wasm_bindgen(js_name = setView)]
    pub fn set_view(&mut self, x: f64, y: f64, w: f64, h: f64) -> Result<(), JsError> {
        self.inner
            .scene()
            .set_view(Some(View { x, y, w, h }))
            .map_err(js_err)
    }

    /// Draw every effect again.
    #[wasm_bindgen(js_name = clearView)]
    pub fn clear_view(&mut self) -> Result<(), JsError> {
        self.inner.scene().set_view(None).map_err(js_err)
    }

    /// Draw at most `layers` layers per host frame; 0 draws every one that changed.
    #[wasm_bindgen(js_name = setDrawBudget)]
    pub fn set_draw_budget(&mut self, layers: u32) {
        self.inner.scene().set_draw_budget(NonZeroU32::new(layers));
    }

    /// Let the effect play real time through a hit-stop (`on`), or freeze with the rest. False for an unknown
    /// handle.
    #[wasm_bindgen(js_name = playThroughHitstop)]
    pub fn play_through_hitstop(&mut self, handle: u32, on: bool) -> bool {
        self.inner.scene().set_plays_through_hitstop(handle, on)
    }

    /// Answer cues with the scene's own feel (`on`, the default) or leave them to the game.
    #[wasm_bindgen(js_name = setFeel)]
    pub fn set_feel(&mut self, on: bool) {
        self.inner.scene().set_feel(on.then(Feel::new));
    }

    /// Where the view sits, `[dx, dy]` in game pixels, at `strength` (1 as designed).
    pub fn shake(&mut self, strength: f64) -> Vec<i32> {
        let (x, y) = self
            .inner
            .scene()
            .feel()
            .map_or((0, 0), |f| f.shake(strength));
        vec![x, y]
    }

    /// How far the view tilts, radians, at `strength`.
    pub fn roll(&mut self, strength: f64) -> f64 {
        self.inner.scene().feel().map_or(0.0, |f| f.roll(strength))
    }

    /// How white the view is lifted, 0 to 1.
    pub fn flash(&mut self) -> f64 {
        self.inner.scene().feel().map_or(0.0, Feel::flash)
    }

    /// Whether a hit-stop holds play now.
    pub fn frozen(&mut self) -> bool {
        self.inner.scene().feel().is_some_and(Feel::frozen)
    }

    pub fn trauma(&mut self) -> f64 {
        self.inner.scene().feel().map_or(0.0, Feel::trauma)
    }

    /// The game's own impacts.
    #[wasm_bindgen(js_name = addTrauma)]
    pub fn add_trauma(&mut self, k: f64) {
        if let Some(f) = self.inner.scene().feel_mut() {
            f.add_trauma(k);
        }
    }

    pub fn freeze(&mut self, secs: f64) {
        if let Some(f) = self.inner.scene().feel_mut() {
            f.freeze(secs);
        }
    }

    pub fn slow(&mut self, secs: f64, speed: f64) {
        if let Some(f) = self.inner.scene().feel_mut() {
            f.slow(secs, speed);
        }
    }
}

/// Paints jobs on a worker (`frames.takeJobs()`); post the shape and bytes back (transferring the bytes) for
/// `scene.deliver`.
#[wasm_bindgen(js_name = Painting)]
#[derive(Default)]
pub struct JsPainting {
    inner: Painting,
}

/// A painted frame, cut and turned into RGBA: its shape for `scene.deliver`, and the bytes to upload.
#[wasm_bindgen]
pub struct Painted {
    cut: Cut,
}

#[wasm_bindgen]
impl Painted {
    #[wasm_bindgen(unchecked_return_type = "Shape")]
    pub fn shape(&self) -> Result<JsValue, JsError> {
        to_js(&self.cut.shape)
    }

    /// `shape.w * shape.h * 4` bytes, straight alpha.
    pub fn color(&self) -> Uint8Array {
        Uint8Array::from(self.cut.color.as_slice())
    }

    /// Lit pixels `r, g, b, 255`, unlit `0, 0, 0, 0`, to draw added.
    pub fn light(&self) -> Uint8Array {
        Uint8Array::from(self.cut.light.as_slice())
    }
}

#[wasm_bindgen(js_class = Painting)]
impl JsPainting {
    #[wasm_bindgen(constructor)]
    pub fn new() -> JsPainting {
        JsPainting::default()
    }

    /// Paint `job`, a `PaintJob` from `frames.takeJobs()` as it came through a worker's message (read strictly).
    pub fn paint(
        &mut self,
        #[wasm_bindgen(unchecked_param_type = "PaintJob | unknown")] job: JsValue,
    ) -> Result<Painted, JsError> {
        let job: PaintJob = parse("job", &job)?;
        let mut bad = None;
        let frame = self.inner.paint_with(&job, || {
            let made = (|| {
                let fx = find(&job.effect)?;
                let opts = if job.opts.is_empty() {
                    JsValue::UNDEFINED
                } else {
                    JSON::parse(&job.opts).map_err(|_| JsError::new("job options are not JSON"))?
                };
                let (o, _) = parse_options(fx.meta(), &opts)?;
                Ok::<_, JsError>(Clip::live(fx, o))
            })();
            made.map_err(|e| bad = Some(e)).ok()
        });
        match (frame, bad) {
            (Some(f), _) => Ok(Painted { cut: Cut::of(f) }),
            (None, Some(e)) => Err(e),
            (None, None) => Err(JsError::new("nothing painted")),
        }
    }

    /// The effect `[scene, handle]` is gone.
    pub fn end(&mut self, scene: u32, handle: u32) {
        self.inner.end(EffectKey { scene, handle });
    }
}
