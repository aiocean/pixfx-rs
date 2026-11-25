//! pixfx in Bevy: live effects drawn among a game's sprites, colour with normal blending and light added, every
//! texture sampled nearest so the pixel art stays crisp.
//!
//! A [`Scene`] decides everything: which effects play, where, which frame each shows, which frames are shared and
//! when one leaves the GPU, and the game's answer to the cues they fire ([`Feel`]: shake, hit-stop, flash). This
//! crate is the Bevy shell: each frame it follows the scene's draw list with a pool of quads, and turns Bevy's
//! clock, cameras and entities into calls.
//!
//! Space: one world unit is one game pixel, and the stage's `y` runs down where Bevy's runs up, so game pixel
//! (`x`, `y`) is world (`x`, `-y`). Every method here takes and gives world positions; [`stage`] and [`world`] turn
//! the positions in a [`StageEvent`] or an [`Actor`] from one to the other. Enlarge the picture with the camera
//! (`OrthographicProjection::scale` of `1 / n`, a whole `n`) so every pixel stays one size.
#![forbid(unsafe_code)]

use std::collections::HashMap;
use std::num::NonZeroU32;

use bevy::app::{App, First, Plugin, PostUpdate};
use bevy::asset::{AssetApp, Assets, Handle, embedded_asset};
use bevy::camera::visibility::VisibilitySystems;
use bevy::camera::{CameraUpdateSystems, OrthographicProjection, Projection};
use bevy::color::Color;
use bevy::ecs::message::{Message, MessageWriter};
use bevy::ecs::prelude::*;
use bevy::image::Image;
use bevy::math::{Quat, Rect, Vec2, Vec3};
use bevy::mesh::{Mesh, Mesh2d};
use bevy::platform::cell::SyncCell;
use bevy::camera::visibility::Visibility;
use bevy::sprite_render::{AlphaMode2d, ColorMaterial, Material2dPlugin, MeshMaterial2d};
use bevy::time::Time;
use bevy::transform::TransformSystems;
use bevy::transform::components::{GlobalTransform, Transform};
use pixfx_core::volley::{Aim, Order};
use pixfx_core::{Actor, Clip, Effect, Feel, Layer, Options};
use pixfx_scene::{Body, Frames, InlinePainter, Scene, StageError, StageEvent, View};

mod shell;

pub use shell::{BevyShell, ColorLayer, LightLayer, Part, Shown, Surface};

/// Game pixel (`x`, `y`) of the stage as a world position.
pub fn world(x: f64, y: f64) -> Vec2 {
    Vec2::new(x as f32, -y as f32)
}

/// World position `at` as a game pixel of the stage.
pub fn stage(at: Vec2) -> (f64, f64) {
    (f64::from(at.x), -f64::from(at.y))
}

/// The z an effect's layers draw at, among the game's sprites: `back` behind the character it plays on, `front`
/// before it. Layers at the same z draw in the order the scene shows them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Depth {
    pub back: f32,
    pub front: f32,
}

impl Default for Depth {
    fn default() -> Self {
        Self {
            back: 0.0,
            front: 1.0,
        }
    }
}

/// One effect playing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Fx(pub u32);

/// Something that happened on the stage ([`StageEvent`], positions in game pixels: [`world`] places them): an
/// effect ended, fired a cue (a sound is the game's to play), or a volley's piece struck.
#[derive(Message, Clone, Copy, Debug, PartialEq)]
pub struct PixfxEvent(pub StageEvent);

/// The camera the feel moves: a shake and a roll for the render only (the game's own `Transform` is left as it
/// was), and a flash over its view. The one camera with it also bounds what is drawn: effects outside its view play
/// on undrawn.
#[derive(Component, Debug, Default)]
pub struct PixfxCamera {
    /// Where the camera was before this frame's shake, put back at the start of the next.
    before: Option<GlobalTransform>,
    flash: Option<Entity>,
}

struct Follow {
    target: Entity,
    offset: Vec2,
}

