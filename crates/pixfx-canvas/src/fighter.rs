//! `pixfx-fighter` for JS: the screen's answer to a fight ([`Feel`]), a fighter's body ([`Body`]) and the drawing
//! of fighters ([`Sprites`]). Every object JS passes is read once, strictly, into the Rust types
//! ([`pixfx_wasm::js::parse`]): a misspelt, missing or mistyped field throws, naming it.

use js_sys::{Array, Function, Object};
use pixfx_fighter::feel::{self, Cue};
use pixfx_wasm::js::{pair, parse, to_js};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

/// What a fight looks and feels like on screen: hit-stop, shake, the struck one's shudder, damage numbers, dust,
/// a knockout's slow motion, an ultimate's dusk. `FightFeel` in JS: `Feel` there is the engine's answer for any
/// game.
#[wasm_bindgen(js_name = FightFeel)]
pub struct Feel {
    pub(crate) inner: feel::Feel,
}

impl Default for Feel {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen(js_class = FightFeel)]
impl Feel {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Feel {
        Feel {
            inner: feel::Feel::new(),
        }
    }

    /// Take in what the fight just did: an array of the fight's events. Each whose `type` is a cue's
    /// ([`Cue::TYPES`]) is read as that cue, its fields strictly (fields the cue does not read, which other parts
    /// of the game read off the same event, are let be); events of other types (effect commands) are skipped.
    pub fn react(
        &mut self,
        #[wasm_bindgen(unchecked_param_type = "Array<Cue | { type: string }>")] events: &Array,
    ) -> Result<(), JsError> {
        let mut cues = Vec::new();
        for (i, e) in events.iter().enumerate() {
            let what = format!("events[{i}]");
            let Typed { ty } = parse(&what, &e)?;
            if Cue::TYPES.contains(&ty.as_str()) {
                cues.push(parse(&what, &e)?);
            }
        }
        self.inner.react(&cues);
        Ok(())
    }

    /// Let `dt` real seconds pass; returns how many seconds the fight should run for them.
    pub fn advance(&mut self, dt: f64) -> f64 {
        self.inner.advance(dt)
    }

    /// Where the screen sits this frame, `[dx, dy]` in game pixels. `strength` is the player's shake setting: 1 as
    /// designed, 0 holds the screen still.
    #[wasm_bindgen(unchecked_return_type = "[number, number]")]
    pub fn shake(&self, strength: f64) -> Array {
        let (x, y) = self.inner.shake(strength);
        pair(x.into(), y.into())
    }

    /// How far each fighter is jolted off its place: `[[dx, dy], [dx, dy]]`.
    #[wasm_bindgen(unchecked_return_type = "[number, number][]")]
    pub fn jolts(&self) -> Array {
        self.inner
            .jolts()
            .iter()
            .map(|&(x, y)| Array::of2(&x.into(), &y.into()))
            .collect()
    }

    /// How dark an ultimate's dusk makes the sky at its darkest.
    #[wasm_bindgen(js_name = duskDepth)]
    pub fn dusk_depth() -> f64 {
        feel::FeelSpec::default().dusk.depth
    }

    /// How dark the sky is, 0 up to the dusk's depth.
    pub fn dusk(&self) -> f64 {
        self.inner.dusk()
    }

    /// Each floating number: `{x, y, alpha, text, by, kind}`, `y` up from the ground, `by` null for a dodge.
    #[wasm_bindgen(unchecked_return_type = "DamageNumber[]")]
    pub fn numbers(&self) -> Result<JsValue, JsError> {
        to_js(&self.inner.numbers())
    }

    /// Each qi streak: `{x0, y0, x1, y1, by, fade}`.
    #[wasm_bindgen(unchecked_return_type = "Streak[]")]
    pub fn streaks(&self) -> Result<JsValue, JsError> {
        to_js(&self.inner.streaks())
    }

    /// How shaken the screen is, 0 to 1.
    pub fn trauma(&self) -> f64 {
        self.inner.trauma()
    }
}

