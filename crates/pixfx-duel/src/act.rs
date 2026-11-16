//! Casting and blows: a cast begins, gathers, lands (`release`) and recovers; a blow is guarded, turned aside by a
//! barrier, erased by a ward, dodged or taken (`hurt`); knockouts; and the effect commands a cast plays. Each draws its
//! dice in a fixed order.

use crate::bar::{has_trait, slot_of, spend};
use crate::data::{
    BURN, Blow, CAST_CIRCLE, CHIP, CLASH, FRAMES, FURY, GUARD_COST, KNOCKBACK, Kind, LUCKY, Luck,
    RING, Spell, Status, TRAITS, Trait, WARD, block_chance, payoff, school, spell, tint,
};
use crate::fighter::{Act, Burning, Phase, Slot, State, Told};
use crate::out::{How, Out, PlayOpts};
use crate::shot::{Shot, ShotKind};
use crate::status::{
    ControlKind, add_chill, add_shock, cleanse, control, damage_of, gain_rage, take,
};
use pixfx_draw::math::{js_max, js_min, js_round, js_sign, or_one};
use pixfx_fighter::body::{Blow as BodyBlow, BodySpec, Doing};
use pixfx_mind::Outcome;

/// JS `clamp(v, lo, hi)`: `Math.max(lo, Math.min(hi, v))`.
pub fn clamp(v: f64, lo: f64, hi: f64) -> f64 {
    js_max(lo, js_min(hi, v))
}

/// A die: `Math.floor(rng() * n)`.
fn roll_index(state: &mut State, n: usize) -> usize {
    let i = libm::floor(state.rng.next() * n as f64) as usize;
    i.min(n.saturating_sub(1))
}

/// A seed for an effect: `Math.floor(rng() * 1e6)`.
fn roll_seed(state: &mut State) -> u32 {
    libm::floor(state.rng.next() * 1e6) as u32
}

/// Start effect `fx` at (`x`, `y`); returns its key.
pub fn play(state: &mut State, fx: &'static str, x: f64, y: f64, opts: PlayOpts) -> u32 {
    state.key += 1;
    let key = state.key;
    state.emit(Out::Play {
        key,
        fx,
        x,
        y,
        opts,
    });
    key
}

/// Play `fx` for fighter `me`'s spell, in its school's colours: `{ ...tint, seed: roll, ...opts }`.
pub fn cast(state: &mut State, me: usize, fx: &'static str, x: f64, y: f64, opts: PlayOpts) -> u32 {
    let base = tint(state.fighters[me].school, fx).copied();
    let seed = roll_seed(state);
    let merged = PlayOpts {
        palette: opts.palette.or(base.and_then(|t| t.palette)),
        accent: opts.accent.or(base.and_then(|t| t.accent)),
        stars: opts.stars.or(base.and_then(|t| t.stars)),
        fire: opts.fire.or(base.and_then(|t| t.fire)),
        glow: opts.glow.or(base.and_then(|t| t.glow)),
        width: opts.width.or(base.and_then(|t| t.width)),
        height: opts.height.or(base.and_then(|t| t.height)),
        flip: opts.flip,
        seed: opts.seed.or(Some(seed)),
    };
    play(state, fx, x, y, merged)
}

/// A streak of `me`'s qi from where it casts to (`x`, `y` up from the ground).
pub fn streak(state: &mut State, me: usize, x: f64, y: f64) {
    let f = &state.fighters[me];
    let (x0, y0) = (f.body.x + f.body.face * 6.0, f.chest());
    state.emit(Out::Streak {
        by: me,
        x0,
        y0,
        x1: x,
        y1: y,
    });
}

/// A spent spell's burst where it ended, when its effect is in the registry.
pub fn burst(state: &mut State, fx: Option<&'static str>, x: f64, y: f64) {
    if let Some(fx) = fx
        && state.has(fx)
    {
        let seed = roll_seed(state);
        play(
            state,
            fx,
            x,
            y,
            PlayOpts {
                seed: Some(seed),
                ..PlayOpts::default()
            },
        );
    }
}

