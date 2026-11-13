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
        Feel { inner: feel::Feel::new() }
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
        Reflect::set(&o, &JsValue::from_str(k), v).map_err(|_| JsError::new("could not build an object"))?;
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
        "tick" => Cue::Tick { by: index(e, "by"), damage: num(e, "damage"), x, y },
        "heal" => Cue::Heal { by: index(e, "by"), amount: num(e, "amount"), x, y },
        "ward" => Cue::Ward { by: index(e, "by"), amount: num(e, "amount"), x, y },
        "dodge" => Cue::Dodge { x, y },
        "unyielding" => Cue::Unyielding,
        "block" => Cue::Block,
        "clash" => Cue::Clash,
        "land" => Cue::Land { x, strength: num(e, "strength") },
        "bump" => Cue::Bump { strength: num(e, "strength") },
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

use pixfx_fighter::body::{self as b, Arena, Blow, Doing, Drive, Motion, Phase};
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

#[wasm_bindgen]
impl Body {
    /// A body standing at `x`, facing right.
    #[wasm_bindgen(constructor)]
    pub fn new(x: f64) -> Body {
        Body { inner: b::Body::new(x) }
    }

    #[wasm_bindgen(getter)]
    pub fn pose(&self) -> String {
        pose_name(self.inner.pose).to_owned()
    }

    /// Off its feet: `{phase, t, juggle, light, spin, bounced, quick, dir}` (a copy), or null.
    #[wasm_bindgen(getter)]
    pub fn down(&self) -> Result<JsValue, JsError> {
        let Some(d) = self.inner.down else { return Ok(JsValue::NULL) };
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
    pub fn step(&mut self, dt: f64, drive: &JsValue, arena: &JsValue, rand: &Function) -> Result<Array, JsError> {
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
        self.inner.hit(Blow { power, dir, freeze, flying });
    }

    /// Whether a blow of `power` throws (a freezing one never does).
    pub fn heavy(power: f64, freeze: bool) -> bool {
        Blow { power, dir: 1.0, freeze, flying: false }.heavy()
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
        let (events, failed) =
            with_roll(rand, |roll| self.inner.settle(dt, won, alive, foe_x, &arena, roll));
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

/// Two bodies cannot stand in one place: part them (the caller skips this when either is knocked out).
#[wasm_bindgen]
pub fn separate(a: &mut Body, b: &mut Body, arena: &JsValue) {
    b::separate(&mut a.inner, &mut b.inner, &arena_of(arena));
}

fn arena_of(v: &JsValue) -> Arena {
    Arena { left: num(v, "left"), right: num(v, "right"), top: num(v, "top") }
}

fn motion(drive: &JsValue) -> Result<Motion, JsError> {
    let m = Reflect::get(drive, &JsValue::from_str("motion"))
        .map_err(|_| JsError::new("drive must be an object"))?;
    if m.as_string().is_some() || m.is_undefined() || m.is_null() {
        return Ok(Motion::Gravity);
    }
    let has = |k: &str| Reflect::has(&m, &JsValue::from_str(k)).unwrap_or(false);
    if has("fly") {
        Ok(Motion::Fly { hover: num(&m, "fly") })
    } else if has("dive") {
        Ok(Motion::Dive { speed: num(&m, "dive"), floor: num(&m, "floor") })
    } else {
        Err(JsError::new("motion: expected \"gravity\", {fly} or {dive, floor}"))
    }
}

/// Run `f` with a roll that calls JS `rand()`; the error says if `rand` failed or gave a non-number.
fn with_roll<T>(rand: &Function, f: impl FnOnce(&mut dyn FnMut() -> f64) -> T) -> (T, Result<(), JsError>) {
    let mut bad = false;
    let mut roll = || match rand.call0(&JsValue::NULL).map(|v| v.as_f64()) {
        Ok(Some(v)) => v,
        _ => {
            bad = true;
            0.0
        }
    };
    let out = f(&mut roll);
    let status = if bad { Err(JsError::new("rand() must return a number")) } else { Ok(()) };
    (out, status)
}

fn events_js(events: &[b::Event]) -> Result<Array, JsError> {
    events
        .iter()
        .map(|e| match *e {
            b::Event::Land { x, strength } => {
                object(&[("type", "land".into()), ("x", x.into()), ("strength", strength.into())])
            }
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