/// Just an event's `type`, to tell cues from the game's other events.
#[derive(Deserialize)]
struct Typed {
    #[serde(rename = "type")]
    ty: String,
}

// ---------- the body ----------

use pixfx_fighter::body::{self as b, Arena, Blow, BodySpec, Doing, Drive};

/// A fighter's body for JS: its fields as properties, the duel's body steps as methods. Poses are the strings the
/// site's duel uses (`"idle"`, `"walk"`, … `"win"`).
#[wasm_bindgen]
pub struct Body {
    pub(crate) inner: b::Body,
}

macro_rules! fields {
    ($($get:ident / $set:ident : $js:literal => $f:ident),* $(,)?) => {
        #[wasm_bindgen]
        impl Body {
            $(
                #[wasm_bindgen(getter = $js)]
                pub fn $get(&self) -> f64 { self.inner.$f }
                #[wasm_bindgen(setter = $js)]
                pub fn $set(&mut self, v: f64) { self.inner.$f = v; }
            )*
        }
    };
}

fields! {
    x / set_x: "x" => x,
    air / set_air: "air" => air,
    vx / set_vx: "vx" => vx,
    vy / set_vy: "vy" => vy,
    face / set_face: "face" => face,
    stun / set_stun: "stun" => stun,
    wake / set_wake: "wake" => wake,
    land / set_land: "land" => land,
    squat / set_squat: "squat" => squat,
    leap / set_leap: "leap" => leap,
    duck / set_duck: "duck" => duck,
    flash / set_flash: "flash" => flash,
    invincible / set_invincible: "invincible" => invincible,
    hit_t / set_hit_t: "hitT" => hit_t,
    hit_power / set_hit_power: "hitPower" => hit_power,
    frozen / set_frozen: "frozen" => frozen,
    shocked / set_shocked: "shocked" => shocked,
    pose_t / set_pose_t: "poseT" => pose_t,
}

/// The default spec with the fields `o` gives over it (`{gravity: 400}`), read strictly; `undefined`/`null` is the
/// default itself.
fn spec_of(o: &JsValue) -> Result<BodySpec, JsError> {
    if o.is_undefined() || o.is_null() {
        return Ok(BodySpec::default());
    }
    if !o.is_object() {
        return Err(JsError::new("body spec: expected an object"));
    }
    let defaults: Object = to_js(&BodySpec::default())?.unchecked_into();
    parse(
        "body spec",
        &Object::assign(&defaults, &Object::from(o.clone())).into(),
    )
}

#[wasm_bindgen]
impl Body {
    /// A body standing at `x`, facing right, built as `spec` says: omitted, the site's duel's body
    /// ([`Body::defaults`]); an object, those defaults with the numbers it gives over them (`{gravity: 400}`).
    #[wasm_bindgen(constructor)]
    pub fn new(
        x: f64,
        #[wasm_bindgen(unchecked_optional_param_type = "Partial<BodySpec>")] spec: JsValue,
    ) -> Result<Body, JsError> {
        Ok(Body {
            inner: b::Body::with_spec(x, spec_of(&spec)?),
        })
    }

    /// The default body's numbers, `{body, chest, halfWidth, walk, jumpSpeed, gravity, …}`: the rules read them
    /// instead of copying them, and pass back the ones they change to the constructor.
    #[wasm_bindgen(unchecked_return_type = "BodySpec")]
    pub fn defaults() -> Result<JsValue, JsError> {
        to_js(&BodySpec::default())
    }

    /// This body's numbers, as [`Body::defaults`] gives them.
    #[wasm_bindgen(getter, unchecked_return_type = "BodySpec")]
    pub fn spec(&self) -> Result<JsValue, JsError> {
        to_js(&self.inner.spec)
    }

    #[wasm_bindgen(getter, unchecked_return_type = "Pose")]
    pub fn pose(&self) -> Result<JsValue, JsError> {
        to_js(&self.inner.pose)
    }

