//! The rotating skill bar: each fight's pool, draws with tiers and traits, spending a slot and evolving it, what a
//! fighter can cast now, and what the HUD shows of the bar and of a fighter's buffs.

use crate::data::{
    BAR, Kind, PAY_DRAW, POOL_SIZE, RAGE, RARITY, Rarity, STALE, School, Status, TRAITS, Trait,
    evolves, fits, payoff, school, slot_power, spell,
};
use crate::fighter::{Fighter, Slot, State};
use crate::out::Out;
use pixfx_draw::math::js_min;

/// `POOL_SIZE` skills of `school` a fighter plays this fight, by the duel's dice: always one of the school's
/// signature kind and one that pays off its status, where it has them, then the rest at random.
pub fn fight_pool(
    state: &mut State,
    s: School,
    usable: &dyn Fn(&str) -> bool,
) -> Vec<&'static str> {
    let d = school(s);
    let mut left: Vec<&'static str> = d.pool.iter().copied().filter(|name| usable(name)).collect();
    for i in (1..left.len()).rev() {
        let j = pick(state, i + 1);
        left.swap(i, j);
    }
    let signature = left
        .iter()
        .copied()
        .find(|name| spell(name).is_some_and(|sp| sp.kind == d.signature));
    let pays = left.iter().copied().find(|name| payoff(name).is_some());
    let mut kept: Vec<&'static str> = Vec::new();
    for name in [signature, pays].into_iter().flatten() {
        if !kept.contains(&name) {
            kept.push(name);
        }
    }
    for name in left {
        if kept.len() < POOL_SIZE && !kept.contains(&name) {
            kept.push(name);
        }
    }
    kept
}

/// A die of `n` faces, 0-based: `Math.floor(rng() * n)`.
fn pick(state: &mut State, n: usize) -> usize {
    let i = libm::floor(state.rng.next() * n as f64) as usize;
    i.min(n.saturating_sub(1))
}

/// The bar slot of fighter `f` holding `name`.
pub fn slot_of<'a>(f: &'a Fighter, name: &str) -> Option<&'a Slot> {
    f.bar.iter().find(|slot| slot.spell() == Some(name))
}

/// Whether `slot` holds a skill drawn with trait `t`.
pub fn has_trait(slot: Option<&Slot>, t: Trait) -> bool {
    matches!(slot, Some(Slot::Held { traits, .. }) if traits.contains(&t))
}

/// What casting `name` costs `f` in qi: a cheap skill half.
pub fn cost_of(f: &Fighter, name: &str) -> f64 {
    let cost = spell(name).map_or(0.0, |s| s.cost);
    cost * if has_trait(slot_of(f, name), Trait::Cheap) {
        TRAITS.cheap_cost
    } else {
        1.0
    }
}

/// Can `f` cast `name` now: in its kit, on its bar if it is a bar skill, off cooldown, paid for.
pub fn ready(f: &Fighter, name: &str) -> bool {
    let Some(s) = spell(name) else { return false };
    f.spells.contains(&name)
        && (!f.pool.contains(&name) || slot_of(f, name).is_some())
        && f.cool(name) <= 0.0
        && f.qi >= cost_of(f, name)
        && (!s.ultimate || f.rage >= RAGE)
        && !(name == "blink" && f.sealed > 0.0)
        // silenced, only the blade answers
        && !(f.silence > 0.0 && s.kind != Kind::Melee)
}

/// Put `slot` at `i` of fighter `f`'s bar (one past the end grows it, as a JS array does).
fn put(f: &mut Fighter, i: usize, slot: Slot) {
    match f.bar.get_mut(i) {
        Some(s) => *s = slot,
        None => f.bar.push(slot),
    }
}

