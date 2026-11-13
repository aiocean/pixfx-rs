//! `pixfx-fighter` for JS: the screen's answer to a fight ([`Feel`]). A game passes the fight's events as plain
//! objects (`{type: "hit", by, power, launch, crit, ranged, share, damage, x, y}`, …), the same shapes the site's
//! duel emits; anything else in the list (effect commands) is ignored.

use js_sys::{Array, Function, Object, Reflect};
use pixfx_fighter::feel::{self, Cue, NumberKind};
use wasm_bindgen::prelude::*;

/// What a fight looks and feels like on screen: hit-stop, shake, the struck one's shudder, damage numbers, dust,
/// a knockout's slow motion, an ultimate's dusk.
#[wasm_bindgen]
pub struct Feel {
    pub(crate) inner: feel::Feel,
}

impl Default for Feel {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen]
impl Feel {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Feel {
        Feel {
            inner: feel::Feel::new(),
        }
    }

    /// Take in what the fight just did: an array of event objects; unknown types are skipped.
    pub fn react(&mut self, events: &Array) -> Result<(), JsError> {
        let mut cues = Vec::new();
        for e in events.iter() {
            if let Some(c) = cue(&e)? {
                cues.push(c);
            }
        }
        self.inner.react(&cues);
        Ok(())
    }

    /// Let `dt` real seconds pass; returns how many seconds the fight should run for them.
    pub fn advance(&mut self, dt: f64) -> f64 {
        self.inner.advance(dt)
    }

    /// Where the screen sits this frame, `[dx, dy]` in game pixels; `rand()` gives numbers in [0, 1).
    pub fn shake(&self, rand: &Function) -> Result<Vec<i32>, JsError> {
        let mut failed = None;
        let mut roll = || match rand.call0(&JsValue::NULL).map(|v| v.as_f64()) {
            Ok(Some(v)) => v,
            _ => {
                failed = Some(());
                0.0
            }
        };
        let (x, y) = self.inner.shake(&mut roll);
        match failed {
            Some(()) => Err(JsError::new("shake: rand() must return a number")),
            None => Ok(vec![x, y]),
        }
    }

    /// How far each fighter is jolted off its place: `[[dx, dy], [dx, dy]]`.
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
    pub fn numbers(&self) -> Result<Array, JsError> {
        self.inner
            .numbers()
            .into_iter()
            .map(|n| {
                let kind = match n.kind {
                    NumberKind::Hit => "hit",
                    NumberKind::Crit => "crit",
                    NumberKind::Tick => "tick",
                    NumberKind::Heal => "heal",
                    NumberKind::Ward => "ward",
                    NumberKind::Dodge => "dodge",
                };
                let by = n.by.map_or(JsValue::NULL, |b| JsValue::from(b as u32));
                object(&[
                    ("x", n.x.into()),
                    ("y", n.y.into()),
                    ("alpha", n.alpha.into()),
                    ("text", n.text.into()),
                    ("by", by),
                    ("kind", kind.into()),
                ])
            })
            .collect()
    }

    /// Each qi streak: `{x0, y0, x1, y1, by, fade}`.
    pub fn streaks(&self) -> Result<Array, JsError> {
        self.inner
            .streaks()
            .into_iter()
            .map(|k| {
                object(&[
                    ("x0", k.x0.into()),
                    ("y0", k.y0.into()),
                    ("x1", k.x1.into()),
                    ("y1", k.y1.into()),
                    ("by", (k.by as u32).into()),
                    ("fade", k.fade.into()),
                ])
            })
            .collect()
    }

    /// How shaken the screen is, 0 to 1.
    pub fn trauma(&self) -> f64 {
        self.inner.trauma()
    }
}

fn object(fields: &[(&str, JsValue)]) -> Result<JsValue, JsError> {
    let o = Object::new();
    for (k, v) in fields {
        Reflect::set(&o, &JsValue::from_str(k), v)
            .map_err(|_| JsError::new("could not build an object"))?;
    }
    Ok(o.into())
}

fn num(e: &JsValue, key: &str) -> f64 {
    Reflect::get(e, &JsValue::from_str(key))
        .ok()
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0)
}

fn flag(e: &JsValue, key: &str) -> bool {
    Reflect::get(e, &JsValue::from_str(key))
        .ok()
        .is_some_and(|v| v.is_truthy())
}