/// Every effect of the game: a scene and the frames it draws from. Play, move and stop effects from any system.
#[derive(Resource)]
pub struct Pixfx {
    frames: SyncCell<Frames<Surface, InlinePainter>>,
    scene: Scene<Surface>,
    depths: HashMap<u32, Depth>,
    follows: HashMap<u32, Follow>,
    /// What the scene showed in the last draw, in order.
    shown: Vec<Shown>,
    /// Effect seconds per real second in the last update: 0 in a hit-stop.
    time_scale: f64,
    shake_strength: f64,
}

impl Default for Pixfx {
    fn default() -> Self {
        let mut frames = Frames::new(InlinePainter::default());
        // one world unit per game pixel: the camera enlarges
        let scene = frames.scene(NonZeroU32::MIN);
        Self {
            frames: SyncCell::new(frames),
            scene,
            depths: HashMap::new(),
            follows: HashMap::new(),
            shown: Vec::new(),
            time_scale: 1.0,
            shake_strength: 1.0,
        }
    }
}

impl Pixfx {
    /// Play `effect` with its origin at world `at`.
    pub fn play(
        &mut self,
        effect: &'static dyn Effect,
        opts: Options,
        at: Vec2,
    ) -> Result<Fx, StageError> {
        let (x, y) = stage(at);
        self.scene.play(Clip::live(effect, opts), x, y).map(Fx)
    }

    /// Draw `fx`'s layers at `depth` from now on.
    pub fn set_depth(&mut self, fx: Fx, depth: Depth) {
        self.depths.insert(fx.0, depth);
    }

    /// Put `fx`'s origin at world `at`, keeping its time. False if it is gone.
    pub fn move_to(&mut self, fx: Fx, at: Vec2) -> Result<bool, StageError> {
        let (x, y) = stage(at);
        self.scene.move_to(fx.0, x, y)
    }

    /// Keep `fx` at `target`'s place plus `offset` (world units) each frame, after the game moved it; `None` lets
    /// it be. When `target` is despawned the effect stops where it was.
    pub fn follow(&mut self, fx: Fx, target: Option<Entity>, offset: Vec2) {
        match target {
            Some(target) => self.follows.insert(fx.0, Follow { target, offset }),
            None => self.follows.remove(&fx.0),
        };
    }

    /// Play it on a character: `actor` is its sprite's frame as the game shows it now, placed in game pixels, or
    /// `None` for none. An effect that wraps its actor (`feedback/power-up`) grips that outline from its next
    /// step; others ignore it. The same sprite at the same place changes nothing, so set it every frame. False if
    /// it is gone.
    pub fn set_actor(&mut self, fx: Fx, actor: Option<Actor>) -> Result<bool, StageError> {
        self.scene.set_actor(fx.0, actor)
    }

    /// Let it end: loops fade out, one-shots are cut short. False if it is gone.
    pub fn stop(&mut self, fx: Fx) -> bool {
        self.scene.stop(fx.0)
    }

    /// Drop it now. False if it is gone.
    pub fn remove(&mut self, fx: Fx) -> bool {
        self.forget(fx.0);
        self.scene.remove(fx.0)
    }

    /// Whether it finished (or was removed).
    pub fn done(&self, fx: Fx) -> bool {
        !self.scene.handles().any(|h| h == fx.0)
    }

    /// Order `fx` to attack toward world `at`, at any distance: an effect that takes orders
    /// ([`pixfx_core::Effect::order`]) launches its volley, whose pieces strike the bodies they meet
    /// ([`Pixfx::set_body`]) but `caster`. False if it takes no orders, was ordered already, or is gone.
    pub fn order(
        &mut self,
        fx: Fx,
        at: Vec2,
        aim: Aim,
        caster: Option<u32>,
    ) -> Result<bool, StageError> {
        let (x, y) = stage(at);
        self.scene.order(fx.0, Order { x, y, aim, caster })
    }

    /// Body `id` (a fighter, a wall) is the world box `body`, or none to drop it: volley pieces that meet it
    /// strike it. Set it as it moves.
    pub fn set_body(&mut self, id: u32, body: Option<Rect>) {
        let body = body.map(|r| {
            let (x, y) = stage(Vec2::new(r.min.x, r.max.y));
            Body {
                x,
                y,
                w: f64::from(r.width()),
                h: f64::from(r.height()),
            }
        });
        self.scene.set_body(id, body);
    }

