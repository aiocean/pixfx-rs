//! The duel's tables, dice and maths, pinned against `tests/fixtures/js_numbers.json`.

use pixfx_duel::data::{
    self, Blow, Kind, LUCKY, PAYOFF, RARITY, Rarity, School, Status, TRAITS, Trait, block_chance, fits, orb_offset,
    shock_interval, slot_power,
};
use pixfx_duel::rng::Rng;
use serde_json::{Map, Value, json};

fn fixture() -> Value {
    serde_json::from_str(include_str!("fixtures/js_numbers.json")).unwrap_or(Value::Null)
}

/// Every number as an f64, so 24 and 24.0 compare equal.
fn norm(v: &Value) -> Value {
    match v {
        Value::Number(n) => json!(n.as_f64().unwrap_or(f64::NAN)),
        Value::Array(a) => Value::Array(a.iter().map(norm).collect()),
        Value::Object(o) => Value::Object(o.iter().map(|(k, v)| (k.clone(), norm(v))).collect()),
        other => other.clone(),
    }
}

fn num(v: &Value) -> f64 {
    v.as_f64().unwrap_or(f64::NAN)
}

#[test]
fn rng_matches_js_for_edge_seeds() -> Result<(), String> {
    let f = fixture();
    let seeds = f["rng"].as_object().ok_or_else(|| "fixture has no rng".to_string())?;
    assert_eq!(seeds.len(), 6);
    for (seed, draws) in seeds {
        let mut rng = Rng::new(seed.parse::<u64>().unwrap_or(0) as u32);
        for (i, d) in draws.as_array().into_iter().flatten().enumerate() {
            assert_eq!(rng.next(), num(d), "seed {seed} draw {i}");
        }
    }
    Ok(())
}

#[test]
fn orb_offset_matches_js() {
    for row in fixture()["orbOffset"].as_array().into_iter().flatten() {
        let (x, y) = orb_offset(num(&row[0]));
        assert!((x - num(&row[1])).abs() < 1e-12 && (y - num(&row[2])).abs() < 1e-12, "t {}", row[0]);
    }
}

#[test]
fn block_chance_matches_js() {
    for row in fixture()["blockChance"].as_array().into_iter().flatten() {
        assert_eq!(block_chance(num(&row[0])), num(&row[1]), "power {}", row[0]);
    }
}

#[test]
fn shock_interval_matches_js() {
    for row in fixture()["shockInterval"].as_array().into_iter().flatten() {
        assert_eq!(shock_interval(num(&row[0])), num(&row[1]), "stacks {}", row[0]);
    }
}

fn rarity(name: &str) -> Rarity {
    Rarity::ALL.into_iter().find(|r| r.name() == name).unwrap_or(Rarity::Common)
}

fn trait_of(name: &str) -> Option<Trait> {
    Trait::ALL.into_iter().find(|t| t.name() == name)
}

#[test]
fn slot_power_matches_js() {
    let rows = fixture()["slotPower"].as_array().cloned().unwrap_or_default();
    assert_eq!(rows.len(), 16);
    for row in rows {
        let traits: Vec<Trait> = row[1].as_array().into_iter().flatten().filter_map(|t| t.as_str().and_then(trait_of)).collect();
        let evolved = row[2].as_bool().unwrap_or(false);
        assert_eq!(slot_power(rarity(row[0].as_str().unwrap_or("")), &traits, evolved), num(&row[3]), "{row}");
    }
}

#[test]
fn fits_matches_js() -> Result<(), String> {
    for row in fixture()["fits"].as_array().into_iter().flatten() {
        let t = row[0].as_str().and_then(trait_of).ok_or_else(|| format!("unknown trait {}", row[0]))?;
        assert_eq!(fits(t, row[1].as_str().unwrap_or("")), row[2].as_bool().unwrap_or(false), "{row}");
    }
    Ok(())
}

fn kind_name(k: Kind) -> &'static str {
    k.name()
}

/// A spell in the fixture's shape: fields left out when they are absent or false.
fn spell_json(s: &data::Spell) -> Value {
    let mut o = Map::new();
    let mut put = |k: &str, v: Value| {
        o.insert(k.to_string(), v);
    };
    put("kind", json!(kind_name(s.kind)));
    if let Some(b) = s.blow {
        put("type", json!(match b {
            Blow::Physical => "physical",
            Blow::Magic => "magic",
        }));
    }
    for (k, v) in [("fx", s.fx), ("hitFx", s.hit_fx), ("endFx", s.end_fx), ("slamFx", s.slam_fx)] {
        if let Some(v) = v {
            put(k, json!(v));
        }
    }
    for (k, v) in [("cost", s.cost), ("windup", s.windup), ("recover", s.recover), ("cooldown", s.cooldown)] {
        put(k, json!(v));
    }
    for (k, v) in [
        ("range", s.range), ("distance", s.distance), ("damage", s.damage), ("sigil", s.sigil), ("impact", s.impact),
        ("radius", s.radius), ("notice", s.notice), ("defDown", s.def_down), ("life", s.life), ("height", s.height),
        ("pull", s.pull), ("reach", s.reach), ("under", s.under), ("tick", s.tick), ("freeze", s.freeze),
        ("speed", s.speed), ("silence", s.silence), ("pullUntil", s.pull_until), ("stealRage", s.steal_rage),
        ("strikes", s.strikes), ("every", s.every), ("aimAhead", s.aim_ahead), ("amount", s.amount), ("time", s.time),
        ("spray", s.spray),
    ] {
        if let Some(v) = v {
            put(k, json!(v));
        }
    }
    for (k, v) in [
        ("shock", s.shock), ("chill", s.chill), ("dive", s.dive), ("burn", s.burn), ("grounded", s.grounded),
        ("ultimate", s.ultimate), ("seals", s.seals), ("reflect", s.reflect), ("feet", s.feet),
    ] {
        if v {
            put(k, json!(true));
        }
    }
    if let Some(c) = s.counter {
        put("counter", json!({ "damage": c.damage, "stun": c.stun }));
    }
    Value::Object(o)
}