/// Fill slot `i` of fighter `f`'s bar with a skill from the fight's pool. A round's first draw is of the school's
/// signature kind; while the foe carries a status, one draw in `PAY_DRAW` is a skill that spends it. A stale skill's
/// replacement is never that skill (`gone`). A draw of a skill already on the bar raises that skill's tier, and the
/// slot draws again from the skills not on the bar; with none left it waits.
pub fn draw(state: &mut State, f: usize, i: usize, first: bool, gone: Option<&str>) {
    let me = &state.fighters[f];
    let held: Vec<&'static str> = me
        .bar
        .iter()
        .enumerate()
        .filter(|&(j, _)| j != i)
        .filter_map(|(_, slot)| slot.spell())
        .collect();
    let mut choice: Vec<&'static str> = me
        .pool
        .iter()
        .copied()
        .filter(|&name| Some(name) != gone)
        .collect();
    let foe = &state.fighters[1 - f];
    let carried = |status: Status| match status {
        Status::Shock => foe.shock.map_or(0.0, |s| s.stacks),
        Status::Chill => foe.chill,
    } > 0.0;
    let signature = school(me.school).signature;
    let want: Vec<&'static str> = if first {
        choice
            .iter()
            .copied()
            .filter(|name| spell(name).is_some_and(|s| s.kind == signature))
            .collect()
    } else {
        let pays: Vec<&'static str> = choice
            .iter()
            .copied()
            .filter(|name| payoff(name).is_some_and(|(status, _)| carried(status)))
            .collect();
        if state.rng.next() < PAY_DRAW {
            pays
        } else {
            Vec::new()
        }
    };
    if !want.is_empty() {
        choice = want;
    }
    let mut drawn = if choice.is_empty() {
        None
    } else {
        Some(choice[pick(state, choice.len())])
    };
    if let Some(name) = drawn
        && held.contains(&name)
    {
        let me = &state.fighters[f];
        let twin = me
            .bar
            .iter()
            .enumerate()
            .position(|(j, slot)| j != i && slot.spell() == Some(name));
        if let Some(twin) = twin {
            let raised = raise(state, &state.fighters[f].bar[twin].clone());
            let rarity = match &raised {
                Slot::Held { rarity, .. } => *rarity,
                Slot::Empty { .. } => Rarity::Common,
            };
            state.fighters[f].bar[twin] = raised;
            state.emit(Out::Tier {
                by: f,
                slot: twin,
                rarity,
            });
        }
        let free: Vec<&'static str> = state.fighters[f]
            .pool
            .iter()
            .copied()
            .filter(|&n| !held.contains(&n) && Some(n) != gone)
            .collect();
        drawn = if free.is_empty() {
            None
        } else {
            Some(free[pick(state, free.len())])
        };
    }
    let Some(name) = drawn else {
        // nothing left to draw: wait and look again
        put(
            &mut state.fighters[f],
            i,
            Slot::Empty {
                left: BAR.wait,
                full: BAR.wait,
            },
        );
        return;
    };
    let r = state.rng.next();
    let epic = RARITY[Rarity::Epic as usize].odds;
    let rarity = if r < epic {
        Rarity::Epic
    } else if r < epic + RARITY[Rarity::Rare as usize].odds {
        Rarity::Rare
    } else {
        Rarity::Common
    };
    let count = match rarity {
        Rarity::Common => usize::from(state.rng.next() < RARITY[0].chance.unwrap_or(0.0)),
        r => RARITY[r as usize].traits,
    };
    let traits = add_traits(state, name, &[], count);
    let power = slot_power(rarity, &traits, false);
    put(
        &mut state.fighters[f],
        i,
        Slot::Held {
            spell: name,
            power,
            evolved: false,
            rarity,
            traits,
            idle: 0.0,
        },
    );
}

/// `traits` and more that fit `name`, rolled, up to `count` (or as many as fit).
pub fn add_traits(state: &mut State, name: &str, traits: &[Trait], count: usize) -> Vec<Trait> {
    let mut fitting: Vec<Trait> = Trait::ALL
        .into_iter()
        .filter(|&t| fits(t, name) && !traits.contains(&t))
        .collect();
    let mut out = traits.to_vec();
    while out.len() < count && !fitting.is_empty() {
        let k = pick(state, fitting.len());
        out.push(fitting.remove(k));
    }
    out
}

/// A bar slot one tier up (a legendary stays legendary): more power, and the traits its new tier brings.
pub fn raise(state: &mut State, slot: &Slot) -> Slot {
    let Slot::Held {
        spell,
        evolved,
        rarity,
        traits,
        idle,
        ..
    } = slot
    else {
        return slot.clone();
    };
    let rarity = rarity.up();
    let traits = add_traits(state, spell, traits, RARITY[rarity as usize].traits);
    let power = slot_power(rarity, &traits, *evolved);
    Slot::Held {
        spell,
        power,
        evolved: *evolved,
        rarity,
        traits,
        idle: *idle,
    }
}