    /// The game's own collisions say volley piece `piece` hit something: it strikes where its tip is and goes.
    /// False if it is not a piece in flight.
    pub fn burst(&mut self, piece: Fx) -> Result<bool, StageError> {
        self.scene.burst(piece.0)
    }

    /// Lay `effect` (`projectiles/trail`) under `mover` as its trail, at `mover`'s depth. None if `mover` is gone
    /// or `effect` draws no trail.
    pub fn trail(
        &mut self,
        mover: Fx,
        effect: &'static dyn Effect,
        opts: Options,
    ) -> Result<Option<Fx>, StageError> {
        let Some(handle) = self.scene.trail(mover.0, Clip::live(effect, opts))? else {
            return Ok(None);
        };
        if let Some(&depth) = self.depths.get(&mover.0) {
            self.depths.insert(handle, depth);
        }
        Ok(Some(Fx(handle)))
    }

    /// What the volley `fx` was ordered to launch still plays: its shots in flight and their hits.
    pub fn launched(&self, fx: Fx) -> Vec<Fx> {
        self.scene.launched(fx.0).map(Fx).collect()
    }

    /// The answer to the cues the effects fire: shake, hit-stop, slow motion, flash.
    pub fn feel(&self) -> Option<&Feel> {
        self.scene.feel()
    }

    /// The game's own impacts ([`Feel::add_trauma`], [`Feel::freeze`], [`Feel::slow`]).
    pub fn feel_mut(&mut self) -> Option<&mut Feel> {
        self.scene.feel_mut()
    }

    /// Answer cues with `feel`, or with nothing: the game answers the cue events itself.
    pub fn set_feel(&mut self, feel: Option<Feel>) {
        self.scene.set_feel(feel);
    }

    /// The player's shake setting: 1 as designed, 0 holds the screen still.
    pub fn set_shake_strength(&mut self, strength: f64) {
        self.shake_strength = strength;
    }

    /// How fast the effects played in the last frame, as a share of real time: 0 in a hit-stop, below 1 in slow
    /// motion. Scale the game's own world by it to freeze with the effects.
    pub fn time_scale(&self) -> f64 {
        self.time_scale
    }

    /// Bytes the shared frames may hold ([`pixfx_core::FRAME_BUDGET`] unless set).
    pub fn set_budget(&mut self, bytes: u64) {
        self.frames.get().set_budget(bytes);
    }

    /// Bytes the shared frames hold.
    pub fn bytes(&mut self) -> u64 {
        self.frames.get().bytes()
    }

    /// Draw at most `layers` layers per frame, or every changed one with `None` (the default); effects left over
    /// show their last frame and wait their turn.
    pub fn set_draw_budget(&mut self, layers: Option<NonZeroU32>) {
        self.scene.set_draw_budget(layers);
    }

    /// The scene, for what this shell does not wrap.
    pub fn scene_mut(&mut self) -> &mut Scene<Surface> {
        &mut self.scene
    }

    /// What the last draw showed, in order.
    pub fn shown(&self) -> &[Shown] {
        &self.shown
    }

    /// Let `dt` seconds pass: the effects advance, cues go to the feel, and every event since the last update is
    /// returned. What [`PixfxPlugin`] runs each frame with Bevy's clock.
    pub fn update(&mut self, dt: f64) -> Result<Vec<StageEvent>, StageError> {
        let played = self.scene.update(dt)?;
        self.time_scale = if dt > 0.0 { played / dt } else { 1.0 };
        let events = self.scene.drain_events();
        for e in &events {
            if let StageEvent::End { handle } = e {
                self.forget(*handle);
            }
        }
        Ok(events)
    }

    /// Have the scene show every effect into `shell`'s assets; [`Pixfx::shown`] then lists what to draw.
    pub fn draw(
        &mut self,
        images: &mut Assets<Image>,
        colors: &mut Assets<ColorLayer>,
        lights: &mut Assets<LightLayer>,
    ) {
        self.shown.clear();
        let mut shell = BevyShell {
            images,
            colors,
            lights,
            shown: &mut self.shown,
        };
        let frames = self.frames.get();
        // the shell cannot fail
        let Ok(()) = self.scene.draw(frames, &mut shell);
        let Ok(()) = frames.end_frame(&mut shell);
    }

