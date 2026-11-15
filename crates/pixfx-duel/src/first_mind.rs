//! The duel's first mind, the yardstick a brain is measured against: `site/tests/mind-baseline.js` as it is, its
//! numbers, its rules, its dice and its quirks (its ward check reads a field fighters never had, so it never wards).
//! It scores what it could do every `THINK` seconds while it is free, seeing only what has shown for `REACTION`
//! seconds, and does the best, with a little noise.

use crate::bar::ready;
use crate::data::{Kind, Spell, school, spell};
use crate::fighter::{Fighter, Phase, State};
use crate::rng::Rng;
use crate::seat::{MakeSeat, Seat};
use crate::shot::{ShotKind, orb_at};
use pixfx_draw::math::{js_max, js_min, js_sign, or_one};
use pixfx_mind::{Choice, Outcome};

const REACTION: f64 = 0.16;
const THINK: f64 = 0.12;
const SPREAD: f64 = 0.8;
const RISK: f64 = 0.6;
const FLOOR: f64 = 0.3;
const LEAD: f64 = 0.8;
const PRIOR: f64 = 6.0;
const BARRIER_THREAT: f64 = 10.0;

/// Something it could do, and how much it wants to.
#[derive(Clone, Debug)]
struct Opt {
    what: What,
    score: f64,
}

#[derive(Clone, Debug)]
enum What {
    Move(f64),
    Jump,
    Guard(f64),
    Duck(f64),
    Fly(f64),
    Meditate,
    Wait,
    Cast(&'static str, Option<f64>),
}

fn opt(what: What, score: f64) -> Opt {
    Opt { what, score }
}

/// The first mind of one fighter.
pub struct FirstMind {
    rng: Rng,
    think: f64,
    calm: f64,
    hold: f64,
    tried: Vec<(&'static str, f64)>,
    landed: Vec<(String, f64)>,
}

impl FirstMind {
    /// Fighter `me`'s first mind in a duel of seed `seed`.
    pub fn new(seed: u32, me: usize) -> FirstMind {
        FirstMind { rng: Rng::new(seed.wrapping_mul(2).wrapping_add(me as u32)), think: 0.0, calm: 0.0, hold: 0.0, tried: Vec::new(), landed: Vec::new() }
    }

    /// A maker for `Duel::new`.
    pub fn make() -> MakeSeat {
        Box::new(|state, me| Box::new(FirstMind::new(state.seed, me)))
    }

    fn tried(&self, name: &str) -> f64 {
        self.tried.iter().find(|(n, _)| *n == name).map_or(0.0, |&(_, v)| v)
    }

    fn landed(&self, name: &str) -> f64 {
        self.landed.iter().find(|(n, _)| n == name).map_or(0.0, |(_, v)| *v)
    }

    fn mind(&mut self, state: &State, me: usize, dt: f64) -> Option<Choice> {
        let f = &state.fighters[me];
        let foe = &state.fighters[1 - me];
        self.think -= dt;
        let b = &f.body;
        if self.think > 0.0 || f.act.is_some() || b.reeling() || b.squat > 0.0 || b.duck > 0.0 || b.frozen > 0.0 || b.shocked > 0.0 || f.hp <= 0.0 {
            return None;
        }
        self.think = THINK * (0.8 + self.rng.next() * 0.4);
        let mut threats = dangers(state, f);
        threats.extend(blows(state, f, foe));
        let mut options = threats.clone();
        options.extend(self.attacks(state, f, foe));
        options.extend(self.positions(state, f, foe));
        // something is about to land: raise the barrier, which answers any of it
        let barrier = f.spells.iter().copied().find(|n| spell(n).is_some_and(|s| s.kind == Kind::Barrier));
        let threat = threats.iter().fold(0.0, |m, o| js_max(m, o.score));
        if let Some(barrier) = barrier
            && f.barrier == 0.0
            && ready(f, barrier)
            && threat > BARRIER_THREAT
        {
            options.push(opt(What::Cast(barrier, None), threat * 1.25));
        }
        let mut best: Option<Opt> = None;
        for mut o in options {
            o.score *= 0.9 + self.rng.next() * 0.2;
            if best.as_ref().is_none_or(|b| o.score > b.score) {
                best = Some(o);
            }
        }
        best.map(|b| self.choice_of(f, b))
    }