    /// Off its feet: `{phase, t, juggle, light, spin, bounced, quick, dir}` (a copy), or null.
    #[wasm_bindgen(getter, unchecked_return_type = "Down | null")]
    pub fn down(&self) -> Result<JsValue, JsError> {
        to_js(&self.inner.down)
    }

    /// A new round: back on its feet at `x`.
    #[wasm_bindgen(js_name = startRound)]
    pub fn start_round(&mut self, x: f64) {
        self.inner.start_round(x);
    }

    /// The body's clocks run on; a fighter that is down lies, rolls or gets up.
    pub fn tick(&mut self, dt: f64, alive: bool) {
        self.inner.tick(dt, alive);
    }

    /// Feet and flight for one step. `drive`: `{alive, acting, meditating, guarding, chilled, intent, foeX,
    /// motion: "gravity" | {fly: hover} | {dive: speed, floor}}`; `arena`: `{left, right, top}`; `rand()` in
    /// [0, 1). Returns the events: `{type: "land", x, strength}`, `{type: "bump", x, y, strength}`.
    #[wasm_bindgen(unchecked_return_type = "BodyEvent[]")]
    pub fn step(
        &mut self,
        dt: f64,
        #[wasm_bindgen(unchecked_param_type = "Drive")] drive: &JsValue,
        #[wasm_bindgen(unchecked_param_type = "Arena")] arena: &JsValue,
        rand: &Function,
    ) -> Result<JsValue, JsError> {
        let drive: Drive = parse("drive", drive)?;
        let arena: Arena = parse("arena", arena)?;
        let (events, failed) = with_roll(rand, |roll| self.inner.step(dt, &drive, &arena, roll));
        failed?;
        to_js(&events)
    }

    /// A blow lands: launch or stagger, knockback, the white flash.
    pub fn hit(&mut self, power: f64, dir: f64, freeze: bool, flying: bool) {
        self.inner.hit(Blow {
            power,
            dir,
            freeze,
            flying,
        });
    }

    /// Two bodies cannot stand in one place: part them (the caller skips this when either is knocked out).
    pub fn separate(
        a: &mut Body,
        b: &mut Body,
        #[wasm_bindgen(unchecked_param_type = "Arena")] arena: &JsValue,
    ) -> Result<(), JsError> {
        b::separate(&mut a.inner, &mut b.inner, &parse("arena", arena)?);
        Ok(())
    }

    /// Whether a blow of `power` throws a default body (a freezing one never does).
    pub fn heavy(power: f64, freeze: bool) -> bool {
        Blow {
            power,
            dir: 1.0,
            freeze,
            flying: false,
        }
        .heavy(&BodySpec::default())
    }

    /// The knockout throws the loser off its feet, away from the foe at `foe_x`.
    #[wasm_bindgen(js_name = knockOut)]
    pub fn knock_out(&mut self, foe_x: f64) {
        self.inner.knock_out(foe_x);
    }

    /// Between rounds: slide, fall and lie, or stand over the loser (`won`).
    #[wasm_bindgen(unchecked_return_type = "BodyEvent[]")]
    pub fn settle(
        &mut self,
        dt: f64,
        won: bool,
        alive: bool,
        foe_x: f64,
        #[wasm_bindgen(unchecked_param_type = "Arena")] arena: &JsValue,
        rand: &Function,
    ) -> Result<JsValue, JsError> {
        let arena: Arena = parse("arena", arena)?;
        let (events, failed) = with_roll(rand, |roll| {
            self.inner.settle(dt, won, alive, foe_x, &arena, roll)
        });
        failed?;
        to_js(&events)
    }

    pub fn reeling(&self) -> bool {
        self.inner.reeling()
    }

    pub fn floored(&self) -> bool {
        self.inner.floored()
    }

    pub fn untouchable(&self) -> bool {
        self.inner.untouchable()
    }

    /// Seconds until it can act again.
    #[wasm_bindgen(js_name = outFor)]
    pub fn out_for(&self) -> f64 {
        self.inner.out_for()
    }