    /// Drop every effect and every surface.
    pub fn clear(
        &mut self,
        images: &mut Assets<Image>,
        colors: &mut Assets<ColorLayer>,
        lights: &mut Assets<LightLayer>,
    ) {
        self.shown.clear();
        let mut shell = BevyShell {
            images,
            colors,
            lights,
            shown: &mut self.shown,
        };
        let frames = self.frames.get();
        let Ok(()) = self.scene.clear(frames, &mut shell);
        let Ok(()) = frames.clear(&mut shell);
        self.depths.clear();
        self.follows.clear();
    }

    fn forget(&mut self, handle: u32) {
        self.depths.remove(&handle);
        self.follows.remove(&handle);
    }

    fn depth(&self, handle: u32, layer: Layer) -> f32 {
        let d = self.depths.get(&handle).copied().unwrap_or_default();
        if layer == Layer::Back { d.back } else { d.front }
    }
}

/// Plays pixfx effects in a Bevy app: add it, then play through the [`Pixfx`] resource. Each frame, after the game
/// moved its entities, the effects follow their targets and advance on Bevy's clock ([`Time`]), the stage's events
/// go out as [`PixfxEvent`]s, and the scene's draw list is shown with a pool of quads.
pub struct PixfxPlugin;

/// The systems that run the effects, in `PostUpdate` once transforms are propagated.
#[derive(SystemSet, Clone, Debug, PartialEq, Eq, Hash)]
pub struct PixfxSystems;

impl Plugin for PixfxPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "pixfx.wesl");
        app.add_plugins((
            Material2dPlugin::<ColorLayer>::default(),
            Material2dPlugin::<LightLayer>::default(),
        ))
        .init_asset::<ColorLayer>()
        .init_asset::<LightLayer>()
        .init_resource::<Pixfx>()
        .init_resource::<Pool>()
        .add_message::<PixfxEvent>()
        .add_systems(First, restore_cameras)
        .configure_sets(
            PostUpdate,
            PixfxSystems
                .after(TransformSystems::Propagate)
                .after(CameraUpdateSystems)
                .before(VisibilitySystems::UpdateFrusta)
                .before(VisibilitySystems::CheckVisibility),
        )
        .add_systems(
            PostUpdate,
            (advance, draw, feel_cameras).chain().in_set(PixfxSystems),
        );
    }
}

/// The quads the draw list is shown with, reused frame to frame: a colour and a light quad per shown surface.
#[derive(Resource, Default)]
struct Pool {
    quad: Option<Handle<Mesh>>,
    pairs: Vec<(Entity, Entity)>,
}

/// One quad of the pool.
#[derive(Component)]
struct PoolQuad;

/// A camera's flash.
#[derive(Component)]
struct Flash;

/// The world box `camera` sees.
fn camera_view(at: &GlobalTransform, projection: &Projection) -> Option<Rect> {
    let Projection::Orthographic(OrthographicProjection { area, .. }) = projection else {
        return None;
    };
    let t = at.translation().truncate();
    Some(Rect::from_corners(area.min + t, area.max + t))
}

fn advance(
    mut pixfx: ResMut<Pixfx>,
    time: Res<Time>,
    targets: Query<&GlobalTransform>,
    cameras: Query<(&GlobalTransform, &Projection), With<PixfxCamera>>,
    mut out: MessageWriter<PixfxEvent>,
) {
    let pixfx = &mut *pixfx;
    let mut gone = Vec::new();
    for (&handle, f) in &pixfx.follows {
        match targets.get(f.target) {
            Ok(at) => {
                let (x, y) = stage(at.translation().truncate() + f.offset);
                let _ = pixfx.scene.move_to(handle, x, y);
            }
            Err(_) => gone.push(handle),
        }
    }
    for handle in gone {
        pixfx.follows.remove(&handle);
        pixfx.scene.stop(handle);
    }
    // the one feel camera bounds what is drawn; with none or several, everything is
    let view = match cameras.single() {
        Ok((at, projection)) => camera_view(at, projection).map(|r| {
            let (x, y) = stage(Vec2::new(r.min.x, r.max.y));
            View {
                x,
                y,
                w: f64::from(r.width()),
                h: f64::from(r.height()),
                scroll_x: 0.0,
                scroll_y: 0.0,
            }
        }),
        Err(_) => None,
    };
    let _ = pixfx.scene.set_view(view);
    if let Some(w) = view.map(|v| v.w) {
        pixfx.scene.set_screen_w(w);
    }
    if let Ok(events) = pixfx.update(time.delta_secs_f64()) {
        out.write_batch(events.into_iter().map(PixfxEvent));
    }
}