    /// The option it chose as a Choice; an attack takes its breath and counts as tried.
    fn choice_of(&mut self, f: &Fighter, best: Opt) -> Choice {
        match best.what {
            What::Move(dir) => Choice::Move { dir },
            What::Jump => Choice::Jump,
            What::Guard(hold) => Choice::Guard { hold },
            What::Duck(hold) => Choice::Duck { hold },
            What::Fly(height) => Choice::Fly { height },
            What::Meditate => Choice::Meditate,
            What::Wait => Choice::Wait,
            What::Cast(name, target) => {
                if spell(name).and_then(|s| s.damage).is_some_and(|d| d != 0.0) {
                    self.calm = school(f.school).tempo * (1.0 - SPREAD / 2.0 + SPREAD * self.rng.next());
                    match self.tried.iter_mut().find(|(n, _)| *n == name) {
                        Some(e) => e.1 += 1.0,
                        None => self.tried.push((name, 1.0)),
                    }
                }
                Choice::Cast { skill: name.to_string(), target }
            }
        }
    }

    /// Each attack it could start now, scored by the damage it expects for what it costs.
    fn attacks(&self, state: &State, me: &Fighter, foe: &Fighter) -> Vec<Opt> {
        let sch = school(me.school);
        let d = (foe.body.x - me.body.x).abs();
        let fact = foe.act.and_then(|a| spell(a.spell).map(|s| (a, s)));
        // how long the foe cannot answer: it is gathering a spell, recovering from one, or staggered
        let busy = foe.body.out_for()
            + fact.map_or(0.0, |(a, s)| {
                let phase = if a.phase == Phase::Windup { s.windup } else { s.recover };
                phase - a.t + if a.phase == Phase::Windup { s.recover } else { 0.0 }
            });
        // a foe on the floor cannot be touched: a blow is only worth starting if it lands once the foe is up
        let open = foe.body.open_in();
        let free = !foe.body.reeling() && foe.act.is_none() && foe.body.frozen == 0.0 && foe.body.shocked == 0.0;
        let striking = fact.is_some_and(|(_, s)| matches!(s.kind, Kind::Melee | Kind::Blast | Kind::Dash));
        let opening = foe.body.out_for()
            + foe.body.frozen
            + foe.body.shocked
            + match fact {
                None => 0.0,
                Some((a, s)) if a.phase == Phase::Recover => s.recover - a.t,
                Some(_) if striking => 0.0,
                Some((a, s)) => s.windup - a.t + s.recover,
            };
        // ahead on life, a miss costs more than it gains; behind, less
        let lead = clamp((me.hp - foe.hp) / 40.0, -1.0, 1.0);
        let value = |name: &str, chance: f64, lands: Option<f64>| -> f64 {
            let Some(s) = spell(name) else { return 0.0 };
            let lands = lands.unwrap_or(s.windup);
            if lands < open {
                return 0.0;
            }
            // in a breath after its last attack, a fighter only punishes
            if self.calm > 0.0 && opening < lands {
                return 0.0;
            }
            let mut chance = chance;
            // a raised barrier turns everything but an ultimate aside: wait it out
            if foe.barrier > 0.0 && !s.ultimate {
                chance *= 0.05;
            }
            // what it has seen this attack do against this foe
            chance = (chance * PRIOR + self.landed(name)) / (PRIOR + self.tried(name));
            let gain = s.damage.unwrap_or(0.0) * chance * sch.aggression;
            let risk = if free { RISK * punisher(foe, me, s.recover) * (1.0 - chance) * (1.0 + LEAD * lead) } else { 0.0 };
            js_max(gain * FLOOR, gain - risk) / (0.5 + s.windup + s.cost / 40.0)
        };
        let sp = |n: &str| spell(n);
        let mut out = Vec::new();
        let dive_windup = sp("dive").map_or(0.0, |s| s.windup);
        if ready(me, "dive") && me.body.air - foe.body.air > 14.0 && d < 44.0 {
            let c = if busy > dive_windup { 0.9 } else if foe.guard > 0.0 { 0.25 } else { 0.6 };
            out.push(opt(What::Cast("dive", None), value("dive", c, None)));
        }
        let slash = sp("slash").copied();
        let (slash_windup, slash_range) = slash.map_or((0.0, 0.0), |s| (s.windup, s.range.unwrap_or(0.0)));
        if ready(me, "slash") && d < slash_range + 8.0 && (foe.body.air - me.body.air).abs() < me.body.spec.chest {
            let c = if busy > slash_windup { 0.95 } else if foe.guard > 0.0 { 0.15 } else { 0.55 };
            out.push(opt(What::Cast("slash", None), value("slash", c, None)));
        }
        // a bolt flies level from my chest: it must cross the foe's body
        let chest = me.body.air + me.body.spec.chest;
        let level = chest > foe.body.air - 2.0 && chest < foe.body.air + foe.body.spec.body;
        if ready(me, "bolt") && d > 24.0 && level {
            let bolt = sp("bolt").copied();
            let travel = d / bolt.and_then(|s| s.speed).unwrap_or(f64::NAN) + bolt.map_or(0.0, |s| s.windup);
            // a bolt into a bolt the foe is gathering only cancels it
            let trade = foe.act.is_some_and(|a| a.spell == "bolt" && a.phase == Phase::Windup);
            let c = if trade {
                0.1
            } else if busy > travel {
                0.9
            } else if foe.guard > 0.0 {
                0.2
            } else {
                0.45
            };
            out.push(opt(What::Cast("bolt", None), value("bolt", c, Some(travel))));
        }
        if ready(me, "orb") && d > 40.0 && !state.shots.iter().any(|s| matches!(s.kind, ShotKind::Orb { .. }) && s.owner == me.index) {
            // a slow ball that hunts the foe down: from far off it holds the space between and seldom misses
            out.push(opt(What::Cast("orb", None), value("orb", if d > 80.0 { 0.8 } else { 0.5 }, None)));
        }
        let pace = if foe.act.is_some() || foe.body.reeling() || foe.medit { 0.0 } else { foe.intent * foe.body.spec.walk };
        for name in ["tribulation", "prison"] {
            if !ready(me, name) {
                continue;
            }
            let Some(s) = sp(name) else { continue };
            let lead = s.windup + s.impact.unwrap_or(f64::NAN);
            if lead < open {
                continue;
            }
            let score = value(name, 0.8, Some(lead)) * s.strikes.unwrap_or(1.0) + 3.0;
            out.push(opt(What::Cast(s.name, Some(foe.body.x + pace * lead * 0.8)), score));
        }
        for &name in &me.spells {
            let Some(s) = sp(name) else { continue };
            if s.kind != Kind::Zone || s.ultimate || !ready(me, name) {
                continue;
            }
            // aim where the foe will be when it lands
            let lead = s.windup + s.sigil.unwrap_or(0.0) + s.impact.unwrap_or(f64::NAN);
            let target = foe.body.x + pace * lead * 0.8;
            // an icicle only finds a foe standing on the floor
            let grounded = if s.grounded {
                if foe.body.air < 10.0 && !foe.fly { 1.0 } else { 0.1 }
            } else {
                1.0
            };
            let pulls = s.pull.is_some_and(|p| p != 0.0);
            let c = if busy > lead * 0.6 {
                0.85
            } else if pulls {
                0.5
            } else {
                0.4
            };
            out.push(opt(What::Cast(name, Some(target)), value(name, c * grounded, Some(lead))));
        }
        for &name in &me.spells {
            let Some(s) = sp(name) else { continue };
            if s.kind == Kind::Blast && ready(me, name) && d < s.range.unwrap_or(f64::NAN) + 4.0 {
                let c = if busy > s.windup { 0.9 } else if foe.guard > 0.0 { 0.2 } else { 0.55 };
                out.push(opt(What::Cast(name, None), value(name, c, None)));
            }
            // a fire or a vortex under the foe: worth it on a foe that cannot leave it soon
            if s.kind == Kind::Void && ready(me, name) && (busy > 0.4 || foe.body.frozen > 0.0 || foe.hp < 75.0) {
                // it lands its blow when it ends, burning on the way
                out.push(opt(What::Cast(name, None), value(name, 0.7, Some(s.windup + s.life.unwrap_or(f64::NAN))) + 4.0));
            }
            // a lightning step: the foe within a stride, on my level
            if s.kind == Kind::Dash
                && ready(me, name)
                && d > 12.0
                && d < s.distance.unwrap_or(f64::NAN) - 10.0
                && (foe.body.air - me.body.air).abs() < me.body.spec.chest
            {
                let c = if busy > s.windup { 0.9 } else if foe.guard > 0.0 { 0.2 } else { 0.6 };
                out.push(opt(What::Cast(name, None), value(name, c, None)));
            }
        }
        // `me.shield === 0` in the JS never holds (fighters have `charges`, not `shield`): the first mind never wards
        if ready(me, "heal") && me.hp < 50.0 {
            let safe = if d > 60.0 && foe.act.is_none() {
                1.0
            } else if busy > 1.0 {
                0.8
            } else {
                0.2
            };
            let amount = sp("heal").and_then(|s| s.amount).unwrap_or(0.0);
            out.push(opt(What::Cast("heal", None), amount * safe * (1.0 - me.hp / 100.0)));
        }
        out
    }