/// Settle fighter `f`'s pose from what the rules have it doing; the body decides when it is off its feet or reeling.
pub fn pose(state: &mut State, f: usize) {
    let me = &mut state.fighters[f];
    let melee = me
        .act
        .is_some_and(|a| spell(a.spell).is_some_and(|s| s.kind == Kind::Melee));
    let doing = match me.act {
        Some(Act {
            phase: Phase::Windup,
            ..
        }) => Doing::Windup { melee },
        Some(Act {
            phase: Phase::Recover,
            ..
        }) => Doing::Recover { melee },
        None if me.guard > 0.0 => Doing::Guard,
        None if me.medit => Doing::Meditate,
        None if me.fly => Doing::Fly,
        None if me.intent != 0.0 => Doing::Walk,
        None => Doing::Idle,
    };
    let pose = me.body.pose_of(doing, me.hp > 0.0);
    me.body.set_pose(pose);
}

/// Fighter `me` begins casting `name` at `target`: drops its guard, pays for it, starts its cooldown, spends its bar
/// slot, and may be lucky.
pub fn begin(state: &mut State, me: usize, name: &'static str, target: Option<f64>) {
    let Some(sp) = spell(name) else { return };
    let f = &state.fighters[me];
    let slot = slot_of(f, name).cloned();
    let at = f.bar.iter().position(|s| s.spell() == Some(name));
    let cost = crate::bar::cost_of(f, name);
    state.fighters[me].qi -= cost;
    if sp.ultimate {
        let f = &mut state.fighters[me];
        f.rage = 0.0;
        let (x, y) = (f.body.x, f.chest());
        state.emit(Out::Ultimate { by: me, x, y });
    }
    let swift = if has_trait(slot.as_ref(), Trait::Swift) {
        TRAITS.swift_cooldown
    } else {
        1.0
    };
    let cooldown = sp.cooldown * swift + sp.windup;
    state.fighters[me].set_cool(name, cooldown, cooldown);
    // a bar skill's one cast empties its slot, and strikes with the power it was drawn with
    let (power, evolved) = match &slot {
        Some(Slot::Held { power, evolved, .. }) => (*power, *evolved),
        _ => (1.0, false),
    };
    // an evolved or lingering skill inflicts one more stack of what it inflicts, each
    let lingering = if has_trait(slot.as_ref(), Trait::Lingering) {
        TRAITS.lingering_stacks
    } else {
        0.0
    };
    let stacks = 1.0 + if evolved { 1.0 } else { 0.0 } + lingering;
    let pierce = has_trait(slot.as_ref(), Trait::Piercing);
    if let Some(at) = at {
        spend(state, me, at);
    }
    let mut act = Act {
        spell: name,
        phase: Phase::Windup,
        t: 0.0,
        target,
        power,
        stacks,
        pierce,
        circle: None,
        blocked: false,
    };
    // luck smiles on a bar skill now and then; Again keeps it on its slot, ready at once, for another cast
    if let (Some(held), Some(at)) = (slot, at)
        && state.rng.next() < LUCKY.chance
    {
        let kind = LUCKY.kinds[roll_index(state, LUCKY.kinds.len())];
        let f = &mut state.fighters[me];
        match kind {
            Luck::Double => act.power *= 2.0,
            Luck::Free => f.qi += cost,
            Luck::Again => {
                f.set_cool_left(name, 0.0);
                // an evolved refill already holds the skill; otherwise it comes back as it was
                if f.bar.get(at).and_then(Slot::spell) != Some(name)
                    && let Slot::Held {
                        spell,
                        power,
                        evolved,
                        rarity,
                        traits,
                        ..
                    } = held
                    && let Some(s) = f.bar.get_mut(at)
                {
                    *s = Slot::Held {
                        spell,
                        power,
                        evolved,
                        rarity,
                        traits,
                        idle: 0.0,
                    };
                }
            }
        }
        f.lucky = Some(crate::fighter::LuckyCast {
            kind,
            spell: name,
            slot: at,
            at: state.time,
        });
        let (x, y) = (f.body.x, f.chest() + 10.0);
        state.emit(Out::Lucky { by: me, kind, x, y });
    }
    state.fighters[me].act = Some(act);
    // a fighter casting is not guarding, as its pose shows
    state.fighters[me].guard = 0.0;
    set_meditating(state, me, false);
    // longer casts stand in a rune circle while they gather
    if !matches!(
        sp.kind,
        Kind::Melee | Kind::Blink | Kind::Dash | Kind::Barrier
    ) && state.has(CAST_CIRCLE)
    {
        let f = &state.fighters[me];
        let opts = PlayOpts {
            palette: Some(&school(f.school).circle),
            seed: Some(me as u32 + 1),
            ..PlayOpts::default()
        };
        let (x, g) = (f.body.x, state.ground);
        let key = play(state, CAST_CIRCLE, x, g, opts);
        if let Some(a) = state.fighters[me].act.as_mut() {
            a.circle = Some(key);
        }
    }
}