fn index(e: &JsValue, key: &str) -> usize {
    let v = num(e, key);
    if v >= 0.0 { v as usize } else { usize::MAX }
}

/// One event object as a [`Cue`]; `None` for types the feel does not answer.
fn cue(e: &JsValue) -> Result<Option<Cue>, JsError> {
    let ty = Reflect::get(e, &JsValue::from_str("type"))
        .map_err(|_| JsError::new("an event must be an object"))?
        .as_string()
        .unwrap_or_default();
    let (x, y) = (num(e, "x"), num(e, "y"));
    Ok(Some(match ty.as_str() {
        "hit" => Cue::Hit {
            by: index(e, "by"),
            power: num(e, "power"),
            launch: flag(e, "launch"),
            crit: flag(e, "crit"),
            ranged: flag(e, "ranged"),
            share: num(e, "share"),
            damage: num(e, "damage"),
            x,
            y,
        },
        "tick" => Cue::Tick {
            by: index(e, "by"),
            damage: num(e, "damage"),
            x,
            y,
        },
        "heal" => Cue::Heal {
            by: index(e, "by"),
            amount: num(e, "amount"),
            x,
            y,
        },
        "ward" => Cue::Ward {
            by: index(e, "by"),
            amount: num(e, "amount"),
            x,
            y,
        },
        "dodge" => Cue::Dodge { x, y },
        "unyielding" => Cue::Unyielding,
        "block" => Cue::Block,
        "clash" => Cue::Clash,
        "land" => Cue::Land {
            x,
            strength: num(e, "strength"),
        },
        "bump" => Cue::Bump {
            strength: num(e, "strength"),
        },
        "ko" => Cue::Ko,
        "streak" => Cue::Streak {
            x0: num(e, "x0"),
            y0: num(e, "y0"),
            x1: num(e, "x1"),
            y1: num(e, "y1"),
            by: index(e, "by"),
        },
        "ultimate" => Cue::Ultimate,
        _ => return Ok(None),
    }))
}

// ---------- the body ----------

use pixfx_fighter::body::{self as b, Arena, Blow, BodySpec, Doing, Drive, Motion, Phase};
use pixfx_fighter::pose::Pose;

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
    hit_t / set_hit_t: "hitT" => hit_t,
    hit_power / set_hit_power: "hitPower" => hit_power,
    frozen / set_frozen: "frozen" => frozen,
    shocked / set_shocked: "shocked" => shocked,
    pose_t / set_pose_t: "poseT" => pose_t,
}

/// Every number of a [`BodySpec`] with its JS name (`juggleLimit`, a whole count, is read on its own).
macro_rules! spec_fields {
    ($($f:ident : $js:literal),* $(,)?) => {
        /// `spec` as a plain object with camelCase keys.
        fn spec_js(spec: &BodySpec) -> Result<JsValue, JsError> {
            object(&[
                $(($js, spec.$f.into()),)*
                ("juggleLimit", spec.juggle_limit.into()),
            ])
        }

        /// The default spec with the numbers `o` gives over it; a key that is there must be a number.
        fn spec_of(o: &JsValue) -> Result<BodySpec, JsError> {
            let mut spec = BodySpec::default();
            if o.is_undefined() || o.is_null() {
                return Ok(spec);
            }
            $(if let Some(v) = number_at(o, $js)? {
                spec.$f = v;
            })*
            if let Some(v) = number_at(o, "juggleLimit")? {
                spec.juggle_limit = v.max(0.0) as u32;
            }
            Ok(spec)
        }
    };
}