    /// Moving to the distance it fights best at, meditating when it is quiet, or waiting.
    fn positions(&mut self, state: &State, me: &Fighter, foe: &Fighter) -> Vec<Opt> {
        let d = foe.body.x - me.body.x;
        let has = |kinds: &[Kind]| me.spells.iter().any(|n| spell(n).is_some_and(|s| kinds.contains(&s.kind)));
        let sch = school(me.school);
        let casting = me.qi >= 25.0;
        let slash_range = spell("slash").and_then(|s| s.range).unwrap_or(0.0);
        let range = if !casting {
            sch.dry
        } else if has(&[Kind::Bolt, Kind::Orb, Kind::Zone, Kind::Void]) {
            sch.range
        } else if has(&[Kind::Blast, Kind::Dash]) {
            34.0
        } else {
            slash_range - 4.0
        };
        let off = d.abs() - range;
        let mut out = vec![opt(What::Wait, 0.4)];
        let spec = &me.body.spec;
        // after an exchange, or with the foe behind a barrier, back off to spacing and let it come
        let reach = js_max(slash_range + spec.half_width + 6.0, range * 0.8);
        if (self.calm > 0.0 || foe.barrier > 0.0) && d.abs() < reach && (foe.body.air - me.body.air).abs() < spec.chest && !foe.body.reeling() {
            let back = or_one(-js_sign(d));
            let room = if back < 0.0 { me.body.x - state.left } else { state.right - me.body.x };
            if room > 14.0 {
                out.push(opt(What::Move(back), 2.4));
            }
        }
        // inside the foe's sword reach with no blade of my own ready: get out of it
        if d.abs() < slash_range + 4.0 && me.body.air - foe.body.air < spec.chest && !ready(me, "slash") {
            let back = or_one(-js_sign(d));
            let room = if back < 0.0 { me.body.x - state.left } else { state.right - me.body.x };
            if room > 14.0 {
                out.push(opt(What::Move(back), 2.2));
            }
        }
        if off.abs() > 8.0 {
            let mut dir = js_sign(d) * js_sign(off);
            // backed against a wall: step out sideways past the foe's reach instead
            if (dir < 0.0 && me.body.x - state.left < 12.0) || (dir > 0.0 && state.right - me.body.x < 12.0) {
                dir = 0.0;
            }
            if dir != 0.0 {
                out.push(opt(What::Move(dir), 0.6 + js_min(off.abs(), 60.0) / 40.0));
            }
        }
        // cornered with the foe inside my range: blink past it and take the open side
        let wall = js_min(me.body.x - state.left, state.right - me.body.x);
        if wall < 16.0 && off < -10.0 && ready(me, "blink") {
            let behind = foe.body.x + js_sign(foe.body.x - me.body.x) * 30.0;
            if behind > state.left + 10.0 && behind < state.right - 10.0 {
                out.push(opt(What::Cast("blink", Some(behind)), 2.5));
            }
        }
        // the height it wants: its school's while it has the qi, the foe's when it must close in with a blade
        let mut want = if me.qi > 40.0 && has(&[Kind::Bolt, Kind::Orb, Kind::Zone, Kind::Void]) { sch.sky } else { 0.0 };
        if !casting || d.abs() < 40.0 {
            want = if foe.body.air > 16.0 && me.qi > 20.0 { js_min(state.top, foe.body.air) } else { want };
        }
        let flying = if me.fly { me.hover } else { 0.0 };
        if (flying - want).abs() > 8.0 {
            out.push(opt(What::Fly(want), 1.2));
        }
        // light on its feet: on the ground and out of reach, it bounds about rather than walking flat
        if me.body.air == 0.0 && !me.fly && d.abs() > slash_range + 10.0 && self.rng.next() < 0.5 {
            out.push(opt(What::Jump, 1.3));
        }
        let threatened = state.shots.iter().any(|s| s.owner != me.index);
        if me.qi < 45.0 && d.abs() > 70.0 && !threatened && foe.act.is_none() && me.body.air == 0.0 && !me.fly {
            out.push(opt(What::Meditate, 2.2 - me.qi / 40.0));
        }
        out
    }
}

impl Seat for FirstMind {
    fn think(&mut self, state: &State, me: usize, dt: f64) -> Option<Choice> {
        self.calm = js_max(0.0, self.calm - dt);
        self.hold = js_max(0.0, self.hold - dt);
        if self.hold > 0.0 {
            return None;
        }
        self.mind(state, me, dt)
    }

