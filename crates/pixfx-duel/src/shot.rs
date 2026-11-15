//! Spells in flight: bolts, orbs, zones (seals, icicles, collapses, barrages), sprays and voids.
//!
//! Every shot has an id. A tick runs the shots in flight at its start, by id, so a shot cast during the tick waits for
//! the next, and a shot ended earlier in the tick (two bolts meeting) does not run again.

use crate::act::{blocks, burst, cast, clash, hurt, play, turn_aside, zone_height};
use crate::data::{Blow, Spell, orb_offset, school, spell};
use crate::fighter::State;
use crate::out::{Out, PlayOpts};
use crate::status::{ControlKind, add_chill, add_shock, control, damage_of, take};
use pixfx_draw::math::{js_min, js_sign, or_one};

/// What kind of shot, and its own state.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ShotKind {
    /// A bolt at (`x`, `y` up from the ground), flying at `vx`.
    Bolt { x: f64, y: f64, vx: f64 },
    /// An orb whose origin drifts at `x`; the ball loops round it.
    Orb { x: f64 },
    /// A zone at `x`, `air` up, falling at `t = impact`; a seal shows `lead` seconds before; a barrage strike after
    /// the first `follow`s the foe and is `aimed` once.
    Zone { x: f64, air: f64, lead: f64, follow: bool, aimed: bool, shown: bool, fell: bool },
    /// A jet that sprays for the spell's `spray` seconds.
    Spray,
    /// A vortex at `x`, ticking every half second.
    Void { x: f64, tick: f64 },
}

/// A spell in flight, cast by fighter `owner`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shot {
    pub id: u32,
    pub owner: usize,
    pub name: &'static str,
    pub spell: &'static Spell,
    pub t: f64,
    pub key: Option<u32>,
    pub power: f64,
    pub stacks: f64,
    pub pierce: bool,
    pub kind: ShotKind,
}

/// Move shot `id` on by `dt`; a shot no longer in flight (ended earlier this tick) does nothing.
pub fn shot(state: &mut State, id: u32, dt: f64) {
    let Some(mut s) = state.shots.iter().find(|s| s.id == id).copied() else { return };
    if run(state, &mut s, dt)
        && let Some(slot) = state.shots.iter_mut().find(|x| x.id == id)
    {
        *slot = s;
    }
}