/// Fighter `me`'s cast moves on by `dt`: at the end of its windup it lands, at the end of its recovery it is over.
/// Then its pose follows.
pub fn advance(state: &mut State, me: usize, dt: f64) {
    if let Some(mut act) = state.fighters[me].act {
        act.t += dt;
        state.fighters[me].act = Some(act);
        if let Some(sp) = spell(act.spell) {
            if act.phase == Phase::Windup && act.t >= sp.windup {
                release(state, me, sp);
                // a blow rung back by a golden bell breaks the striker's own action
                if let Some(a) = state.fighters[me].act {
                    state.fighters[me].act = Some(Act {
                        phase: Phase::Recover,
                        t: 0.0,
                        ..a
                    });
                    if let Some(key) = a.circle {
                        state.emit(Out::Stop { key });
                    }
                }
            } else if act.phase == Phase::Recover && act.t >= recover_of(&act, sp) {
                state.fighters[me].act = None;
            }
        }
    }
    pose(state, me);
}

/// How long `act` of `sp` recovers: an unsafe skill whose blow was guarded recovers longer.
pub fn recover_of(act: &Act, sp: &Spell) -> f64 {
    if act.blocked && FRAMES.unsafe_on_block.contains(&act.spell) {
        sp.recover + FRAMES.blocked_recover
    } else {
        sp.recover
    }
}

/// Fighter `me`'s blow was guarded: its act remembers it.
fn guarded(state: &mut State, me: usize) {
    if let Some(a) = state.fighters[me].act.as_mut() {
        a.blocked = true;
    }
}

/// Is fighter `f` in its ultimate's opening, where nothing touches it?
fn opening(state: &State, f: usize) -> bool {
    state.fighters[f].act.is_some_and(|a| {
        a.phase == Phase::Windup
            && a.t < FRAMES.ultimate_invincible
            && spell(a.spell).is_some_and(|s| s.ultimate)
    })
}