/// Each layer's quads sit this far apart in z, so equal depths draw in the scene's order.
const Z_STEP: f32 = 1.0e-5;

#[allow(clippy::too_many_arguments)]
fn draw(
    mut commands: Commands,
    mut pixfx: ResMut<Pixfx>,
    mut pool: ResMut<Pool>,
    mut images: ResMut<Assets<Image>>,
    mut colors: ResMut<Assets<ColorLayer>>,
    mut lights: ResMut<Assets<LightLayer>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut quads: Quads,
    mut color_quads: Query<&mut MeshMaterial2d<ColorLayer>>,
    mut light_quads: Query<&mut MeshMaterial2d<LightLayer>>,
) {
    pixfx.draw(&mut images, &mut colors, &mut lights);
    let quad = pool
        .quad
        .get_or_insert_with(|| meshes.add(bevy::prelude::Rectangle::new(1.0, 1.0)))
        .clone();
    let pixfx = &*pixfx;
    let mut used = 0;
    for (i, shown) in pixfx.shown.iter().enumerate() {
        let pass = shown.pass;
        let s = &shown.surface;
        let depth = pixfx.depth(pass.handle, pass.layer) + i as f32 * 2.0 * Z_STEP;
        // canvas pixels per frame pixel: 1 at the scene's scale of 1
        let sx = pass.rect.w as f32 / s.frame.0.max(1) as f32;
        let sy = pass.rect.h as f32 / s.frame.1.max(1) as f32;
        let place = |cut: pixfx_core::PixelBox, z: f32| {
            let (w, h) = (cut.w as f32 * sx, cut.h as f32 * sy);
            let x = pass.rect.x as f32 + cut.x as f32 * sx + w / 2.0;
            let y = pass.rect.y as f32 + cut.y as f32 * sy + h / 2.0;
            Transform::from_xyz(x, -y, z).with_scale(Vec3::new(w, h, 1.0))
        };
        if used == pool.pairs.len() {
            let color = commands
                .spawn((
                    PoolQuad,
                    Mesh2d(quad.clone()),
                    MeshMaterial2d::<ColorLayer>::default(),
                    Transform::default(),
                    Visibility::Hidden,
                ))
                .id();
            let light = commands
                .spawn((
                    PoolQuad,
                    Mesh2d(quad.clone()),
                    MeshMaterial2d::<LightLayer>::default(),
                    Transform::default(),
                    Visibility::Hidden,
                ))
                .id();
            pool.pairs.push((color, light));
        }
        let Some(&(color, light)) = pool.pairs.get(used) else {
            continue;
        };
        used += 1;
        let color_at = s.color.as_ref().map(|p| (place(p.cut, depth), p.material.clone()));
        let light_at = s
            .light
            .as_ref()
            .map(|p| (place(p.cut, depth + Z_STEP), p.material.clone()));
        match color_at {
            Some((at, material)) => {
                show(&mut commands, &mut quads, color, at);
                set_material(&mut commands, &mut color_quads, color, material);
            }
            None => hide(&mut quads, color),
        }
        match light_at {
            Some((at, material)) => {
                show(&mut commands, &mut quads, light, at);
                set_material(&mut commands, &mut light_quads, light, material);
            }
            None => hide(&mut quads, light),
        }
    }
    for &(color, light) in pool.pairs.iter().skip(used) {
        hide(&mut quads, color);
        hide(&mut quads, light);
    }
}

type QuadData = (
    &'static mut Transform,
    &'static mut GlobalTransform,
    &'static mut Visibility,
);
type Quads<'w, 's> = Query<'w, 's, QuadData, With<PoolQuad>>;