/// One tick of shot `s`; returns whether it is still in flight (false: it ended itself).
fn run(state: &mut State, s: &mut Shot, dt: f64) -> bool {
    s.t += dt;
    let foe = 1 - s.owner;
    let g = state.ground;
    let sp = s.spell;
    let blow = sp.blow.unwrap_or(Blow::Magic);
    match s.kind {
        ShotKind::Bolt { mut x, y, mut vx } => {
            x += vx * dt;
            s.kind = ShotKind::Bolt { x, y, vx };
            if let Some(key) = s.key {
                state.emit(Out::Move { key, x, y: g - y });
            }
            // two bolts meet head on, or a bolt meets an orb: both are spent
            let met = state.shots.iter().find_map(|o| {
                if o.id == s.id || o.owner == s.owner {
                    return None;
                }
                match o.kind {
                    ShotKind::Bolt { x: ox, y: oy, .. } if (ox - x).abs() < 6.0 && (oy - y).abs() < 8.0 => Some((*o, None)),
                    ShotKind::Orb { .. } => {
                        let (bx, by) = orb_at(o);
                        (libm::hypot(bx - x, by + y) < 10.0).then_some((*o, Some((bx, by))))
                    }
                    _ => None,
                }
            });
            if let Some((o, ball)) = met {
                match (o.kind, ball) {
                    (ShotKind::Bolt { x: ox, .. }, _) => clash(state, (ox + x) / 2.0, g - y, &[]),
                    (_, Some((bx, by))) => burst(state, o.spell.hit_fx, bx, g + by),
                    _ => {}
                }
                end_shot(state, &o);
                end_shot(state, s);
                return false;
            }
            // an ice mirror sends the bolt back the way it came, now its own
            let f = &state.fighters[foe];
            let reflects = f.barrier > 0.0 && f.barrier_spell.and_then(spell).is_some_and(|b| b.reflect);
            if reflects && overlaps(state, foe, x, y, 9.0) {
                s.owner = foe;
                vx = -vx;
                x += js_sign(vx) * 10.0;
                s.kind = ShotKind::Bolt { x, y, vx };
                turn_aside(state, foe, x, g - y);
                // the effect cannot turn round in flight: the old bolt goes out and a new one flies back, facing its
                // new way and in the colours of the one who sent it back
                stop_shot(state, s);
                let opts = PlayOpts {
                    palette: Some(&school(state.fighters[foe].school).bolt),
                    flip: Some(vx < 0.0),
                    seed: Some(foe as u32 + 7),
                    ..PlayOpts::default()
                };
                s.key = Some(play(state, sp.fx.unwrap_or(""), x, g - y, opts));
                return true;
            }
            if overlaps(state, foe, x, y, 3.0) {
                if !s.pierce && blocks(state, foe, x - vx, s.power, blow) {
                    clash(state, x, g - y, &[]);
                } else if hurt(state, foe, s.power, js_sign(vx), s.name, s.stacks) && sp.shock {
                    add_shock(state, foe, s.owner, s.stacks);
                }
                end_shot(state, s);
                return false;
            }
            if x < state.left - 40.0 || x > state.right + 40.0 {
                end_shot(state, s);
                return false;
            }
            true
        }
        ShotKind::Orb { mut x } => {
            // the orb's origin drifts toward the foe; the ball itself wanders round it on its own loop
            x += js_sign(state.fighters[foe].body.x - x) * sp.speed.unwrap_or(0.0) * dt;
            s.kind = ShotKind::Orb { x };
            if let Some(key) = s.key {
                state.emit(Out::Move { key, x, y: g });
            }
            let (bx, by) = orb_at(s);
            if overlaps(state, foe, bx, -by, sp.radius.unwrap_or(0.0)) {
                if !s.pierce && blocks(state, foe, bx, s.power, blow) {
                    clash(state, bx, g + by, &[]);
                } else {
                    let dir = or_one(js_sign(state.fighters[foe].body.x - bx));
                    if hurt(state, foe, s.power, dir, s.name, s.stacks) && sp.shock {
                        add_shock(state, foe, s.owner, s.stacks);
                    }
                }
                burst(state, sp.hit_fx, bx, g + by);
                end_shot(state, s);
                return false;
            }
            if s.t >= sp.life.unwrap_or(0.0) {
                end_shot(state, s);
                return false;
            }
            true
        }
        ShotKind::Zone { mut x, mut air, lead, follow, mut aimed, mut shown, mut fell } => {
            let fspec = state.fighters[foe].body.spec;
            // a strike of a barrage aims where the foe was a moment before it falls
            if follow && !aimed && s.t >= -sp.aim_ahead.unwrap_or(0.0) {
                aimed = true;
                x = crate::act::clamp(state.fighters[foe].body.x, state.left + 6.0, state.right - 6.0);
                air = if sp.grounded { 0.0 } else { state.fighters[foe].body.air };
            }
            if !shown && s.t >= 0.0 {
                shown = true;
                if let Some(key) = s.key {
                    state.emit(Out::Stop { key });
                }
                s.key = None;
                let y = g - zone_height(sp, &fspec) - air;
                cast(state, s.owner, sp.fx.unwrap_or(""), x, y, PlayOpts::default());
            }
            s.kind = ShotKind::Zone { x, air, lead, follow, aimed, shown, fell };
            if s.t < 0.0 {
                return true;
            }
            // a collapse drags the foe toward its heart while it sucks in
            let reach = sp.reach.unwrap_or(0.0);
            let pull = sp.pull.unwrap_or(0.0);
            let f = &mut state.fighters[foe];
            if pull != 0.0 && s.t < sp.pull_until.unwrap_or(0.0) && (x - f.body.x).abs() < reach && f.hp > 0.0 {
                let dx = x - f.body.x;
                f.body.x += js_sign(dx) * js_min(dx.abs(), pull * dt);
            }
            // a prison locks the space round it until it falls: no blinking out
            if sp.seals && !fell && (x - f.body.x).abs() < reach {
                f.sealed = 2.0 * dt;
            }
            if !fell && s.t >= sp.impact.unwrap_or(0.0) {
                fell = true;
                s.kind = ShotKind::Zone { x, air, lead, follow, aimed, shown, fell };
                let f = &state.fighters[foe];
                let near = (f.body.x - x).abs() < sp.radius.unwrap_or(0.0) + fspec.half_width;
                let level = if sp.grounded { f.body.air < 10.0 } else { (f.body.air - air).abs() < 22.0 };
                let dir = or_one(js_sign(f.body.x - x));
                if near && level && hurt(state, foe, s.power, dir, s.name, s.stacks) {
                    if let Some(freeze) = sp.freeze.filter(|v| *v != 0.0) {
                        control(state, foe, ControlKind::Freeze, freeze);
                    }
                    if sp.shock {
                        add_shock(state, foe, s.owner, s.stacks);
                    }
                }
            }
            if s.t >= sp.impact.unwrap_or(0.0) + 0.2 {
                remove_shot(state, s);
                return false;
            }
            true
        }
        ShotKind::Spray => {
            if s.t >= sp.spray.unwrap_or(0.0) {
                stop_shot(state, s);
                remove_shot(state, s);
                return false;
            }
            true
        }
        ShotKind::Void { x, mut tick } => {
            // the vortex drags the foe in, and burns while it stands under it
            let dx = x - state.fighters[foe].body.x;
            let f = &mut state.fighters[foe];
            if dx.abs() < sp.reach.unwrap_or(0.0) && f.hp > 0.0 {
                f.body.x += js_sign(dx) * js_min(dx.abs(), sp.pull.unwrap_or(0.0) * dt);
            }
            // it burns whoever stands under it, a tick every half second
            tick += dt;
            let under = sp.under.unwrap_or(0.0);
            if tick >= 0.5 {
                tick -= 0.5;
                if dx.abs() < under && state.fighters[foe].hp > 0.0 {
                    let by = s.owner;
                    let roll = state.rng.next();
                    let amount = damage_of(&state.fighters[by], &state.fighters[foe], sp.tick.unwrap_or(0.0), blow, false, roll);
                    let share = take(state, foe, amount);
                    let (fx, fy) = (state.fighters[foe].body.x, state.fighters[foe].chest());
                    state.emit(Out::Tick { by, damage: amount, share, x: fx, y: fy });
                    // a cold fire chills whoever stands in it, a stack a tick
                    if sp.chill {
                        add_chill(state, foe, s.stacks);
                    }
                }
            }
            s.kind = ShotKind::Void { x, tick };
            if s.t >= sp.life.unwrap_or(0.0) {
                burst(state, sp.end_fx, x, g - sp.height.unwrap_or(0.0));
                if dx.abs() < under {
                    hurt(state, foe, s.power, or_one(-js_sign(dx)), s.name, s.stacks);
                }
                end_shot(state, s);
                return false;
            }
            true
        }
    }
}