/// The moment fighter `me`'s spell lands: it hits, flies, falls or heals.
pub fn release(state: &mut State, me: usize, sp: &'static Spell) {
    let Some(act) = state.fighters[me].act else {
        return;
    };
    let foe = 1 - me;
    let g = state.ground;
    let blow = sp.blow.unwrap_or(Blow::Magic);
    // the blow, with the power of the bar skill it was cast from
    let hit = sp.damage.unwrap_or(0.0) * act.power;
    {
        let f = &mut state.fighters[me];
        if f.stats.attuned && sp.kind != Kind::Melee && !sp.ultimate && sp.cost > 0.0 {
            f.attune += 1.0;
            if f.attune % 2.0 == 0.0 {
                gain_rage(f, 1.0);
            }
        }
    }
    let (mx, mair, face) = (
        state.fighters[me].body.x,
        state.fighters[me].body.air,
        state.fighters[me].body.face,
    );
    let mspec = state.fighters[me].body.spec;
    let fspec = state.fighters[foe].body.spec;
    match sp.kind {
        Kind::Melee => {
            state.fighters[me].body.vx += face * 50.0;
            if sp.dive {
                state.fighters[me].fly = false;
                // a dive that ends on the ground cracks it
                if let Some(slam) = sp.slam_fx
                    && mair < 2.0
                    && state.has(slam)
                {
                    cast(state, me, slam, mx + face * 6.0, g, PlayOpts::default());
                    state.emit(Out::Land {
                        x: mx,
                        strength: 1.0,
                    });
                }
            }
            let (fx, fair, fhp) = (
                state.fighters[foe].body.x,
                state.fighters[foe].body.air,
                state.fighters[foe].hp,
            );
            let range = sp.range.unwrap_or(0.0);
            if (fx - mx).abs() <= range + fspec.half_width
                && (fair - mair).abs() < mspec.chest + 4.0
                && fhp > 0.0
            {
                let slash_windup = spell("slash").map_or(0.0, |s| s.windup);
                let crossing = state.fighters[foe].act.is_some_and(|a| {
                    a.spell == "slash" && a.phase == Phase::Windup && a.t > slash_windup * 0.6
                });
                if !act.pierce && blocks(state, foe, mx, hit, blow) {
                    guarded(state, me);
                    clash(state, (mx + fx) / 2.0, g - mspec.chest - mair, &[me, foe]);
                } else if crossing {
                    // both blades swing at once: they meet
                    clash(state, (mx + fx) / 2.0, g - mspec.chest - mair, &[me, foe]);
                    state.fighters[foe].act = None;
                } else {
                    hurt(state, foe, hit, face, act.spell, act.stacks);
                }
            }
        }
        Kind::Bolt => {
            let y = mair + mspec.chest;
            let opts = PlayOpts {
                palette: Some(&school(state.fighters[me].school).bolt),
                flip: Some(face < 0.0),
                seed: Some(me as u32 + 7),
                ..PlayOpts::default()
            };
            let key = play(state, sp.fx.unwrap_or(""), mx + face * 8.0, g - y, opts);
            let vx = face * sp.speed.unwrap_or(0.0);
            push(
                state,
                me,
                act,
                sp,
                Some(key),
                hit,
                ShotKind::Bolt {
                    x: mx + face * 8.0,
                    y,
                    vx,
                },
            );
        }
        Kind::Orb => {
            let x = mx + face * 10.0;
            let key = play(
                state,
                sp.fx.unwrap_or(""),
                x,
                g,
                PlayOpts {
                    seed: Some(me as u32 + 3),
                    ..PlayOpts::default()
                },
            );
            push(state, me, act, sp, Some(key), hit, ShotKind::Orb { x });
        }
        Kind::Zone => {
            let x = clamp(
                act.target.unwrap_or(f64::NAN),
                state.left + 6.0,
                state.right - 6.0,
            );
            // what hangs in the air (a collapse) gathers at the foe's height; what falls (an icicle) hits the floor
            let air = if sp.grounded {
                0.0
            } else {
                state.fighters[foe].body.air
            };
            // a sealed spell first marks the spot with the caster's seal, which the foe can see and leave
            let sigil = sp.sigil.unwrap_or(0.0);
            let mark = if sigil > 0.0 && state.has(CAST_CIRCLE) {
                let opts = PlayOpts {
                    palette: Some(&school(state.fighters[me].school).circle),
                    seed: Some(me as u32 + 11),
                    ..PlayOpts::default()
                };
                Some(play(state, CAST_CIRCLE, x, g - air, opts))
            } else {
                None
            };
            // a barrage falls strike after strike, each where the foe will be when it lands
            let strikes = sp.strikes.unwrap_or(1.0);
            let mut k = 0.0;
            while k < strikes {
                let kind = ShotKind::Zone {
                    x,
                    air,
                    lead: sigil,
                    follow: k > 0.0,
                    aimed: false,
                    shown: false,
                    fell: false,
                };
                let key = if k == 0.0 { mark } else { None };
                let id = push(state, me, act, sp, key, hit, kind);
                if let Some(s) = state.shots.iter_mut().find(|s| s.id == id) {
                    s.t = -k * sp.every.unwrap_or(0.0) - sigil;
                }
                k += 1.0;
            }
            streak(state, me, x, air + zone_height(sp, &fspec));
        }
        Kind::Blast => {
            let range = sp.range.unwrap_or(0.0);
            let x = mx + face * range * 0.7;
            let key = cast(
                state,
                me,
                sp.fx.unwrap_or(""),
                x,
                g - mspec.chest - mair,
                PlayOpts::default(),
            );
            if sp.spray.is_some() {
                push(state, me, act, sp, Some(key), 0.0, ShotKind::Spray);
            }
            let (fx, fair, fhp) = (
                state.fighters[foe].body.x,
                state.fighters[foe].body.air,
                state.fighters[foe].hp,
            );
            if (fx - mx).abs() <= range + fspec.half_width
                && (fair - mair).abs() < fspec.body
                && fhp > 0.0
            {
                if !act.pierce && blocks(state, foe, mx, hit, blow) {
                    guarded(state, me);
                    clash(state, fx - face * 4.0, g - fspec.chest - fair, &[foe]);
                } else if hurt(state, foe, hit, face, act.spell, act.stacks) && sp.shock {
                    add_shock(state, foe, me, act.stacks);
                }
            }
        }
        Kind::Void => {
            let x = clamp(
                state.fighters[foe].body.x,
                state.left + 10.0,
                state.right - 10.0,
            );
            let key = cast(
                state,
                me,
                sp.fx.unwrap_or(""),
                x,
                g,
                PlayOpts {
                    seed: Some(me as u32 + 5),
                    ..PlayOpts::default()
                },
            );
            push(
                state,
                me,
                act,
                sp,
                Some(key),
                hit,
                ShotKind::Void { x, tick: 0.0 },
            );
            let height = sp.height.unwrap_or(0.0);
            streak(state, me, x, if height != 0.0 { height } else { 4.0 });
        }
        Kind::Dash => {
            // gone in a flash and back a stride past the foe, cutting through it on the way
            let from = mx;
            let to = clamp(
                mx + face * sp.distance.unwrap_or(0.0),
                state.left + mspec.half_width,
                state.right - mspec.half_width,
            );
            let fx = sp.fx.unwrap_or("");
            cast(state, me, fx, from, g - mair, PlayOpts::default());
            state.fighters[me].body.x = to;
            cast(state, me, fx, to, g - mair, PlayOpts::default());
            let y = mair + mspec.chest;
            state.emit(Out::Streak {
                by: me,
                x0: from,
                y0: y,
                x1: to,
                y1: y,
            });
            let lo = js_min(from, to) - fspec.half_width;
            let hi = js_max(from, to) + fspec.half_width;
            let (fx, fair, fhp) = (
                state.fighters[foe].body.x,
                state.fighters[foe].body.air,
                state.fighters[foe].hp,
            );
            if fx > lo && fx < hi && (fair - mair).abs() < fspec.body && fhp > 0.0 {
                if !act.pierce && blocks(state, foe, from, hit, blow) {
                    guarded(state, me);
                    clash(state, fx, g - fspec.chest - fair, &[foe]);
                } else {
                    hurt(state, foe, hit, face, act.spell, act.stacks);
                }
            }
        }
        Kind::Heal => {
            let f = &mut state.fighters[me];
            cleanse(f);
            // a heal on the bar heals by its power, as its damage would
            let gained = js_min(100.0 - f.hp, sp.amount.unwrap_or(0.0) * act.power);
            f.hp += gained;
            let (amount, x, y) = (js_round((gained / 100.0) * f.stats.hp), f.body.x, f.chest());
            state.emit(Out::Heal {
                by: me,
                amount,
                x,
                y,
            });
            cast(
                state,
                me,
                sp.fx.unwrap_or(""),
                mx,
                g - mspec.body - mair,
                PlayOpts::default(),
            );
        }
        Kind::Ward => {
            cleanse(&mut state.fighters[me]);
            let charges = 1.0 + roll_index(state, WARD.charges as usize) as f64;
            let f = &mut state.fighters[me];
            f.charges = charges;
            f.shield_t = WARD.time;
            let (x, y) = (f.body.x, f.chest());
            state.emit(Out::Ward {
                by: me,
                amount: charges,
                x,
                y,
            });
            let lift = if sp.feet { 0.0 } else { mspec.body };
            cast(
                state,
                me,
                sp.fx.unwrap_or(""),
                mx,
                g - lift - mair,
                PlayOpts::default(),
            );
        }
        Kind::Barrier => {
            drop_barrier(state, me);
            state.fighters[me].barrier = sp.time.unwrap_or(0.0);
            state.fighters[me].barrier_spell = Some(act.spell);
            let key = cast(
                state,
                me,
                sp.fx.unwrap_or(""),
                mx,
                g - mair,
                PlayOpts {
                    seed: Some(me as u32 + 21),
                    ..PlayOpts::default()
                },
            );
            state.fighters[me].barrier_key = Some(key);
            let y = state.fighters[me].chest();
            state.emit(Out::Barrier { by: me, x: mx, y });
        }
        Kind::Blink => {
            let fx = sp.fx.unwrap_or("");
            cast(state, me, fx, mx, g - mair, PlayOpts::default());
            let to = clamp(
                act.target.unwrap_or(f64::NAN),
                state.left + mspec.half_width,
                state.right - mspec.half_width,
            );
            state.fighters[me].body.x = to;
            cast(state, me, fx, to, g - mair, PlayOpts::default());
        }
    }
}