#[test]
fn every_spell_is_ported_field_for_field() -> Result<(), String> {
    let f = fixture();
    let js = f["spells"].as_object().ok_or_else(|| "fixture has no spells".to_string())?;
    let names: Vec<&str> = data::spells().iter().map(|s| s.name).collect();
    assert_eq!(names, js.keys().map(String::as_str).collect::<Vec<_>>(), "names in the fixture's order");
    for (name, want) in js {
        let s = data::spell(name).ok_or_else(|| format!("no spell {name}"))?;
        let got = spell_json(s);
        // keys in any order: compare as sorted maps
        let sort = |v: &Value| v.as_object().map(|o| o.iter().map(|(k, v)| (k.clone(), v.clone())).collect::<std::collections::BTreeMap<_, _>>());
        assert_eq!(sort(&norm(&got)), sort(&norm(want)), "spell {name}");
    }
    Ok(())
}

#[test]
fn every_school_and_its_stats_are_ported() -> Result<(), String> {
    let f = fixture();
    for (key, school) in [("thunder", School::Thunder), ("frost", School::Frost)] {
        let d = data::school(school);
        let js = &f["schools"][key];
        assert_eq!(json!(d.name), js["name"]);
        assert_eq!(json!(d.basics), js["basics"], "{key} basics");
        assert_eq!(json!(d.pool), js["pool"], "{key} pool");
        assert_eq!(json!(d.ultimate), js["ultimate"]);
        assert_eq!(json!(kind_name(d.signature)), js["signature"]);
        assert_eq!(norm(&json!(d.postures)), norm(&js["postures"]));
        for (k, v) in [("range", d.range), ("dry", d.dry), ("aggression", d.aggression), ("tempo", d.tempo), ("sky", d.sky)] {
            assert_eq!(v, num(&js[k]), "{key}.{k}");
        }
        assert_eq!(json!(d.bolt), js["bolt"]);
        assert_eq!(json!(d.circle), js["circle"]);
        let tints = js["tint"].as_object().ok_or_else(|| format!("{key} has no tint"))?;
        assert_eq!(d.tint.len(), tints.len(), "{key} tints");
        for (fx, want) in tints {
            let (_, t) = d.tint.iter().find(|(name, _)| name == fx).ok_or_else(|| format!("{key} tint {fx}"))?;
            let mut o = Map::new();
            for (k, v) in [("palette", t.palette), ("accent", t.accent), ("stars", t.stars), ("fire", t.fire)] {
                if let Some(v) = v {
                    o.insert(k.into(), json!(v));
                }
            }
            if let Some(g) = t.glow {
                o.insert("glow".into(), json!(g));
            }
            for (k, v) in [("width", t.width), ("height", t.height)] {
                if let Some(v) = v {
                    o.insert(k.into(), json!(v));
                }
            }
            assert_eq!(norm(&Value::Object(o)), norm(want), "{key} tint {fx}");
        }
        let s = data::stats(school);
        let got = json!({
            "hp": s.hp, "atk": s.atk, "pdef": s.pdef, "mdef": s.mdef, "crit": s.crit, "critDamage": s.crit_damage,
            "dodge": s.dodge, "fury": s.fury, "unyielding": s.unyielding, "attuned": s.attuned,
        });
        assert_eq!(norm(&got), norm(&f["stats"][key]), "{key} stats");
    }
    Ok(())
}

#[test]
fn rarity_traits_luck_and_payoffs_are_ported() -> Result<(), String> {
    let f = fixture();
    for r in Rarity::ALL {
        let d = RARITY[r as usize];
        let mut o = json!({ "odds": d.odds, "power": d.power, "traits": d.traits });
        if let Some(c) = d.chance {
            o["chance"] = json!(c);
        }
        assert_eq!(norm(&o), norm(&f["rarity"][r.name()]), "{}", r.name());
    }
    assert_eq!(json!(Rarity::ALL.map(|r| r.name())), f["constants"]["TIERS"]);
    let traits = json!({
        "swift": { "cooldown": TRAITS.swift_cooldown }, "heavy": { "power": TRAITS.heavy_power }, "piercing": {},
        "lingering": { "stacks": TRAITS.lingering_stacks }, "cheap": { "cost": TRAITS.cheap_cost },
    });
    assert_eq!(norm(&traits), norm(&f["traits"]));
    assert_eq!(json!(Trait::ALL.map(|t| t.name())), json!(["swift", "heavy", "piercing", "lingering", "cheap"]));
    assert_eq!(norm(&json!({ "chance": LUCKY.chance, "kinds": ["double", "free", "again"] })), norm(&f["lucky"]));
    let js = f["payoff"].as_object().ok_or_else(|| "no payoff".to_string())?;
    assert_eq!(PAYOFF.len(), js.len());
    for (name, status, per) in PAYOFF {
        let s = match status {
            Status::Shock => "shock",
            Status::Chill => "chill",
        };
        assert_eq!(norm(&json!({ "status": s, "per": per })), norm(&js[*name]), "payoff {name}");
    }
    assert_eq!(data::RAGE, num(&f["constants"]["RAGE"]));
    assert_eq!(data::STALE, num(&f["constants"]["STALE"]));
    assert_eq!(data::POOL_SIZE as f64, num(&f["constants"]["POOL_SIZE"]));
    assert_eq!(json!({ "lamp": data::DECOR[0], "night": data::DECOR[1] }), f["constants"]["DECOR"]);
    Ok(())
}