/// An orb's ball relative to the ground: its origin plus its loop at its age.
pub fn orb_at(s: &Shot) -> (f64, f64) {
    let x = match s.kind {
        ShotKind::Orb { x } => x,
        _ => 0.0,
    };
    let (ox, oy) = orb_offset(s.t);
    (x + ox, oy)
}

/// Does a point (`x`, `y` up from the ground) touch fighter `f`, give or take `r`?
pub fn overlaps(state: &State, f: usize, x: f64, y: f64, r: f64) -> bool {
    let b = &state.fighters[f].body;
    if b.untouchable() {
        return false;
    }
    let top = if b.duck > 0.0 { b.spec.duck } else { b.spec.body };
    (x - b.x).abs() < b.spec.half_width + r && y > b.air - r && y < b.air + top + r
}

/// Shot `s` ends: its effect stops and it leaves the list.
pub fn end_shot(state: &mut State, s: &Shot) {
    stop_shot(state, s);
    remove_shot(state, s);
}

/// Shot `s`'s effect stops.
pub fn stop_shot(state: &mut State, s: &Shot) {
    if let Some(key) = s.key {
        state.emit(Out::Stop { key });
    }
}

/// Shot `s` leaves the list.
fn remove_shot(state: &mut State, s: &Shot) {
    state.shots.retain(|x| x.id != s.id);
}