    /// Seconds until a blow can touch it.
    #[wasm_bindgen(js_name = openIn)]
    pub fn open_in(&self) -> f64 {
        self.inner.open_in()
    }

    /// Settle its pose from what the rules have it doing: `"windup"`, `"recover"` (with `melee`), `"guard"`,
    /// `"meditate"`, `"fly"`, `"walk"` or `"idle"`. A new pose restarts the pose clock.
    #[wasm_bindgen(js_name = updatePose)]
    pub fn update_pose(
        &mut self,
        #[wasm_bindgen(
            unchecked_param_type = "\"windup\" | \"recover\" | \"guard\" | \"meditate\" | \"fly\" | \"walk\" | \"idle\""
        )]
        doing: &str,
        melee: bool,
        alive: bool,
    ) -> Result<(), JsError> {
        let doing = match doing {
            "windup" => Doing::Windup { melee },
            "recover" => Doing::Recover { melee },
            "guard" => Doing::Guard,
            "meditate" => Doing::Meditate,
            "fly" => Doing::Fly,
            "walk" => Doing::Walk,
            "idle" => Doing::Idle,
            other => return Err(JsError::new(&format!("unknown doing {other:?}"))),
        };
        let pose = self.inner.pose_of(doing, alive);
        self.inner.set_pose(pose);
        Ok(())
    }
}

/// Run `f` with a roll that calls JS `rand()`; the error says if `rand` failed or gave a non-number.
fn with_roll<T>(
    rand: &Function,
    f: impl FnOnce(&mut dyn FnMut() -> f64) -> T,
) -> (T, Result<(), JsError>) {
    let mut bad = false;
    let mut roll = || match rand.call0(&JsValue::NULL).map(|v| v.as_f64()) {
        Ok(Some(v)) => v,
        _ => {
            bad = true;
            0.0
        }
    };
    let out = f(&mut roll);
    let status = if bad {
        Err(JsError::new("rand() must return a number"))
    } else {
        Ok(())
    };
    (out, status)
}

// ---------- drawing fighters ----------

use crate::Surface;
use crate::stage::{Pass, Rect};
use pixfx_core::Layer;
use pixfx_draw::{Frame, Rgba};
use pixfx_fighter::look::Look;
use pixfx_fighter::paint::{self, Appearance};
use pixfx_fighter::status::{self, DomeStyle, Figure, Mark, Silhouette};
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};

/// Fighters, their marks and the dust they raise, drawn onto a canvas in game pixels upscaled by `scale`, colour
/// normally then light additively, like a [`crate::Stage`]'s effects. Draw into it between `begin()` and `end()`;
/// `end()` puts what was drawn onto the canvas as it stands (in the context's current transform), so a host
/// layers it where it wants among its own drawing.
#[wasm_bindgen]
pub struct Sprites {
    ctx: CanvasRenderingContext2d,
    scale: u32,
    frame: Frame,
    surface: Surface,
}

#[wasm_bindgen]
impl Sprites {
    /// Draw onto `canvas` (the same one a Stage draws on is fine) at `scale` canvas pixels per game pixel.
    #[wasm_bindgen(constructor)]
    pub fn new(canvas: HtmlCanvasElement, scale: u32) -> Result<Sprites, JsError> {
        let scale = scale.max(1);
        let ctx = canvas
            .get_context("2d")
            .map_err(|_| JsError::new("could not get the canvas 2d context"))?
            .ok_or_else(|| JsError::new("the canvas has no 2d context"))?
            .dyn_into::<CanvasRenderingContext2d>()
            .map_err(|_| JsError::new("the canvas 2d context has an unexpected type"))?;
        let frame = Frame::new(
            canvas.width().div_ceil(scale).max(1),
            canvas.height().div_ceil(scale).max(1),
            0,
            0,
        );
        let surface = Surface::new(&frame)?;
        Ok(Sprites {
            ctx,
            scale,
            frame,
            surface,
        })
    }

    /// Start a new picture: nothing drawn yet.
    pub fn begin(&mut self) {
        self.frame.clear();
    }

