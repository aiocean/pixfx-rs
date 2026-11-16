//! `pixfx-duel` for JS: the landing duel's rules ([`Duel`]) and the tables the page reads. It translates only: every
//! rule is the crate's.

use crate::js::{parse, to_js};
use pixfx_duel::data::{Blow, DECOR, RAGE, Rarity, STALE, School, Stats, TRAITS, spells, stats};
use pixfx_duel::duel::{self as d, DuelConfig};
use pixfx_duel::first_mind::FirstMind;
use pixfx_duel::seat::{Brain, MakeSeat, PolicySeat, shipped};
use pixfx_duel::status::damage;
use pixfx_mind::policy::Net;
use serde::ser::{SerializeMap, Serializer};
use serde::{Deserialize, Serialize};
use std::rc::Rc;
use wasm_bindgen::prelude::*;

/// Where a duel is fought, as JS hands it: the arena from `left` to `right` standing on `ground`, how high a fighter
/// may fly (`sky`), and the effects in the registry.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Config {
    seed: u32,
    left: f64,
    right: f64,
    ground: f64,
    sky: f64,
    effects: Vec<String>,
}

/// Who sits in a corner: the trained policy (in the style the fight drew), the utility brain, or the duel's first
/// mind.
#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
enum SeatKind {
    Policy,
    Brain,
    First,
}

impl SeatKind {
    fn make(self, net: &Rc<Net>) -> MakeSeat {
        match self {
            SeatKind::Policy => PolicySeat::make(Rc::clone(net)),
            SeatKind::Brain => Brain::make(),
            SeatKind::First => FirstMind::make(),
        }
    }
}

/// The landing duel: two fighters and the minds that decide for them, stepped in whole ticks.
#[wasm_bindgen]
pub struct Duel {
    inner: d::Duel,
}

#[wasm_bindgen]
impl Duel {
    /// A duel fought where `config` says (`{seed, left, right, ground, sky, effects}`), its corners seated by `seats`
    /// (`["policy" | "brain" | "first", ...]`, the trained policy when left out), its first round begun.
    #[wasm_bindgen(constructor)]
    pub fn new(
        #[wasm_bindgen(
            unchecked_param_type = "{ seed: number, left: number, right: number, ground: number, sky: number, effects: string[] }"
        )]
        config: &JsValue,
        #[wasm_bindgen(
            unchecked_param_type = "[\"policy\" | \"brain\" | \"first\", \"policy\" | \"brain\" | \"first\"] | undefined"
        )]
        seats: &JsValue,
    ) -> Result<Duel, JsError> {
        let c: Config = parse("config", config)?;
        let net = shipped().map_err(|e| JsError::new(&e.to_string()))?;
        let [a, b] = if seats.is_undefined() || seats.is_null() {
            [SeatKind::Policy, SeatKind::Policy]
        } else {
            parse("seats", seats)?
        };
        let config = DuelConfig {
            seed: c.seed,
            left: c.left,
            right: c.right,
            ground: c.ground,
            sky: c.sky,
            effects: c.effects,
        };
        Ok(Duel {
            inner: d::Duel::new(config, [a.make(&net), b.make(&net)]),
        })
    }

    /// Advance by `dt` seconds: the effect commands and events of this step.
    pub fn step(&mut self, dt: f64) -> Result<JsValue, JsError> {
        to_js(&self.inner.step(dt))
    }

    /// The duel as the page reads it: `{time, round, wins, winner, left, right, ground, fighters}`.
    pub fn view(&self) -> Result<JsValue, JsError> {
        to_js(&pixfx_duel::view::view(&self.inner.state))
    }

    /// Fighter `me`'s seat (0 or 1) thinks nothing for `seconds`.
    pub fn hold(&mut self, me: usize, seconds: f64) -> Result<(), JsError> {
        if me >= 2 {
            return Err(JsError::new(&format!(
                "hold: no corner {me}, a duel has corners 0 and 1"
            )));
        }
        self.inner.hold(me, seconds);
        Ok(())
    }
}

/// Every spell by name, in the duel's order, as `duel.js`'s `SPELLS`.
#[wasm_bindgen(js_name = duelSpells)]
pub fn duel_spells() -> Result<JsValue, JsError> {
    struct Table;
    impl Serialize for Table {
        fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
            let mut map = s.serialize_map(Some(spells().len()))?;
            for spell in spells() {
                map.serialize_entry(spell.name, spell)?;
            }
            map.end()
        }
    }
    to_js(&Table)
}

/// What each trait does, as `duel.js`'s `TRAITS`.
#[wasm_bindgen(js_name = duelTraits)]
pub fn duel_traits() -> Result<JsValue, JsError> {
    to_js(&serde_json::json!({
        "swift": { "cooldown": TRAITS.swift_cooldown },
        "heavy": { "power": TRAITS.heavy_power },
        "piercing": {},
        "lingering": { "stacks": TRAITS.lingering_stacks },
        "cheap": { "cost": TRAITS.cheap_cost },
    }))
}

/// The duel's constants the page reads: `{RAGE, STALE, DECOR: {lamp, night}, TIERS}`.
#[wasm_bindgen(js_name = duelConstants)]
pub fn duel_constants() -> Result<JsValue, JsError> {
    let [lamp, night] = DECOR;
    to_js(
        &serde_json::json!({ "RAGE": RAGE, "STALE": STALE, "DECOR": { "lamp": lamp, "night": night }, "TIERS": Rarity::ALL }),
    )
}

/// Each school's fighter stats, as `duel.js`'s `STATS`.
#[wasm_bindgen(js_name = duelStats)]
pub fn duel_stats() -> Result<JsValue, JsError> {
    to_js(&serde_json::json!({ "thunder": stats(School::Thunder), "frost": stats(School::Frost) }))
}

/// A striker as `damageOf` reads it.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Striker {
    stats: Stats,
    #[serde(default)]
    fury: f64,
}

/// A target as `damageOf` reads it.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Target {
    stats: Stats,
    #[serde(default)]
    def_down: f64,
}

/// The damage `by` (`{stats, fury}`) deals `f` (`{stats, defDown}`) with a blow of `power` and `type`
/// (`"physical" | "magic"`), `crit` or not, at spread `roll` (0 to 1).
#[wasm_bindgen(js_name = damageOf)]
pub fn damage_of(
    #[wasm_bindgen(unchecked_param_type = "{ stats: Stats, fury?: number }")] by: &JsValue,
    #[wasm_bindgen(unchecked_param_type = "{ stats: Stats, defDown?: number }")] f: &JsValue,
    power: f64,
    #[wasm_bindgen(unchecked_param_type = "\"physical\" | \"magic\"")] blow: &JsValue,
    crit: bool,
    roll: f64,
) -> Result<f64, JsError> {
    let by: Striker = parse("by", by)?;
    let f: Target = parse("f", f)?;
    let blow: Blow = parse("type", blow)?;
    Ok(damage(
        &by.stats, by.fury, &f.stats, f.def_down, power, blow, crit, roll,
    ))
}

/// Where an orb bobs `t` seconds after it was cast, from its line: `[dx, dy]`.
#[wasm_bindgen(js_name = orbOffset)]
pub fn orb_offset(t: f64) -> js_sys::Array {
    let (x, y) = pixfx_duel::data::orb_offset(t);
    crate::js::pair(x, y)
}