/// Put a new shot of fighter `me`'s cast in flight; returns its id.
fn push(
    state: &mut State,
    me: usize,
    act: Act,
    sp: &'static Spell,
    key: Option<u32>,
    power: f64,
    kind: ShotKind,
) -> u32 {
    state.shot_id += 1;
    let id = state.shot_id;
    state.shots.push(Shot {
        id,
        owner: me,
        name: act.spell,
        spell: sp,
        t: 0.0,
        key,
        power,
        stacks: act.stacks,
        pierce: act.pierce,
        kind,
    });
    id
}

/// How high a zone strikes: what hangs in the air (a collapse) at the foe's chest, what falls (an icicle) at the floor.
pub fn zone_height(sp: &Spell, foe: &BodySpec) -> f64 {
    if sp.grounded { 0.0 } else { foe.chest }
}

/// Does fighter `f` block a blow of `damage` from `from_x`? A block costs qi and still lets a little damage through
/// (chip); with too little qi left the guard breaks and the blow lands.
pub fn blocks(state: &mut State, f: usize, from_x: f64, damage: f64, blow: Blow) -> bool {
    let me = &mut state.fighters[f];
    if me.guard == 0.0 || me.body.reeling() || js_sign(from_x - me.body.x) != me.body.face {
        return false;
    }
    if me.qi < GUARD_COST {
        me.guard = 0.0;
        return false;
    }
    me.qi -= GUARD_COST;
    // a blow too heavy for the guard breaks through whole
    if state.rng.next() >= block_chance(damage) {
        return false;
    }
    // what gets through a guard is a share of the blow, cut by defence like any blow
    let chip = damage_of(
        &state.fighters[1 - f],
        &state.fighters[f],
        damage,
        blow,
        false,
        0.5,
    ) * CHIP;
    take(state, f, chip);
    true
}