/// The cast of slot `i`'s skill empties the slot, which counts down before it draws again, unless the skill's partner
/// is on the bar: then it refills at once, evolved, for one more cast.
pub fn spend(state: &mut State, f: usize, i: usize) {
    let me = &state.fighters[f];
    let Some(Slot::Held {
        spell: name,
        evolved,
        rarity,
        traits,
        ..
    }) = me.bar.get(i)
    else {
        return;
    };
    let partner = evolves(me.school)
        .iter()
        .find(|pair| pair.contains(name))
        .and_then(|pair| pair.iter().copied().find(|n| n != name));
    let paired = partner.is_some_and(|p| {
        me.bar
            .iter()
            .enumerate()
            .any(|(j, other)| j != i && other.spell() == Some(p))
    });
    if !evolved && paired {
        let slot = Slot::Held {
            spell: name,
            power: slot_power(*rarity, traits, true),
            evolved: true,
            rarity: *rarity,
            traits: traits.clone(),
            idle: 0.0,
        };
        state.fighters[f].bar[i] = slot;
        return;
    }
    let wait = BAR.wait + BAR.spread * state.rng.next();
    state.fighters[f].bar[i] = Slot::Empty {
        left: wait,
        full: wait,
    };
}

/// A slot of the bar as the screen shows it.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct SlotView {
    pub spell: Option<&'static str>,
    pub empty: bool,
    pub ultimate: bool,
    pub ready: bool,
    /// The cooldown (or the empty slot's wait) left, out of `full`.
    pub left: f64,
    pub full: f64,
    /// How charged it is: an ultimate charges with rage.
    pub charge: f64,
    pub power: f64,
    pub evolved: bool,
    pub rarity: Rarity,
    pub traits: Vec<Trait>,
    /// How near it is to leaving the bar unused, 0 to 1.
    pub stale: f64,
}

const NO_SLOT: SlotView = SlotView {
    spell: None,
    empty: true,
    ultimate: false,
    ready: false,
    left: 0.0,
    full: 0.0,
    charge: 1.0,
    power: 1.0,
    evolved: false,
    rarity: Rarity::Common,
    traits: Vec::new(),
    stale: 0.0,
};

/// What fighter `f`'s skill bar shows, slot by slot (1, 2, 3, then the ultimate).
pub fn skill_bar(f: &Fighter) -> Vec<SlotView> {
    let mut slots: Vec<SlotView> = f
        .bar
        .iter()
        .map(|slot| match slot {
            Slot::Empty { left, full } => SlotView {
                left: *left,
                full: *full,
                ..NO_SLOT
            },
            Slot::Held {
                spell,
                power,
                evolved,
                rarity,
                traits,
                idle,
            } => SlotView {
                spell: Some(spell),
                empty: false,
                ready: f.hp > 0.0 && ready(f, spell),
                left: f.cool(spell),
                full: f.cool_full(spell),
                power: *power,
                evolved: *evolved,
                rarity: *rarity,
                traits: traits.clone(),
                stale: js_min(1.0, idle / STALE),
                ..NO_SLOT
            },
        })
        .collect();
    while slots.len() < BAR.slots {
        slots.push(NO_SLOT);
    }
    let ult = school(f.school).ultimate;
    let owned = f.spells.contains(&ult);
    slots.push(SlotView {
        spell: owned.then_some(ult),
        empty: !owned,
        ultimate: true,
        ready: owned && f.hp > 0.0 && ready(f, ult),
        left: f.cool(ult),
        full: f.cool_full(ult),
        charge: f.rage / RAGE,
        ..NO_SLOT
    });
    slots
}

/// A buff or debuff the HUD shows under a fighter's bars, with its stacks (0: none to show).
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Buff {
    pub id: &'static str,
    pub good: bool,
    pub stacks: f64,
}

/// What the HUD shows under fighter `f`'s bars, good first, then bad: its school's passives, then each thing it
/// carries. Nothing once it is knocked out.
pub fn buffs_of(f: &Fighter) -> Vec<Buff> {
    if f.hp <= 0.0 {
        return Vec::new();
    }
    let shock = f.shock.map_or(0.0, |s| s.stacks);
    let all = [
        ("fury", true, f.fury > 0.0, f.fury),
        ("unyielding", true, f.stats.unyielding && !f.rose, 0.0),
        ("attune", true, f.stats.attuned, f.attune % 2.0),
        ("ward", true, f.charges > 0.0, f.charges),
        ("barrier", true, f.barrier > 0.0, 0.0),
        ("grit", true, f.grit >= 1.0, libm::floor(f.grit)),
        ("burn", false, f.burn.is_some(), 0.0),
        ("chill", false, f.chill > 0.0, f.chill),
        ("shock", false, shock > 0.0, shock),
        ("frozen", false, f.body.frozen > 0.0, 0.0),
        ("defDown", false, f.def_down > 0.0, 0.0),
        ("silence", false, f.silence > 0.0, 0.0),
        ("sealed", false, f.sealed > 0.0, 0.0),
    ];
    all.into_iter()
        .filter(|&(_, _, on, _)| on)
        .map(|(id, good, _, stacks)| Buff { id, good, stacks })
        .collect()
}