    /// Fighter `a` in `look` (a preset's name, `"thunder"` or `"frost"`, or a look object as `lookColors` reads
    /// it), its feet on the floor at row `ground` (game pixels). `a`:
    /// `{x, air, pose, poseT, face, vx, vy, out, flash, untouchable, frozen, shocked, hitT, hitPower, dodged, jolt: [dx, dy],
    /// act: null | {how: "summon"|"palm"|"lunge"|"prayer"|"cross", dive, progress: number|null, long}}`. `marks`:
    /// null, or `{shield, barrier, rang, burning, defDown, silence, chilled, fury, poseT, dome: "banded"|"faceted",
    /// burn, shock, ice, t, seed}`, the last five drawing fire, lightning and frost over the body at time `t`,
    /// following the shape of its pose.
    pub fn fighter(
        &mut self,
        #[wasm_bindgen(unchecked_param_type = "string | Look")] look: &JsValue,
        #[wasm_bindgen(unchecked_param_type = "Appearance")] a: &JsValue,
        ground: f64,
        #[wasm_bindgen(unchecked_param_type = "Marks | null")] marks: &JsValue,
    ) -> Result<(), JsError> {
        let look = look_of(look)?;
        let a: Appearance = parse("appearance", a)?;
        let marks: Option<Marks> = parse("marks", marks)?;
        paint::draw(&mut self.frame, &look, &a, ground);
        if let Some(marks) = marks {
            // the marks in the order the duel has always drawn them
            let mut m = Vec::new();
            if marks.shield {
                m.push(Mark::Shield);
            }
            if marks.barrier {
                m.push(Mark::Barrier {
                    style: marks.dome,
                    glow: look.glow,
                    rang: marks.rang,
                });
            }
            for (on, mark) in [
                (marks.burning, Mark::Burning),
                (marks.def_down, Mark::DefDown),
                (marks.silence, Mark::Silence),
                (marks.chilled, Mark::Chilled),
            ] {
                if on {
                    m.push(mark);
                }
            }
            m.push(Mark::Fury(marks.fury));
            let figure = Figure {
                feet_x: a.x,
                feet_y: ground - a.air,
                face: a.face,
                silhouette: Silhouette::of(a.pose),
            };
            let (t, seed) = (marks.t, marks.seed);
            if marks.burn {
                status::draw_burning(&mut self.frame, t, seed, &figure, 1.0);
            }
            if marks.ice {
                status::draw_frozen(&mut self.frame, t, seed, &figure, 1.0);
            }
            if marks.shock {
                status::draw_shocked(&mut self.frame, t, seed, &figure, 1.0);
            }
            status::draw_marks(
                &mut self.frame,
                &m,
                marks.pose_t,
                figure.feet_x,
                figure.feet_y,
            );
        }
        Ok(())
    }

    /// The dust `feel` shows, on the floor at row `ground`.
    pub fn dust(&mut self, feel: &Feel, ground: i32) {
        feel.inner.draw_dust(&mut self.frame, ground);
    }

    /// A portrait: the head of fighter `a` (an appearance as for [`Sprites::fighter`]; the site passes it idle) in
    /// look `look`, twice the sprite's size, filling the `size`-pixel square at (`x`, `y`), at opacity `alpha`.
    pub fn portrait(
        &mut self,
        #[wasm_bindgen(unchecked_param_type = "string | Look")] look: &JsValue,
        #[wasm_bindgen(unchecked_param_type = "Appearance")] a: &JsValue,
        x: i32,
        y: i32,
        size: i32,
        alpha: f64,
    ) -> Result<(), JsError> {
        let look = look_of(look)?;
        let idle: Appearance = parse("appearance", a)?;
        let mut sprite = paint::paint(&look, &idle);
        paint::outline(&mut sprite, look.outline);
        // the face and the hair round it, a quarter of the square either side of the middle, 35 pixels up
        let (hx, hy) = (paint::CX - size / 4, paint::FEET - 35);
        let (w, h) = (self.frame.w as i32, self.frame.h as i32);
        for v in 0..size {
            for u in 0..size {
                let (px, py) = (x + u, y + v);
                if px < 0 || py < 0 || px >= w || py >= h {
                    continue;
                }
                let (sx, sy) = (hx + u / 2, hy + v / 2);
                if sx < 0 || sy < 0 {
                    continue;
                }
                let Some(c) = sprite.pixel(sx as usize, sy as usize) else {
                    continue;
                };
                if c.3 == 0 {
                    continue;
                }
                let a = pixfx_draw::math::js_round(f64::from(c.3) * alpha).clamp(0.0, 255.0) as u8;
                if let Some(slot) = self.frame.color.get_mut((py * w + px) as usize) {
                    *slot = Rgba(c.0, c.1, c.2, a).pack();
                }
            }
        }
        Ok(())
    }