/// Does fighter `f`'s barrier turn this blow aside? An ultimate breaks through it. A golden bell rings back at a
/// blade, palm or dash that struck it from close by: the striker takes a blow of its own, is stunned and thrown off.
pub fn turns_aside(state: &mut State, f: usize, dir: f64, name: &str) -> bool {
    let s = spell(name);
    let me = &state.fighters[f];
    if me.barrier == 0.0 || s.is_some_and(|s| s.ultimate) {
        return false;
    }
    let (x, air, spec) = (me.body.x, me.body.air, me.body.spec);
    turn_aside(state, f, x - dir * 8.0, state.ground - spec.chest - air);
    let me = &state.fighters[f];
    let Some(barrier_spell) = me.barrier_spell else {
        return true;
    };
    let counter = spell(barrier_spell).and_then(|b| b.counter);
    let by = 1 - f;
    let other = &state.fighters[by];
    let close = (other.body.x - x).abs() < 60.0 && (other.body.air - air).abs() < spec.body;
    let struck_close = s.is_some_and(|s| matches!(s.kind, Kind::Melee | Kind::Blast | Kind::Dash));
    if let Some(counter) = counter
        && close
        && struck_close
    {
        let (x1, y1) = (other.body.x, other.chest());
        let y0 = state.fighters[f].chest();
        state.emit(Out::Streak {
            by: f,
            x0: x,
            y0,
            x1,
            y1,
        });
        if hurt(
            state,
            by,
            counter.damage,
            or_one(js_sign(x1 - x)),
            barrier_spell,
            1.0,
        ) {
            control(state, by, ControlKind::Stun, counter.stun);
        }
    }
    true
}

/// Fighter `f`'s barrier rings where a blow struck it, at (`x`, `y`) on screen.
pub fn turn_aside(state: &mut State, f: usize, x: f64, y: f64) {
    state.fighters[f].rang = RING;
    let (fx, fy) = (state.fighters[f].body.x, state.fighters[f].chest());
    state.emit(Out::Block {
        by: f,
        x: fx,
        y: fy,
    });
    if state.has(CLASH) {
        cast(
            state,
            f,
            CLASH,
            x,
            y + 30.0,
            PlayOpts {
                height: Some(30.0),
                seed: Some(f as u32 + 31),
                ..PlayOpts::default()
            },
        );
    }
}

/// Fighter `f`'s barrier falls, and its dome goes.
pub fn drop_barrier(state: &mut State, f: usize) {
    if let Some(key) = state.fighters[f].barrier_key {
        state.emit(Out::Stop { key });
    }
    state.fighters[f].barrier = 0.0;
    state.fighters[f].barrier_key = None;
}