spec_fields! {
    body: "body",
    chest: "chest",
    half_width: "halfWidth",
    room: "room",
    walk: "walk",
    backwalk: "backwalk",
    chill: "chill",
    jump_speed: "jumpSpeed",
    gravity: "gravity",
    apex_speed: "apexSpeed",
    apex_gravity: "apexGravity",
    fly_rate: "flyRate",
    fly_max: "flyMax",
    launch_damage: "launchDamage",
    launch_speed: "launchSpeed",
    launch_per_damage: "launchPerDamage",
    launch_pop: "launchPop",
    launch_spin: "launchSpin",
    hitstun_base: "hitstunBase",
    hitstun_per: "hitstunPer",
    juggle_decay: "juggleDecay",
    knockdown_bounce: "knockdownBounce",
    knockdown_keep: "knockdownKeep",
    knockdown_lie: "knockdownLie",
    knockdown_rise: "knockdownRise",
    knockdown_quick: "knockdownQuick",
    knockdown_flop: "knockdownFlop",
    knockdown_roll: "knockdownRoll",
    knockdown_roll_distance: "knockdownRollDistance",
    knockdown_grace: "knockdownGrace",
    jump_squat: "jumpSquat",
    landing_soft: "landingSoft",
    landing_hard: "landingHard",
    duck: "duck",
    flash: "flash",
    bounce: "bounce",
    hard_landing: "hardLanding",
    knockback: "knockback",
    freeze_knockback: "freezeKnockback",
    launch_knockback: "launchKnockback",
    wall_keep: "wallKeep",
    soft_landing: "softLanding",
    slide: "slide",
    slide_down: "slideDown",
}

/// `o[key]` as a number: `None` when absent, an error naming the key when it is something else.
fn number_at(o: &JsValue, key: &str) -> Result<Option<f64>, JsError> {
    let v = Reflect::get(o, &JsValue::from_str(key))
        .map_err(|_| JsError::new("a body spec must be an object"))?;
    if v.is_undefined() {
        return Ok(None);
    }
    v.as_f64()
        .map(Some)
        .ok_or_else(|| JsError::new(&format!("body spec: {key} must be a number")))
}

#[wasm_bindgen]
impl Body {
    /// A body standing at `x`, facing right, built as `spec` says: omitted, the site's duel's body
    /// ([`Body::defaults`]); an object, those defaults with the numbers it gives over them (`{gravity: 400}`).
    #[wasm_bindgen(constructor)]
    pub fn new(x: f64, spec: JsValue) -> Result<Body, JsError> {
        Ok(Body {
            inner: b::Body::with_spec(x, spec_of(&spec)?),
        })
    }

    /// The default body's numbers, `{body, chest, halfWidth, walk, jumpSpeed, gravity, …}`: the rules read them
    /// instead of copying them, and pass back the ones they change to the constructor.
    pub fn defaults() -> Result<JsValue, JsError> {
        spec_js(&BodySpec::default())
    }

    /// This body's numbers, as [`Body::defaults`] gives them.
    #[wasm_bindgen(getter)]
    pub fn spec(&self) -> Result<JsValue, JsError> {
        spec_js(&self.inner.spec)
    }

    #[wasm_bindgen(getter)]
    pub fn pose(&self) -> String {
        pose_name(self.inner.pose).to_owned()
    }

