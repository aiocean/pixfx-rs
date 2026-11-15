//! Statuses that stack, hard controls cut by grit, burns, life lost and damage dealt, as `duel.js` has them. Each
//! takes the state and the index of the fighter it touches, and emits what it did in `duel.js`'s order.

use crate::data::{BURN, Blow, CHILL, DEF_DOWN, DEFENCE, FREEZE, FURY, GRIT, RAGE, SHOCK, SPREAD, shock_interval};
use crate::fighter::{Fighter, Shocked, State};
use crate::out::Out;
use pixfx_draw::math::{js_max, js_min, js_round};

/// A hard control: frozen (no moving, no acting), held by a shock, or stunned.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ControlKind {
    Freeze,
    Hold,
    Stun,
}

/// Fighter `f` takes a hard control of `kind` for `seconds`, cut by its grit; returns how long it holds.
pub fn control(state: &mut State, f: usize, kind: ControlKind, seconds: f64) -> f64 {
    let me = &state.fighters[f];
    if me.hp <= 0.0 {
        return 0.0;
    }
    if kind == ControlKind::Freeze && me.grit >= GRIT.thaw {
        add_chill(state, f, 1.0);
        return 0.0;
    }
    let me = &mut state.fighters[f];
    let time = seconds / (1.0 + me.grit);
    me.grit += GRIT.per;
    let body = &mut me.body;
    match kind {
        ControlKind::Freeze => body.frozen = js_max(body.frozen, time),
        ControlKind::Hold => body.shocked = js_max(body.shocked, time),
        ControlKind::Stun => {
            body.stun = js_max(body.stun, time);
            body.shocked = js_max(body.shocked, time);
        }
    }
    time
}

/// Fighter `f` takes `n` chill stacks; the last one it can hold freezes it.
pub fn add_chill(state: &mut State, f: usize, n: f64) {
    let me = &mut state.fighters[f];
    if me.hp <= 0.0 {
        return;
    }
    me.chill = js_min(CHILL.max, me.chill + n);
    me.chill_t = CHILL.time;
    if me.chill >= CHILL.max {
        me.chill = 0.0;
        me.chill_t = 0.0;
        control(state, f, ControlKind::Freeze, FREEZE);
    }
}

/// Fighter `f` takes `n` shock stacks from fighter `by`.
pub fn add_shock(state: &mut State, f: usize, by: usize, n: f64) {
    let me = &mut state.fighters[f];
    if me.hp <= 0.0 {
        return;
    }
    let mut shock = me.shock.unwrap_or(Shocked { stacks: 0.0, tick: 0.0, by });
    shock.stacks = js_min(SHOCK.max, shock.stacks + n);
    if shock.tick <= 0.0 {
        shock.tick = shock_interval(shock.stacks);
    }
    shock.by = by;
    me.shock = Some(shock);
}

/// Fighter `f`'s shock stacks twitch, one a tick, faster the more there are.
pub fn shocking(state: &mut State, f: usize, dt: f64) {
    let Some(mut shock) = state.fighters[f].shock else { return };
    shock.tick -= dt;
    state.fighters[f].shock = Some(shock);
    if shock.tick > 0.0 || state.fighters[f].hp <= 0.0 {
        return;
    }
    let by = shock.by;
    let amount = js_round(state.fighters[by].stats.atk * SHOCK.share);
    let share = take(state, f, amount);
    let (x, y) = (state.fighters[f].body.x, state.fighters[f].chest());
    state.emit(Out::Tick { by, damage: amount, share, x, y });
    control(state, f, ControlKind::Hold, SHOCK.hold);
    let me = &mut state.fighters[f];
    let Some(mut shock) = me.shock else { return };
    shock.stacks -= 1.0;
    if shock.stacks <= 0.0 {
        me.shock = None;
    } else {
        shock.tick = shock_interval(shock.stacks);
        me.shock = Some(shock);
    }
}

/// A burn on fighter `f` ticks: a share of its caster's attack, past defence, until it runs out.
pub fn burning(state: &mut State, f: usize, dt: f64) {
    let Some(mut burn) = state.fighters[f].burn else { return };
    burn.t -= dt;
    burn.tick -= dt;
    state.fighters[f].burn = Some(burn);
    if burn.tick <= 0.0 && state.fighters[f].hp > 0.0 {
        burn.tick += BURN.every;
        state.fighters[f].burn = Some(burn);
        let share = take(state, f, burn.per);
        let (x, y) = (state.fighters[f].body.x, state.fighters[f].chest());
        state.emit(Out::Tick { by: burn.by, damage: burn.per, share, x, y });
    }
    if burn.t <= 0.0 {
        state.fighters[f].burn = None;
    }
}

/// Fighter `f` loses `amount` life, and an unyielding fighter stands up once a round from a killing blow with 1 life
/// left. Returns the share of full life lost.
pub fn take(state: &mut State, f: usize, amount: f64) -> f64 {
    let me = &mut state.fighters[f];
    let mut share = (amount / me.stats.hp) * 100.0;
    if share >= me.hp && me.stats.unyielding && !me.rose {
        me.rose = true;
        share = me.hp - 100.0 / me.stats.hp;
        let (x, y) = (me.body.x, me.chest());
        state.emit(Out::Unyielding { by: f, x, y });
    }
    let me = &mut state.fighters[f];
    me.hp = js_max(0.0, me.hp - share);
    share
}

/// Fighter `f` gains `n` rage, up to full.
pub fn gain_rage(f: &mut Fighter, n: f64) {
    f.rage = js_min(RAGE, f.rage + n);
}

/// The damage `by` does to `f` with a blow of `power` and `blow`; `roll` in [0, 1) spreads it.
pub fn damage_of(by: &Fighter, f: &Fighter, power: f64, blow: Blow, crit: bool, roll: f64) -> f64 {
    let atk = by.stats.atk * (1.0 + FURY.per * by.fury);
    let def = (if blow == Blow::Physical { f.stats.pdef } else { f.stats.mdef })
        * (if f.def_down > 0.0 { 1.0 - DEF_DOWN } else { 1.0 });
    let spread = 1.0 - SPREAD / 2.0 + SPREAD * roll;
    js_max(
        1.0,
        js_round(atk * (power / 10.0) * (DEFENCE / (DEFENCE + def)) * (if crit { by.stats.crit_damage } else { 1.0 }) * spread),
    )
}

/// A heal or a ward washes off what clings to fighter `f`: burn, silence, defence-down, chill and shock.
pub fn cleanse(f: &mut Fighter) {
    f.burn = None;
    f.silence = 0.0;
    f.def_down = 0.0;
    f.chill = 0.0;
    f.chill_t = 0.0;
    f.shock = None;
}
