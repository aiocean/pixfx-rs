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