/// Fighter `f` takes a blow of `power` from spell `name`, pushed toward `dir`; reported as a `Hit` (or a `Dodge`).
/// Caught gathering a cast it is a COUNTER, caught recovering a PUNISH (never from a field's strike): harder, and
/// staggered longer. An ultimate's opening lets the blow through, drawing no dice. Returns whether it landed.
pub fn hurt(
    state: &mut State,
    f: usize,
    power: f64,
    dir: f64,
    name: &'static str,
    stacks: f64,
) -> bool {
    if state.fighters[f].hp <= 0.0 || state.fighters[f].body.untouchable() {
        return false;
    }
    if opening(state, f) {
        let me = &state.fighters[f];
        let (x, y) = (me.body.x, me.chest());
        state.emit(Out::Dodge { x, y });
        return false;
    }
    if turns_aside(state, f, dir, name) {
        return false;
    }
    let by = 1 - f;
    // a ward's charge erases the whole blow
    if state.fighters[f].charges > 0.0 {
        state.fighters[f].charges -= 1.0;
        let me = &state.fighters[f];
        let y = state.ground - me.body.spec.chest - me.body.air;
        let x = me.body.x;
        clash(state, x, y, &[f]);
        return false;
    }
    let s = spell(name);
    let ultimate = s.is_some_and(|s| s.ultimate);
    // an ultimate cannot be dodged
    if !ultimate && state.rng.next() < state.fighters[f].stats.dodge {
        let me = &mut state.fighters[f];
        me.dodged = 0.3;
        me.body.vx -= dir * 40.0;
        let (x, y) = (me.body.x, me.chest());
        state.emit(Out::Dodge { x, y });
        return false;
    }
    // a payoff spends the stacks of its status on f, a little more power for each
    let mut power = power;
    if let Some((status, per)) = payoff(name) {
        let me = &mut state.fighters[f];
        let on = match status {
            Status::Shock => me.shock.map_or(0.0, |k| k.stacks),
            Status::Chill => me.chill,
        };
        power *= 1.0 + per * on;
        match status {
            Status::Shock => me.shock = None,
            Status::Chill => {
                me.chill = 0.0;
                me.chill_t = 0.0;
            }
        }
    }
    // caught at a weak moment, decided from what it was doing before the blow breaks it
    let how = match state.fighters[f].act.map(|a| a.phase) {
        _ if s.is_some_and(|s| s.kind == Kind::Void) => How::Plain,
        Some(Phase::Windup) => How::Counter,
        Some(Phase::Recover) => How::Punish,
        None => How::Plain,
    };
    let bonus = match how {
        How::Plain => None,
        How::Counter => Some(FRAMES.counter),
        How::Punish => Some(FRAMES.punish),
    };
    if let Some(b) = bonus {
        power *= b.power;
    }
    let crit = state.rng.next() < state.fighters[by].stats.crit;
    let roll = state.rng.next();
    let blow = s.and_then(|s| s.blow).unwrap_or(Blow::Magic);
    let amount = damage_of(
        &state.fighters[by],
        &state.fighters[f],
        power,
        blow,
        crit,
        roll,
    );
    let share = take(state, f, amount);
    // a frost blow freezes where it strikes: it holds the target in place rather than throwing it
    let freeze = s.is_some_and(|s| s.freeze.is_some_and(|v| v != 0.0));
    let launch = BodyBlow {
        power,
        dir: 1.0,
        freeze,
        flying: false,
    }
    .heavy(&BodySpec::default());
    let kind = s.map(|s| s.kind);
    let ranged = matches!(kind, Some(Kind::Bolt | Kind::Orb));
    let (x, y) = (state.fighters[f].body.x, state.fighters[f].chest());
    state.emit(Out::Hit {
        by,
        spell: name,
        power,
        launch,
        ranged,
        damage: amount,
        crit,
        how,
        share,
        x,
        y,
    });
    // rage for the blow: an attuned fighter's spells were paid for when cast, so only its blade earns here
    let attacker = &mut state.fighters[by];
    if !ultimate && !(attacker.stats.attuned && kind != Some(Kind::Melee)) {
        gain_rage(attacker, 1.0);
    }
    state.tell(
        by,
        Told::Outcome(Outcome::Landed {
            skill: name.to_string(),
        }),
    );
    let attacker = &mut state.fighters[by];
    if attacker.stats.fury {
        attacker.fury = js_min(FURY.max, attacker.fury + 1.0);
    }
    // a heavy blow throws it up, where it can be hit again before it lands; a lighter one staggers it where it stands
    let me = &mut state.fighters[f];
    let flying = me.fly;
    me.body.hit(BodyBlow {
        power,
        dir,
        freeze,
        flying,
    });
    // a blow that staggers rather than throws holds a fighter caught at a weak moment longer
    if let Some(b) = bonus
        && me.body.down.is_none()
    {
        me.body.stun += b.stagger;
    }
    me.fly = false;
    // struck while recovering from a move: the move was punished, and its mind learns so
    if let Some(a) = me.act
        && a.phase == Phase::Recover
    {
        state.tell(
            f,
            Told::Outcome(Outcome::Punished {
                skill: a.spell.to_string(),
            }),
        );
    }
    // a hit breaks a cast: what was gathering is lost
    break_act(state, f);
    state.fighters[f].guard = 0.0;
    set_meditating(state, f, false);
    // what the spell leaves behind
    let Some(s) = s else { return true };
    if s.burn {
        let a = &state.fighters[by];
        let per = js_round(a.stats.atk * (1.0 + FURY.per * a.fury) * BURN.share);
        state.fighters[f].burn = Some(Burning {
            t: BURN.time,
            tick: BURN.every,
            per,
            by,
        });
    }
    if let Some(d) = s.def_down.filter(|d| *d != 0.0) {
        state.fighters[f].def_down = d;
    }
    if let Some(d) = s.silence.filter(|d| *d != 0.0) {
        state.fighters[f].silence = d;
    }
    if s.chill {
        add_chill(state, f, stacks);
    }
    if let Some(steal) = s.steal_rage.filter(|v| *v != 0.0)
        && state.fighters[f].rage > 0.0
    {
        state.fighters[f].rage -= steal;
        gain_rage(&mut state.fighters[by], steal);
    }
    true
}

