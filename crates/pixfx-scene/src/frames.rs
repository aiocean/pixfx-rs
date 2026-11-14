//! The frames every scene of a game shares, and who paints them. One [`Frames`] serves any number of [`Scene`]s
//! (a canvas each, a Phaser scene each, a gpui window each): a frame one scene painted shows in every other
//! without being painted again, under one byte budget. Painting goes through a [`Painter`]: on the spot
//! ([`InlinePainter`]), or elsewhere (a worker, a thread), the frame handed back later with [`Frames::deliver`].
//!
//! [`Scene`]: crate::Scene

use std::collections::{HashMap, HashSet};

use pixfx_core::{Clip, FrameId, FrameStore, Layer, PixelBox, Placement, Sim};
use pixfx_draw::Frame;

use crate::{Receive, Scene, Shell};
use std::num::NonZeroU32;

/// One effect among every scene sharing a [`Frames`]: the scene that plays it and its handle there.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct EffectKey {
    pub scene: u32,
    pub handle: u32,
}

/// Which frame a paint is for, so a frame painted elsewhere finds its way back: a frame shared by every effect
/// showing it, or an effect's own frame while it fades out, drawn at clock frame `clock` and placed as `place`.
#[derive(Clone, Debug, PartialEq)]
pub enum Target {
    Shared(FrameId),
    Own { clock: u64, place: Placement },
}

/// One layer of one frame of one effect to paint: the effect's state brought to `frame` ([`Sim::seek`]), drawn
/// placed as `place` at opacity `fade`. The clip says what the effect is; a painter that has not seen `effect`
/// before makes its state from it.
pub struct Job<'a> {
    pub effect: EffectKey,
    pub clip: &'a Clip,
    pub frame: u64,
    pub fade: f64,
    pub place: Placement,
    pub layer: Layer,
    pub target: Target,
}

/// Paints the frames scenes ask for. It keeps each effect's state between paints, so stepping a playing effect
/// costs one tick a frame, wherever it paints.
pub trait Painter {
    /// Paint `job` now and lend the frame, or `None`: it is painted elsewhere and comes back through
    /// [`Frames::deliver`] (shared) or [`Scene::deliver_own`] (own), and the scene shows what it showed before
    /// until then.
    fn paint(&mut self, job: &Job<'_>) -> Option<&Frame>;
    /// The effect is gone: let go of its state.
    fn end(&mut self, effect: EffectKey);
}

/// Paints on the spot, on the caller's thread: what Canvas 2D, gpui, files and tests use.
#[derive(Default)]
pub struct InlinePainter {
    sims: HashMap<EffectKey, (Sim, Frame)>,
}

impl InlinePainter {
    /// Effects whose state it keeps.
    pub fn len(&self) -> usize {
        self.sims.len()
    }

    pub fn is_empty(&self) -> bool {
        self.sims.is_empty()
    }
}

impl Painter for InlinePainter {
    fn paint(&mut self, job: &Job<'_>) -> Option<&Frame> {
        let (sim, frame) = self
            .sims
            .entry(job.effect)
            .or_insert_with(|| (job.clip.sim(), job.clip.blank()));
        sim.seek(job.frame);
        job.clip.draw(sim, frame, job.layer, job.place, job.fade);
        Some(frame)
    }

    fn end(&mut self, effect: EffectKey) {
        self.sims.remove(&effect);
    }
}

/// Bytes a surface holding `frame` takes: its colour and light as RGBA, cut to the box that holds anything (shells
/// upload only that box; an effect lights a small part of its frame).
pub(crate) fn frame_bytes(frame: &Frame) -> u64 {
    PixelBox::lit(frame).map_or(0, |b| u64::from(b.w) * u64::from(b.h) * 8)
}

/// The shared frames of a game and the painter that paints every frame its scenes show. The host ends each of
/// its frames with [`Frames::end_frame`] once every scene drew: past the budget the least recently shown frames
/// go then, never one shown in that host frame.
pub struct Frames<S, P> {
    pub(crate) store: FrameStore<S>,
    pub(crate) painter: P,
    /// Shared frames asked of the painter and not delivered yet: asked once, however many effects wait for them.
    pub(crate) pending: HashSet<(FrameId, Layer)>,
    /// Host frames ended; frames shown in the current one are marked with `draws + 1`.
    pub(crate) draws: u64,
    next_scene: u32,
}

impl<S, P: Painter> Frames<S, P> {
    pub fn new(painter: P) -> Self {
        Self {
            store: FrameStore::default(),
            painter,
            pending: HashSet::new(),
            draws: 0,
            next_scene: 0,
        }
    }

    /// A new scene drawing from these frames, `scale` canvas pixels per game pixel.
    pub fn scene(&mut self, scale: NonZeroU32) -> Scene<S> {
        let id = self.next_scene;
        self.next_scene = self.next_scene.wrapping_add(1);
        Scene::new(id, scale)
    }

    pub fn painter(&self) -> &P {
        &self.painter
    }

    pub fn painter_mut(&mut self) -> &mut P {
        &mut self.painter
    }

    /// The host frame being drawn: what shown frames are marked with.
    pub(crate) fn now(&self) -> u64 {
        self.draws + 1
    }

    /// Hold `frame` (a [`Frame`], or what the shell takes from a painter elsewhere: [`Receive`]) as `layer` of
    /// frame `id`, painted elsewhere ([`Painter::paint`] gave `None`) or baked, so
    /// effects showing it paint nothing. Held like a painted frame: past the budget it goes, least recently shown
    /// first, and is painted again if shown again. A frame already held keeps what it has.
    pub fn deliver<D, H: Receive<D, Surface = S>>(
        &mut self,
        id: &FrameId,
        layer: Layer,
        frame: &D,
        shell: &mut H,
    ) -> Result<(), H::Error> {
        self.pending.remove(&(id.clone(), layer));
        let now = self.now();
        self.store.show(id, layer, now, || {
            let bytes = shell.bytes_of(frame);
            Ok((shell.make_from(frame)?, bytes))
        })?;
        Ok(())
    }

    /// How many bytes the shared frames may hold before the least recently shown go ([`pixfx_core::FRAME_BUDGET`] unless
    /// set); takes effect at the next [`Frames::end_frame`].
    pub fn set_budget(&mut self, bytes: u64) {
        self.store.set_budget(bytes);
    }

    /// Bytes the shared frames hold now.
    pub fn bytes(&self) -> u64 {
        self.store.bytes()
    }

    /// End the host's frame, once every scene drew: past the budget, free the least recently shown shared frames,
    /// never one shown in this host frame.
    pub fn end_frame<H: Shell<Surface = S>>(&mut self, shell: &mut H) -> Result<(), H::Error> {
        self.draws += 1;
        for surface in self.store.trim(self.draws) {
            shell.free(surface)?;
        }
        Ok(())
    }

    /// Free every shared frame: for a host shutting down. Frames asked of the painter and not delivered are
    /// forgotten; one delivered later is held again.
    pub fn clear<H: Shell<Surface = S>>(&mut self, shell: &mut H) -> Result<(), H::Error> {
        self.pending.clear();
        for surface in self.store.clear() {
            shell.free(surface)?;
        }
        Ok(())
    }
}

impl<S> Default for Frames<S, InlinePainter> {
    fn default() -> Self {
        Self::new(InlinePainter::default())
    }
}