    /// Off its feet: `{phase, t, juggle, light, spin, bounced, quick, dir}` (a copy), or null.
    #[wasm_bindgen(getter)]
    pub fn down(&self) -> Result<JsValue, JsError> {
        let Some(d) = self.inner.down else {
            return Ok(JsValue::NULL);
        };
        let phase = match d.phase {
            Phase::Air => "air",
            Phase::Down => "down",
            Phase::Roll => "roll",
            Phase::Rise => "rise",
        };
        object(&[
            ("phase", phase.into()),
            ("t", d.t.into()),
            ("juggle", d.juggle.into()),
            ("light", d.light.into()),
            ("spin", d.spin.into()),
            ("bounced", d.bounced.into()),
            ("quick", d.quick.into()),
            ("dir", d.dir.into()),
        ])
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
    pub fn step(
        &mut self,
        dt: f64,
        drive: &JsValue,
        arena: &JsValue,
        rand: &Function,
    ) -> Result<Array, JsError> {
        let drive = Drive {
            alive: flag(drive, "alive"),
            acting: flag(drive, "acting"),
            meditating: flag(drive, "meditating"),
            guarding: flag(drive, "guarding"),
            chilled: flag(drive, "chilled"),
            intent: num(drive, "intent"),
            motion: motion(drive)?,
            foe_x: num(drive, "foeX"),
        };
        let arena = arena_of(arena);
        let (events, failed) = with_roll(rand, |roll| self.inner.step(dt, &drive, &arena, roll));
        failed?;
        events_js(&events)
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
    pub fn separate(a: &mut Body, b: &mut Body, arena: &JsValue) {
        b::separate(&mut a.inner, &mut b.inner, &arena_of(arena));
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
    pub fn settle(
        &mut self,
        dt: f64,
        won: bool,
        alive: bool,
        foe_x: f64,
        arena: &JsValue,
        rand: &Function,
    ) -> Result<Array, JsError> {
        let arena = arena_of(arena);
        let (events, failed) = with_roll(rand, |roll| {
            self.inner.settle(dt, won, alive, foe_x, &arena, roll)
        });
        failed?;
        events_js(&events)
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
    pub fn update_pose(&mut self, doing: &str, melee: bool, alive: bool) -> Result<(), JsError> {
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

fn arena_of(v: &JsValue) -> Arena {
    Arena {
        left: num(v, "left"),
        right: num(v, "right"),
        top: num(v, "top"),
    }
}

fn motion(drive: &JsValue) -> Result<Motion, JsError> {
    let m = Reflect::get(drive, &JsValue::from_str("motion"))
        .map_err(|_| JsError::new("drive must be an object"))?;
    if m.as_string().is_some() || m.is_undefined() || m.is_null() {
        return Ok(Motion::Gravity);
    }
    let has = |k: &str| Reflect::has(&m, &JsValue::from_str(k)).unwrap_or(false);
    if has("fly") {
        Ok(Motion::Fly {
            hover: num(&m, "fly"),
        })
    } else if has("dive") {
        Ok(Motion::Dive {
            speed: num(&m, "dive"),
            floor: num(&m, "floor"),
        })
    } else {
        Err(JsError::new(
            "motion: expected \"gravity\", {fly} or {dive, floor}",
        ))
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

fn events_js(events: &[b::Event]) -> Result<Array, JsError> {
    events
        .iter()
        .map(|e| match *e {
            b::Event::Land { x, strength } => object(&[
                ("type", "land".into()),
                ("x", x.into()),
                ("strength", strength.into()),
            ]),
            b::Event::Bump { x, y, strength } => object(&[
                ("type", "bump".into()),
                ("x", x.into()),
                ("y", y.into()),
                ("strength", strength.into()),
            ]),
        })
        .collect()
}

pub(crate) fn pose_name(p: Pose) -> &'static str {
    match p {
        Pose::Idle => "idle",
        Pose::Walk => "walk",
        Pose::Jump => "jump",
        Pose::Fly => "fly",
        Pose::Land => "land",
        Pose::Crouch => "crouch",
        Pose::Duck => "duck",
        Pose::Draw => "draw",
        Pose::Slash => "slash",
        Pose::Channel => "channel",
        Pose::Cast => "cast",
        Pose::Guard => "guard",
        Pose::Meditate => "meditate",
        Pose::Hurt => "hurt",
        Pose::Frozen => "frozen",
        Pose::Shocked => "shocked",
        Pose::Launched => "launched",
        Pose::Tumble => "tumble",
        Pose::Fall => "fall",
        Pose::Down => "down",
        Pose::Roll => "roll",
        Pose::Sit => "sit",
        Pose::Kneel => "kneel",
        Pose::Win => "win",
    }
}

// ---------- drawing fighters ----------

use crate::Surface;
use crate::stage::{Pass, Rect};
use pixfx_core::color::to_hex;
use pixfx_core::{Frame, Layer, Rgba};
use pixfx_fighter::look::{Hairdo, Look};
use pixfx_fighter::paint::{self, Appearance, Windup};
use pixfx_fighter::pose::Gather;
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
    /// `{x, air, pose, poseT, face, vx, out, flash, frozen, shocked, hitT, hitPower, dodged, jolt: [dx, dy],
    /// act: null | {how: "summon"|"palm"|"lunge"|"prayer"|"cross", dive, progress: number|null, long}}`. `marks`:
    /// null, or `{shield, barrier, rang, burning, defDown, silence, chilled, fury, poseT, dome: "banded"|"faceted",
    /// burn, shock, ice, t, seed}`, the last five drawing fire, lightning and frost over the body at time `t`,
    /// following the shape of its pose.
    pub fn fighter(
        &mut self,
        look: &JsValue,
        a: &JsValue,
        ground: f64,
        marks: &JsValue,
    ) -> Result<(), JsError> {
        let look = look_of(look)?;
        let a = appearance(a)?;
        paint::draw(&mut self.frame, &look, &a, ground);
        if marks.is_object() {
            // the marks in the order the duel has always drawn them
            let mut m = Vec::new();
            if flag(marks, "shield") {
                m.push(Mark::Shield);
            }
            if flag(marks, "barrier") {
                let style = if text(marks, "dome") == "faceted" {
                    DomeStyle::Faceted
                } else {
                    DomeStyle::Banded
                };
                m.push(Mark::Barrier {
                    style,
                    glow: look.glow,
                    rang: flag(marks, "rang"),
                });
            }
            for (key, mark) in [
                ("burning", Mark::Burning),
                ("defDown", Mark::DefDown),
                ("silence", Mark::Silence),
                ("chilled", Mark::Chilled),
            ] {
                if flag(marks, key) {
                    m.push(mark);
                }
            }
            m.push(Mark::Fury(num(marks, "fury").max(0.0) as u32));
            let figure = Figure {
                feet_x: a.x,
                feet_y: ground - a.air,
                face: a.face,
                silhouette: Silhouette::of(a.pose),
            };
            let (t, seed) = (num(marks, "t"), num(marks, "seed").max(0.0) as u32);
            if flag(marks, "burn") {
                status::draw_burning(&mut self.frame, t, seed, &figure, 1.0);
            }
            if flag(marks, "ice") {
                status::draw_frozen(&mut self.frame, t, seed, &figure, 1.0);
            }
            if flag(marks, "shock") {
                status::draw_shocked(&mut self.frame, t, seed, &figure, 1.0);
            }
            status::draw_marks(
                &mut self.frame,
                &m,
                num(marks, "poseT"),
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
        look: &JsValue,
        a: &JsValue,
        x: i32,
        y: i32,
        size: i32,
        alpha: f64,
    ) -> Result<(), JsError> {
        let look = look_of(look)?;
        let idle = appearance(a)?;
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
                let a = pixfx_core::math::js_round(f64::from(c.3) * alpha).clamp(0.0, 255.0) as u8;
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

/// A look's colours the host draws its own parts in: `{glow, outline, ribbon: [light, dark]}` as `#rrggbb`.
/// `look` is a preset's name (`"thunder"`, `"frost"`) or a whole look: `{robe: [3], inner: [2], trim: [2],
/// hair: [2], skin: [2], boot, blade: [3], jade, ribbon: [2], glow, outline, hairdo: "topknot" | "flowing"}`, each
/// colour `"#rrggbb"`, lists light to dark.
#[wasm_bindgen(js_name = lookColors)]
pub fn look_colors(look: &JsValue) -> Result<JsValue, JsError> {
    let l = look_of(look)?;
    let ribbon: Array = l.ribbon.iter().map(|&c| JsValue::from(to_hex(c))).collect();
    object(&[
        ("glow", to_hex(l.glow).into()),
        ("outline", to_hex(l.outline).into()),
        ("ribbon", ribbon.into()),
    ])
}

/// A look: a preset by name, or read from an object (see [`look_colors`]).
fn look_of(v: &JsValue) -> Result<Look, JsError> {
    if let Some(name) = v.as_string() {
        return Look::preset(&name).ok_or_else(|| {
            JsError::new(&format!(
                "unknown look {name:?}: expected \"thunder\", \"frost\" or a look object"
            ))
        });
    }
    if !v.is_object() {
        return Err(JsError::new("a look must be a preset's name or an object"));
    }
    let hairdo = match text(v, "hairdo").as_str() {
        "topknot" => Hairdo::Topknot,
        "flowing" => Hairdo::Flowing,
        other => {
            return Err(JsError::new(&format!(
                "look.hairdo: expected \"topknot\" or \"flowing\", got {other:?}"
            )));
        }
    };
    Ok(Look {
        robe: shades(v, "robe")?,
        inner: shades(v, "inner")?,
        trim: shades(v, "trim")?,
        hair: shades(v, "hair")?,
        skin: shades(v, "skin")?,
        boot: colour(v, "boot")?,
        blade: shades(v, "blade")?,
        jade: colour(v, "jade")?,
        ribbon: shades(v, "ribbon")?,
        glow: colour(v, "glow")?,
        outline: colour(v, "outline")?,
        hairdo,
    })
}

/// `o[key]` as a `"#rrggbb"` colour.
fn colour(o: &JsValue, key: &str) -> Result<Rgba, JsError> {
    let s = Reflect::get(o, &JsValue::from_str(key))
        .ok()
        .and_then(|v| v.as_string())
        .ok_or_else(|| JsError::new(&format!("look.{key}: expected a \"#rrggbb\" colour")))?;
    pixfx_core::color::hex(&s).map_err(|e| JsError::new(&format!("look.{key}: {e}")))
}

/// `o[key]` as a list of exactly `N` colours, light to dark.
fn shades<const N: usize>(o: &JsValue, key: &str) -> Result<[Rgba; N], JsError> {
    let list = Reflect::get(o, &JsValue::from_str(key)).unwrap_or(JsValue::UNDEFINED);
    let wrong = || {
        JsError::new(&format!(
            "look.{key}: expected a list of {N} \"#rrggbb\" colours"
        ))
    };
    if !Array::is_array(&list) || Array::from(&list).length() as usize != N {
        return Err(wrong());
    }
    let list = Array::from(&list);
    let mut out = [Rgba::NONE; N];
    for (i, slot) in out.iter_mut().enumerate() {
        let s = list.get(i as u32).as_string().ok_or_else(wrong)?;
        *slot = pixfx_core::color::hex(&s)
            .map_err(|e| JsError::new(&format!("look.{key}[{i}]: {e}")))?;
    }
    Ok(out)
}

fn text(e: &JsValue, key: &str) -> String {
    Reflect::get(e, &JsValue::from_str(key))
        .ok()
        .and_then(|v| v.as_string())
        .unwrap_or_default()
}

fn pose_of_name(name: &str) -> Result<Pose, JsError> {
    const ALL: [Pose; 24] = [
        Pose::Idle,
        Pose::Walk,
        Pose::Jump,
        Pose::Fly,
        Pose::Land,
        Pose::Crouch,
        Pose::Duck,
        Pose::Draw,
        Pose::Slash,
        Pose::Channel,
        Pose::Cast,
        Pose::Guard,
        Pose::Meditate,
        Pose::Hurt,
        Pose::Frozen,
        Pose::Shocked,
        Pose::Launched,
        Pose::Tumble,
        Pose::Fall,
        Pose::Down,
        Pose::Roll,
        Pose::Sit,
        Pose::Kneel,
        Pose::Win,
    ];
    ALL.into_iter()
        .find(|&p| pose_name(p) == name)
        .ok_or_else(|| JsError::new(&format!("unknown pose {name:?}")))
}

fn appearance(a: &JsValue) -> Result<Appearance, JsError> {
    let act = Reflect::get(a, &JsValue::from_str("act"))
        .map_err(|_| JsError::new("appearance must be an object"))?;
    let act = if act.is_object() {
        let how = match text(&act, "how").as_str() {
            "summon" => Gather::Summon,
            "lunge" => Gather::Lunge,
            "prayer" => Gather::Prayer,
            "cross" => Gather::Cross,
            _ => Gather::Palm,
        };
        let progress = Reflect::get(&act, &JsValue::from_str("progress"))
            .ok()
            .and_then(|v| v.as_f64());
        Some(paint::Act {
            how,
            dive: flag(&act, "dive"),
            windup: progress.map(|progress| Windup {
                progress,
                long: flag(&act, "long"),
            }),
        })
    } else {
        None
    };
    let jolt = Reflect::get(a, &JsValue::from_str("jolt")).unwrap_or(JsValue::UNDEFINED);
    let at = |i: u32| {
        Reflect::get_u32(&jolt, i)
            .ok()
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as i32
    };
    let jolt = if jolt.is_object() {
        [at(0), at(1)]
    } else {
        [0, 0]
    };
    Ok(Appearance {
        pose: pose_of_name(&text(a, "pose"))?,
        pose_t: num(a, "poseT"),
        face: num(a, "face"),
        x: num(a, "x"),
        air: num(a, "air"),
        vx: num(a, "vx"),
        out: flag(a, "out"),
        flash: flag(a, "flash"),
        frozen: flag(a, "frozen"),
        shocked: flag(a, "shocked"),
        hit_t: num(a, "hitT"),
        hit_power: num(a, "hitPower"),
        dodged: num(a, "dodged"),
        act,
        jolt,
    })
}