/// Place quad `e` at `at` and show it: its global transform too, since transforms were propagated already.
fn show(commands: &mut Commands, quads: &mut Quads, e: Entity, at: Transform) {
    match quads.get_mut(e) {
        Ok((mut t, mut g, mut v)) => {
            if *t != at {
                *t = at;
                *g = GlobalTransform::from(at);
            }
            if *v != Visibility::Inherited {
                *v = Visibility::Inherited;
            }
        }
        // made this frame: its components land with the commands
        Err(_) => {
            commands
                .entity(e)
                .insert((at, GlobalTransform::from(at), Visibility::Inherited));
        }
    }
}

fn hide(quads: &mut Quads, e: Entity) {
    if let Ok((_, _, mut v)) = quads.get_mut(e)
        && *v != Visibility::Hidden
    {
        *v = Visibility::Hidden;
    }
}

fn set_material<M: bevy::sprite_render::Material2d>(
    commands: &mut Commands,
    quads: &mut Query<&mut MeshMaterial2d<M>>,
    e: Entity,
    material: Handle<M>,
) {
    match quads.get_mut(e) {
        Ok(mut m) => {
            if m.0 != material {
                m.0 = material;
            }
        }
        Err(_) => {
            commands.entity(e).insert(MeshMaterial2d(material));
        }
    }
}

/// Put every shaken camera back where the game left it, before the game moves it.
fn restore_cameras(mut cameras: Query<(&mut PixfxCamera, &mut GlobalTransform)>) {
    for (mut camera, mut at) in &mut cameras {
        if let Some(before) = camera.before.take() {
            *at = before;
        }
    }
}

/// Shake and roll each feel camera for this frame's render, and lay the flash over its view.
fn feel_cameras(
    mut commands: Commands,
    pixfx: Res<Pixfx>,
    pool: Res<Pool>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut cameras: Query<(&mut PixfxCamera, &mut GlobalTransform, &Projection), Without<Flash>>,
    mut flashes: Query<
        (&mut GlobalTransform, &mut Visibility, &MeshMaterial2d<ColorMaterial>),
        With<Flash>,
    >,
) {
    let Some(feel) = pixfx.feel() else {
        return;
    };
    let (dx, dy) = feel.shake(pixfx.shake_strength);
    let roll = feel.roll(pixfx.shake_strength);
    let flash = feel.flash() as f32;
    for (mut camera, mut at, projection) in &mut cameras {
        let view = camera_view(&at, projection);
        if dx != 0 || dy != 0 || roll != 0.0 {
            camera.before = Some(*at);
            // y runs down on the stage, so its clockwise roll is a negative turn here
            let shaken = Transform::from_translation(Vec3::new(dx as f32, -dy as f32, 0.0))
                .with_rotation(Quat::from_rotation_z(-roll as f32));
            *at = GlobalTransform::from(at.compute_transform().mul_transform(shaken));
        }
        let Some(view) = view else {
            continue;
        };
        // over everything the camera sees, at the far end of its depth
        let z = at.translation().z + 999.0;
        let cover = Transform::from_translation(view.center().extend(z))
            .with_scale((view.size() * 2.0).extend(1.0));
        let shown = flashes.get_mut(camera.flash.unwrap_or(Entity::PLACEHOLDER));
        match shown {
            Ok((mut g, mut v, m)) => {
                let visible = if flash > 0.0 { Visibility::Inherited } else { Visibility::Hidden };
                if *v != visible {
                    *v = visible;
                }
                if flash > 0.0 {
                    *g = GlobalTransform::from(cover);
                    if let Some(mut m) = materials.get_mut(&m.0) {
                        m.color = Color::srgba(1.0, 1.0, 1.0, flash);
                    }
                }
            }
            Err(_) if flash > 0.0 => {
                let Some(quad) = pool.quad.clone() else {
                    continue;
                };
                let material = materials.add(ColorMaterial {
                    color: Color::srgba(1.0, 1.0, 1.0, flash),
                    alpha_mode: AlphaMode2d::Blend,
                    ..ColorMaterial::default()
                });
                let e = commands
                    .spawn((
                        Flash,
                        Mesh2d(quad),
                        MeshMaterial2d(material),
                        cover,
                        GlobalTransform::from(cover),
                    ))
                    .id();
                camera.flash = Some(e);
            }
            Err(_) => {}
        }
    }
}