    fn tell(&mut self, outcome: &Outcome) {
        if let Outcome::Landed { skill } = outcome {
            match self.landed.iter_mut().find(|(n, _)| n == skill) {
                Some(e) => e.1 += 1.0,
                None => self.landed.push((skill.clone(), 1.0)),
            }
        }
    }

    fn reset(&mut self) {
        self.think = 0.4 + self.rng.next() * 0.3;
        self.calm = 0.0;
        self.tried.clear();
        self.landed.clear();
        self.hold = 0.0;
    }

    fn hold(&mut self, seconds: f64) {
        self.hold = seconds;
    }
}

fn clamp(v: f64, lo: f64, hi: f64) -> f64 {
    js_max(lo, js_min(hi, v))
}

fn at_wall(state: &State, f: &Fighter) -> bool {
    let hw = f.body.spec.half_width;
    f.body.x <= state.left + hw + 2.0 || f.body.x >= state.right - hw - 2.0
}

/// What is coming at `me`, and the ways out of each.
fn dangers(state: &State, me: &Fighter) -> Vec<Opt> {
    let mut out: Vec<Opt> = Vec::new();
    let x = me.body.x;
    let spec = &me.body.spec;
    let blink_distance = spell("blink").and_then(|s| s.distance).unwrap_or(f64::NAN);
    // a blink away from the danger at `from`, unless that is a wall, scored `score`
    let blink = |out: &mut Vec<Opt>, from: f64, score: f64| {
        if !ready(me, "blink") {
            return;
        }
        let away = js_sign(or_one(x - from));
        let mut to = x + away * blink_distance;
        if to < state.left + 10.0 || to > state.right - 10.0 {
            to = x - away * blink_distance;
        }
        out.push(opt(What::Cast("blink", Some(to)), score));
    };
    for s in &state.shots {
        if s.owner == me.index {
            continue;
        }
        let sp: &Spell = s.spell;
        let damage = sp.damage.unwrap_or(f64::NAN);
        // a barrage still falling: its next strike aims where I stand, so keep on the move, toward the open side
        let follow = matches!(s.kind, ShotKind::Zone { follow: true, .. });
        if follow && s.t < 0.0 && s.t > -sp.every.unwrap_or(f64::NAN) - sp.aim_ahead.unwrap_or(f64::NAN) + REACTION {
            let way = if x - state.left > state.right - x { -1.0 } else { 1.0 };
            let dir = if me.body.vx != 0.0 { js_sign(me.body.vx) * if at_wall(state, me) { -1.0 } else { 1.0 } } else { way };
            out.push(opt(What::Move(dir), damage * 1.4));
            continue;
        }
        // a sealed spell is seen from the moment its seal shows
        let lead = match s.kind {
            ShotKind::Zone { lead, .. } => lead,
            _ => 0.0,
        };
        if s.t + lead < REACTION {
            continue;
        }
        match s.kind {
            ShotKind::Bolt { x: sx, y: sy, vx } => {
                let toward = js_sign(x - sx) == js_sign(vx);
                let tti = (x - sx).abs() / vx.abs();
                if !toward || tti > 0.6 || sy < me.body.air - 3.0 || sy > me.body.air + spec.body + 3.0 {
                    continue;
                }
                let urgency = damage * (1.2 - tti);
                if js_sign(sx - x) == me.body.face {
                    out.push(opt(What::Guard(tti + 0.15), urgency * 1.1));
                }
                if me.body.air == 0.0 && !me.fly && tti < 0.24 && tti > 0.08 {
                    out.push(opt(What::Jump, urgency * 1.25));
                }
                // a bolt over a ducking body's height: drop under it
                if me.body.air == 0.0 && !me.fly && sy > spec.duck + 3.0 && tti < 0.4 {
                    out.push(opt(What::Duck(tti + 0.12), urgency * 1.2));
                }
                // on the sword: rise over it, or sink under it
                if me.qi > 12.0 && tti > 0.15 {
                    let over = sy + 4.0;
                    if over < state.top {
                        out.push(opt(What::Fly(over), urgency * 1.15));
                    }
                    if me.fly && sy > spec.body + 2.0 {
                        out.push(opt(What::Fly(0.0), urgency * 1.1));
                    }
                }
                if ready(me, "bolt") && tti > 0.3 {
                    out.push(opt(What::Cast("bolt", None), urgency * 0.9));
                }
                blink(&mut out, sx, urgency * 0.7);
            }
            ShotKind::Orb { .. } => {
                let (bx, by) = orb_at(s);
                let d = libm::hypot(bx - x, -by - (me.body.air + spec.chest));
                if d > 48.0 {
                    continue;
                }
                let urgency = damage * (1.4 - d / 48.0);
                out.push(opt(What::Move(or_one(js_sign(x - bx))), urgency));
                if ready(me, "bolt") {
                    out.push(opt(What::Cast("bolt", None), urgency * 1.15));
                }
                blink(&mut out, bx, urgency * 0.9);
            }
            ShotKind::Zone { x: zx, fell: false, lead, .. } if s.t + lead >= sp.notice.unwrap_or(f64::NAN) => {
                let dx = x - zx;
                if dx.abs() > sp.reach.unwrap_or(0.0) + sp.radius.unwrap_or(f64::NAN) + spec.half_width + 4.0 {
                    continue;
                }
                let left = sp.impact.unwrap_or(f64::NAN) - s.t;
                let urgency = damage * (1.5 - left);
                let way = if dx.abs() < 1.0 {
                    if zx - state.left > state.right - zx { -1.0 } else { 1.0 }
                } else {
                    js_sign(dx)
                };
                out.push(opt(What::Move(way), urgency * 1.3));
                if sp.grounded && me.qi > 12.0 {
                    out.push(opt(What::Fly(24.0), urgency * 1.35));
                }
                blink(&mut out, zx, urgency * if left < 0.3 { 1.4 } else { 0.6 });
            }
            ShotKind::Void { x: vx, .. } => {
                let dx = x - vx;
                let reach = sp.reach.unwrap_or(f64::NAN);
                if dx.abs() > reach {
                    continue;
                }
                let urgency = 14.0 * (1.2 - dx.abs() / reach);
                out.push(opt(What::Move(or_one(js_sign(dx))), urgency * 1.2));
                blink(&mut out, vx, urgency * 1.1);
            }
            _ => {}
        }
    }
    out
}

/// The foe winding up a blow that reaches `me` (a slash or a palm): step out of reach, block, blink away, or strike
/// first. Seen only once the wind-up has shown for `REACTION` seconds.
fn blows(state: &State, me: &Fighter, foe: &Fighter) -> Vec<Opt> {
    let Some(a) = foe.act else { return Vec::new() };
    if a.phase != Phase::Windup || a.t < REACTION {
        return Vec::new();
    }
    let Some(s) = spell(a.spell) else { return Vec::new() };
    if s.kind != Kind::Melee && s.kind != Kind::Blast {
        return Vec::new();
    }
    let spec = &me.body.spec;
    let range = s.range.unwrap_or(f64::NAN);
    let d = (foe.body.x - me.body.x).abs();
    if d > range + spec.half_width + 6.0 {
        return Vec::new();
    }
    let left = s.windup - a.t;
    let urgency = s.damage.unwrap_or(f64::NAN) * 1.2;
    let mut out = Vec::new();
    // backing off works if I can walk out of reach before it lands
    let out_by = range + spec.half_width + 2.0 - d;
    if out_by / spec.walk < left + 0.05 {
        out.push(opt(What::Move(or_one(js_sign(me.body.x - foe.body.x))), urgency * 1.2));
    }
    if js_sign(foe.body.x - me.body.x) == me.body.face {
        out.push(opt(What::Guard(left + 0.2), urgency * 0.9));
    }
    let slash = spell("slash");
    let (sw, sr) = slash.map_or((0.0, 0.0), |s| (s.windup, s.range.unwrap_or(0.0)));
    if ready(me, "slash") && sw < left && d < sr + foe.body.spec.half_width {
        out.push(opt(What::Cast("slash", None), urgency * 1.1));
    }
    if ready(me, "blink") {
        let to = me.body.x + js_sign(or_one(me.body.x - foe.body.x)) * spell("blink").and_then(|s| s.distance).unwrap_or(f64::NAN);
        if to > state.left + 10.0 && to < state.right - 10.0 {
            out.push(opt(What::Cast("blink", Some(to)), urgency * 0.8));
        }
    }
    out
}

/// The hardest blow `foe` could land on `me` within `time` seconds: an attack it has ready that reaches me from where
/// it stands and lands in time.
fn punisher(foe: &Fighter, me: &Fighter, time: f64) -> f64 {
    let d = (foe.body.x - me.body.x).abs();
    let level = (foe.body.air - me.body.air).abs();
    let spec = &me.body.spec;
    let mut most = 0.0;
    for &name in &foe.spells {
        let Some(s) = spell(name) else { continue };
        let damage = s.damage.unwrap_or(0.0);
        if damage == 0.0 || s.ultimate || !ready(foe, name) {
            continue;
        }
        let range = s.range.unwrap_or(f64::NAN);
        let close = (s.kind == Kind::Melee && !s.dive && d < range + spec.half_width && level < spec.chest)
            || (s.kind == Kind::Blast && d < range + spec.half_width && level < spec.body)
            || (s.kind == Kind::Dash && d < s.distance.unwrap_or(f64::NAN) && level < spec.chest);
        let lands = if close {
            s.windup
        } else if s.kind == Kind::Bolt && level < spec.body {
            s.windup + d / s.speed.unwrap_or(f64::NAN)
        } else {
            f64::INFINITY
        };
        if lands <= time + REACTION {
            most = js_max(most, damage);
        }
    }
    most
}