/// Two blows meet at (`x`, `y` on screen): a clash, its spark, and the fighters in it pushed apart.
pub fn clash(state: &mut State, x: f64, y: f64, fighters: &[usize]) {
    let up = state.ground - y;
    state.emit(Out::Clash { x, y: up });
    if state.has(CLASH) {
        let seed = roll_seed(state);
        play(
            state,
            CLASH,
            x,
            y + 30.0,
            PlayOpts {
                height: Some(30.0),
                seed: Some(seed),
                ..PlayOpts::default()
            },
        );
    }
    for &f in fighters {
        let body = &mut state.fighters[f].body;
        body.vx += js_sign(or_one(body.x - x)) * KNOCKBACK;
    }
}

/// Fighter `f` is knocked out: the other wins the round, and both stop what they were doing.
pub fn knock_out(state: &mut State, f: usize) {
    let other = 1 - f;
    let (x, y) = (state.fighters[f].body.x, state.fighters[f].chest());
    state.emit(Out::Ko { x, y });
    // the knockout throws the loser off its feet, if the blow has not already
    let ox = state.fighters[other].body.x;
    state.fighters[f].body.knock_out(ox);
    state.fighters[f].fly = false;
    state.fighters[other].fly = false;
    state.winner = Some(other);
    state.wins[other] += 1;
    break_act(state, f);
    set_meditating(state, f, false);
    break_act(state, other);
    set_meditating(state, other, false);
}

/// Fighter `f` stops what it was doing: a cast still gathering loses its rune circle.
pub fn break_act(state: &mut State, f: usize) {
    if let Some(a) = state.fighters[f].act
        && a.phase == Phase::Windup
        && let Some(key) = a.circle
    {
        state.emit(Out::Stop { key });
    }
    state.fighters[f].act = None;
}

/// Between rounds: the loser falls and lies where it lands; the winner comes down and stands over it.
pub fn settle(state: &mut State, f: usize, dt: f64) {
    let foe_x = state.fighters[1 - f].body.x;
    let won = state.winner == Some(f);
    let arena = state.arena();
    let State { rng, fighters, .. } = state;
    let me = &mut fighters[f];
    let alive = me.hp > 0.0;
    let events = me
        .body
        .settle(dt, won, alive, foe_x, &arena, &mut || rng.next());
    for e in events {
        state.emit(e.into());
    }
}

/// Fighter `f` sits down to meditate, or gets up; it sits in a spirit flame while it does.
pub fn set_meditating(state: &mut State, f: usize, on: bool) {
    if state.fighters[f].medit == on {
        return;
    }
    state.fighters[f].medit = on;
    if on && state.has("fire/spirit-flame") {
        let x = state.fighters[f].body.x;
        let y = state.ground - 8.0;
        let key = play(
            state,
            "fire/spirit-flame",
            x,
            y,
            PlayOpts {
                seed: Some(f as u32 + 11),
                ..PlayOpts::default()
            },
        );
        state.fighters[f].aura = Some(key);
    } else if !on && let Some(key) = state.fighters[f].aura {
        state.emit(Out::Stop { key });
        state.fighters[f].aura = None;
    }
}