    /// Put what was drawn since `begin()` onto the canvas.
    pub fn end(&mut self) -> Result<(), JsError> {
        let s = u64::from(self.scale);
        let rect = Rect {
            x: 0,
            y: 0,
            w: u64::from(self.frame.w) * s,
            h: u64::from(self.frame.h) * s,
        };
        let pass = Pass {
            handle: 0,
            layer: Layer::Front,
            rect,
        };
        self.ctx.save();
        self.ctx.set_image_smoothing_enabled(false);
        let drawn = self.surface.blit(&self.ctx, pass, &self.frame);
        self.ctx.restore();
        drawn.map_err(|e| JsError::new(&e))
    }
}

/// What hangs on a fighter, for [`Sprites::fighter`]: a ward's shell, a barrier (`dome` its pattern, `rang` when a
/// blow has just rung on it), a sign for each effect on it, `fury` pips, and fire (`burn`), lightning (`shock`) and
/// frost (`ice`) over the body at time `t`, drawn from `seed`, following the shape of its pose.
#[derive(Deserialize, tsify::Tsify)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Marks {
    pub shield: bool,
    pub barrier: bool,
    pub rang: bool,
    pub burning: bool,
    pub def_down: bool,
    pub silence: bool,
    pub chilled: bool,
    pub fury: u32,
    /// Seconds since the pose began.
    pub pose_t: f64,
    pub dome: DomeStyle,
    pub burn: bool,
    pub shock: bool,
    pub ice: bool,
    pub t: f64,
    pub seed: u32,
}

/// A look's colours the host draws its own parts in, as `"#rrggbb"`.
#[derive(Serialize, tsify::Tsify)]
pub struct LookColors {
    pub glow: Rgba,
    pub outline: Rgba,
    /// Light, dark.
    pub ribbon: [Rgba; 2],
}

/// A look's colours the host draws its own parts in: `{glow, outline, ribbon: [light, dark]}` as `#rrggbb`.
/// `look` is a preset's name (`"thunder"`, `"frost"`) or a whole look: `{robe: [3], inner: [2], trim: [2],
/// hair: [2], skin: [2], boot, blade: [3], jade, ribbon: [2], glow, outline, hairdo: "topknot" | "flowing"}`, each
/// colour `"#rrggbb"`, lists light to dark.
#[wasm_bindgen(js_name = lookColors, unchecked_return_type = "LookColors")]
pub fn look_colors(
    #[wasm_bindgen(unchecked_param_type = "string | Look")] look: &JsValue,
) -> Result<JsValue, JsError> {
    let l = look_of(look)?;
    to_js(&LookColors {
        glow: l.glow,
        outline: l.outline,
        ribbon: l.ribbon,
    })
}

/// A look: a preset by name, or read from an object (see [`look_colors`]).
fn look_of(v: &JsValue) -> Result<Look, JsError> {
    match v.as_string() {
        Some(name) => Look::preset(&name).ok_or_else(|| {
            JsError::new(&format!(
                "unknown look {name:?}: expected \"thunder\", \"frost\" or a look object"
            ))
        }),
        None => parse("look", v),
    }
}
