//! Every action a fighter could take this thought, with what it is worth on its own: the duel's first mind's
//! `dangers`, `blows`, `attacks` and `positions`, reasoning with skill kinds instead of names. Attacks are worth the
//! damage they are likely to land for what they cost, less what a miss costs (Sirlin's Playing to Win). The yomi
//! level in force lifts the answers to what the foe is predicted to do.

use crate::choice::Choice;
use crate::habits::FoeMove;
use crate::pick::Read;
use crate::posture::Category;
use crate::reading::{punisher, Reading};
use crate::sense::{Arena, Doing, Foe, Me, Phase, Shape, Threat};
use crate::skill::{BodySize, Profile, Skill, SkillKind};

/// A thing to do, its category, and what it is worth before the posture weighs it.
#[derive(Clone, Debug, PartialEq)]
pub struct Candidate {
    pub choice: Choice,
    pub category: Category,
    pub base: f64,
    /// The skill it uses, as its index in the fighter's skills.
    pub skill: Option<usize>,
}

/// What the candidates are built from.
pub struct Ctx<'a> {
    pub me: &'a Me,
    /// The foe as seen (late).
    pub foe: &'a Foe,
    /// The threats as seen (old enough).
    pub threats: &'a [Threat],
    pub arena: &'a Arena,
    pub skills: &'a [Skill],
    pub ready: &'a [bool],
    pub body: &'a BodySize,
    pub profile: &'a Profile,
    pub reading: &'a Reading,
    pub read: &'a Read,
    /// The breath after my last attack still left, seconds.
    pub calm: f64,
    pub now: f64,
    /// What the foe will likely do next (yomi level 1), and how sure.
    pub predicted: Option<(FoeMove, f64)>,
    /// What the foe likely answers my usual move with (yomi level 2), and how sure.
    pub answer: Option<(FoeMove, f64)>,
    pub level: u8,
    /// A die in `[0, 1)` for choices made on a whim (a hop).
    pub roll: f64,
}

/// A read below this confidence lifts nothing.
const SURE: f64 = 0.3;
const UP: f64 = 1.3;
const DOWN: f64 = 0.75;

fn category(kind: SkillKind) -> Category {
    match kind {
        SkillKind::Melee | SkillKind::Dive | SkillKind::Dash | SkillKind::Blast => Category::Close,
        SkillKind::Bolt | SkillKind::Seeker => Category::Ranged,
        SkillKind::Area | SkillKind::Field => Category::Area,
        SkillKind::Heal | SkillKind::Ward => Category::Buff,
        SkillKind::Barrier => Category::Defend,
        SkillKind::Blink => Category::Evade,
    }
}

fn sign(x: f64) -> f64 {
    if x < 0.0 { -1.0 } else { 1.0 }
}

struct Out<'c, 'a> {
    ctx: &'c Ctx<'a>,
    list: Vec<Candidate>,
}

impl Out<'_, '_> {
    fn push(&mut self, choice: Choice, category: Category, base: f64, skill: Option<usize>) {
        if base.is_finite() && base > 0.0 {
            self.list.push(Candidate { choice, category, base, skill });
        }
    }

    fn cast(&mut self, i: usize, target: Option<f64>, base: f64, as_: Option<Category>) {
        let Some(s) = self.ctx.skills.get(i) else { return };
        let category = as_.unwrap_or(category(s.kind));
        self.push(Choice::Cast { skill: s.name.clone(), target }, category, base, Some(i));
    }

    fn walk(&mut self, dir: f64, category: Category, base: f64) {
        self.push(Choice::Move { dir }, category, base, None);
    }
}

impl Ctx<'_> {
    /// The ready skill of `kind` (the cheapest, for a blade that is not a dive).
    fn ready_of(&self, kind: SkillKind) -> Option<usize> {
        self.skills
            .iter()
            .enumerate()
            .filter(|(i, s)| s.kind == kind && self.ready.get(*i).copied().unwrap_or(false))
            .min_by(|a, b| a.1.cost.total_cmp(&b.1.cost))
            .map(|(i, _)| i)
    }

    fn holds(&self, test: impl Fn(SkillKind) -> bool) -> bool {
        self.skills.iter().any(|s| test(s.kind))
    }

    fn room(&self, dir: f64) -> f64 {
        if dir < 0.0 { self.me.x - self.arena.left } else { self.arena.right - self.me.x }
    }

    fn blade(&self) -> f64 {
        self.skills.iter().filter(|s| s.kind == SkillKind::Melee).map(|s| s.reach).fold(24.0, f64::min)
    }

    fn guarding(&self) -> bool {
        matches!(self.foe.doing, Doing::Guard)
    }

    /// What attack `i` is worth, landing `lands` seconds after it starts with this `chance`.
    fn value(&self, i: usize, chance: f64, lands: f64) -> f64 {
        let Some(s) = self.skills.get(i) else { return 0.0 };
        let r = self.reading;
        if lands < r.open_in {
            return 0.0;
        }
        // in a breath after its last attack, a fighter only punishes: the foe must still be open when this lands
        if self.calm > 0.0 && r.opening < lands {
            return 0.0;
        }
        let mut chance = chance;
        // a raised barrier turns everything but an ultimate aside: wait it out
        if self.foe.barrier && !s.ultimate {
            chance *= 0.05;
        }
        let chance = self.read.chance(i, chance, self.profile.prior);
        let gain = s.damage * chance * self.profile.aggression;
        let risk = if r.free {
            self.profile.risk
                * punisher(self.foe, self.me, self.body, s.recover, self.profile.reaction)
                * (1.0 - chance)
                * (1.0 + self.profile.lead * r.lead)
        } else {
            0.0
        };
        (gain * self.profile.floor).max(gain - risk) / (0.5 + s.windup + s.cost / 40.0) * self.read.penalty(i, self.now)
    }
}

/// Every candidate for this thought.
pub fn candidates(ctx: &Ctx) -> Vec<Candidate> {
    let mut out = Out { ctx, list: Vec::new() };
    let threat = dangers(&mut out) ;
    let threat = threat.max(blows(&mut out));
    // something big is about to land: raise the barrier, which answers any of it
    if let Some(b) = ctx.ready_of(SkillKind::Barrier)
        && !ctx.me.barrier
        && threat > 10.0
    {
        out.cast(b, None, threat * 1.25, None);
    }
    attacks(&mut out);
    positions(&mut out);
    let mut list = out.list;
    for (read, on) in [(ctx.predicted, 1), (ctx.answer, 2)] {
        if ctx.level == on
            && let Some((m, sure)) = read
            && sure >= SURE
        {
            for c in &mut list {
                c.base *= answer(m, c);
            }
        }
    }
    list
}

/// How much a candidate is worth against a foe about to do `m`.
fn answer(m: FoeMove, c: &Candidate) -> f64 {
    use Category as C;
    match (m, c.category) {
        // a foe that will guard: go round it with an area or a blast, not into it with a blade
        (FoeMove::Guard, C::Area) => UP,
        (FoeMove::Guard, C::Close) if matches!(c.choice, Choice::Cast { .. }) => DOWN,
        // a foe that will come in: meet it
        (FoeMove::Approach, C::Close) => UP,
        // a foe that will attack: be ready for it
        (FoeMove::Close, C::Defend | C::Retreat) => UP,
        (FoeMove::Close, C::Close) => DOWN,
        // a foe that will shoot: close the gap or guard
        (FoeMove::Ranged, C::Approach | C::Close | C::Defend) => UP,
        (FoeMove::Area, C::Approach | C::Evade) => UP,
        (FoeMove::Meditate, C::Approach | C::Close | C::Ranged | C::Area) => UP,
        (FoeMove::Retreat, C::Ranged | C::Area) => UP,
        (FoeMove::Jump | FoeMove::Fly, C::Ranged) => UP,
        _ => 1.0,
    }
}

/// What is coming at me, and the ways out of each; returns the worst urgency.
fn dangers(out: &mut Out) -> f64 {
    let ctx = out.ctx;
    let (me, body, arena) = (ctx.me, ctx.body, ctx.arena);
    let grounded = me.air == 0.0 && !me.flying;
    let mut worst: f64 = 0.0;
    let blink = |out: &mut Out, from: f64, base: f64| {
        let Some(b) = ctx.ready_of(SkillKind::Blink) else { return };
        let d = ctx.skills[b].reach;
        let away = sign(me.x - from);
        let mut to = me.x + away * d;
        if to < arena.left + 10.0 || to > arena.right - 10.0 {
            to = me.x - away * d;
        }
        out.cast(b, Some(to), base, Some(Category::Evade));
    };
    for th in ctx.threats {
        match th.shape {
            Shape::Barrage { .. } => {
                // its next strike aims where I stand: keep moving, toward the open side
                let open = if me.x - arena.left > arena.right - me.x { -1.0 } else { 1.0 };
                let dir = if me.vx != 0.0 { sign(me.vx) * if ctx.room(sign(me.vx)) < 12.0 { -1.0 } else { 1.0 } } else { open };
                let u = th.damage * 1.4;
                worst = worst.max(u);
                out.walk(dir, Category::Evade, u);
            }
            Shape::Projectile { x, y, vx } => {
                if vx == 0.0 || sign(me.x - x) != sign(vx) {
                    continue;
                }
                let tti = (me.x - x).abs() / vx.abs();
                if tti > 0.6 || y < me.air - 3.0 || y > me.air + body.body + 3.0 {
                    continue;
                }
                let u = th.damage * (1.2 - tti);
                worst = worst.max(u);
                if sign(x - me.x) == me.face {
                    out.push(Choice::Guard { hold: tti + 0.15 }, Category::Defend, u * 1.1, None);
                }
                if grounded && tti < 0.24 && tti > 0.08 {
                    out.push(Choice::Jump, Category::Evade, u * 1.25, None);
                }
                // a shot over a ducking body's height: drop under it
                if grounded && y > body.duck + 3.0 && tti < 0.4 {
                    out.push(Choice::Duck { hold: tti + 0.12 }, Category::Defend, u * 1.2, None);
                }
                // on the sword: rise over it, or sink under it
                if me.qi > 12.0 && tti > 0.15 {
                    if y + 4.0 < arena.top {
                        out.push(Choice::Fly { height: y + 4.0 }, Category::Evade, u * 1.15, None);
                    }
                    if me.flying && y > body.body + 2.0 {
                        out.push(Choice::Fly { height: 0.0 }, Category::Evade, u * 1.1, None);
                    }
                }
                if let Some(b) = ctx.ready_of(SkillKind::Bolt)
                    && tti > 0.3
                {
                    out.cast(b, None, u * 0.9, None);
                }
                blink(out, x, u * 0.7);
            }
            Shape::Seeker { x, y, .. } => {
                let d = libm::hypot(x - me.x, y - (me.air + body.chest));
                if d > 48.0 {
                    continue;
                }
                let u = th.damage * (1.4 - d / 48.0);
                worst = worst.max(u);
                out.walk(sign(me.x - x), Category::Evade, u);
                if let Some(b) = ctx.ready_of(SkillKind::Bolt) {
                    out.cast(b, None, u * 1.15, None);
                }
                blink(out, x, u * 0.9);
            }
            Shape::Area { x, radius, reach, lands_in, grounded: low } => {
                let dx = me.x - x;
                if dx.abs() > reach + radius + body.half_width + 4.0 {
                    continue;
                }
                let u = th.damage * (1.5 - lands_in);
                worst = worst.max(u);
                let way = if dx.abs() < 1.0 { if x - arena.left > arena.right - x { -1.0 } else { 1.0 } } else { sign(dx) };
                out.walk(way, Category::Evade, u * 1.3);
                if low && me.qi > 12.0 {
                    out.push(Choice::Fly { height: 24.0 }, Category::Evade, u * 1.35, None);
                }
                blink(out, x, u * if lands_in < 0.3 { 1.4 } else { 0.6 });
            }
            Shape::Field { x, reach } => {
                let dx = me.x - x;
                if reach <= 0.0 || dx.abs() > reach {
                    continue;
                }
                let u = th.damage * (1.2 - dx.abs() / reach);
                worst = worst.max(u);
                out.walk(sign(dx), Category::Evade, u * 1.2);
                blink(out, x, u * 1.1);
            }
        }
    }
    worst
}

/// The foe winding up a blow that reaches me: step out, block, blink, or strike first. Returns its urgency.
fn blows(out: &mut Out) -> f64 {
    let ctx = out.ctx;
    let (me, foe, body) = (ctx.me, ctx.foe, ctx.body);
    let Doing::Act { kind: SkillKind::Melee | SkillKind::Blast | SkillKind::Dive, phase: Phase::Windup, t, windup, reach, damage, .. } =
        foe.doing
    else {
        return 0.0;
    };
    let d = (foe.x - me.x).abs();
    if d > reach + body.half_width + 6.0 {
        return 0.0;
    }
    let left = windup - t;
    let u = damage * 1.2;
    let away = sign(me.x - foe.x);
    // backing off works if I can walk out of reach before it lands
    if (reach + body.half_width + 2.0 - d) / body.walk.max(1.0) < left + 0.05 {
        out.walk(away, Category::Retreat, u * 1.2);
    }
    if sign(foe.x - me.x) == me.face {
        out.push(Choice::Guard { hold: left + 0.2 }, Category::Defend, u * 0.9, None);
    }
    if let Some(b) = ctx.ready_of(SkillKind::Melee)
        && ctx.skills[b].windup < left
        && d < ctx.skills[b].reach + body.half_width
    {
        out.cast(b, None, u * 1.1, None);
    }
    if let Some(b) = ctx.ready_of(SkillKind::Blink) {
        let to = me.x + away * ctx.skills[b].reach;
        if to > ctx.arena.left + 10.0 && to < ctx.arena.right - 10.0 {
            out.cast(b, Some(to), u * 0.8, Some(Category::Evade));
        }
    }
    u
}

/// Each attack I could start now, worth the damage I expect from it for what it costs me.
fn attacks(out: &mut Out) {
    let ctx = out.ctx;
    let (me, foe, body, r) = (ctx.me, ctx.foe, ctx.body, ctx.reading);
    let d = r.d;
    let busy = r.busy;
    let guard = ctx.guarding();
    let pace = match foe.doing {
        Doing::Move { dir } if foe.out_for == 0.0 => dir * body.walk,
        _ => 0.0,
    };
    for (i, s) in ctx.skills.iter().enumerate() {
        if !ctx.ready.get(i).copied().unwrap_or(false) {
            continue;
        }
        let hit = |quick: f64, guarded: f64, open: f64| if busy > s.windup { quick } else if guard { guarded } else { open };
        match s.kind {
            SkillKind::Dive if me.air - foe.air > 14.0 && d < 44.0 => out.cast(i, None, ctx.value(i, hit(0.9, 0.25, 0.6), s.windup), None),
            SkillKind::Melee if d < s.reach + 8.0 && (foe.air - me.air).abs() < body.chest => {
                out.cast(i, None, ctx.value(i, hit(0.95, 0.15, 0.55), s.windup), None);
            }
            SkillKind::Bolt if d > 24.0 && s.speed > 0.0 => {
                // a bolt flies level from my chest: it must cross the foe's body
                let chest = me.air + body.chest;
                if chest <= foe.air - 2.0 || chest >= foe.air + body.body {
                    continue;
                }
                let travel = d / s.speed + s.windup;
                // a bolt into a bolt the foe is gathering only cancels it
                let trade = matches!(foe.doing, Doing::Act { kind: SkillKind::Bolt, phase: Phase::Windup, .. });
                let chance = if trade { 0.1 } else if busy > travel { 0.9 } else if guard { 0.2 } else { 0.45 };
                out.cast(i, None, ctx.value(i, chance, travel), None);
            }
            // a slow ball that hunts the foe down: from far off it holds the space between
            SkillKind::Seeker if d > 40.0 && !me.seeker_out => out.cast(i, None, ctx.value(i, if d > 80.0 { 0.8 } else { 0.5 }, s.windup), None),
            SkillKind::Area => {
                // aim where the foe will be when it lands
                let lead = s.windup + s.lands;
                if lead < r.open_in {
                    continue;
                }
                let target = (foe.x + pace * lead * 0.8).clamp(ctx.arena.left + 6.0, ctx.arena.right - 6.0);
                let base = if s.ultimate {
                    ctx.value(i, 0.8, lead) * f64::from(s.strikes.max(1)) + 3.0
                } else {
                    // a blow that only finds a foe on the floor
                    let low = if s.grounded && (foe.air >= 10.0 || foe.flying) { 0.1 } else { 1.0 };
                    let chance = if busy > lead * 0.6 { 0.85 } else if s.inflicts.pull > 0.0 { 0.5 } else { 0.4 };
                    ctx.value(i, chance * low, lead)
                };
                out.cast(i, Some(target), base, None);
            }
            SkillKind::Blast if d < s.reach + 4.0 => out.cast(i, None, ctx.value(i, hit(0.9, 0.2, 0.55), s.windup), None),
            // a fire or a vortex under the foe: worth it on a foe that cannot leave it soon
            SkillKind::Field if busy > 0.4 || foe.frozen > 0.0 || foe.hp < 75.0 => {
                out.cast(i, Some(foe.x), ctx.value(i, 0.7, s.windup + s.lands) + 4.0, None);
            }
            // a rush through the foe: within a stride, on my level
            SkillKind::Dash if d > 12.0 && d < s.reach - 10.0 && (foe.air - me.air).abs() < body.chest => {
                out.cast(i, None, ctx.value(i, hit(0.9, 0.2, 0.6), s.windup), None);
            }
            SkillKind::Ward if !me.shield && me.hp < 90.0 => {
                let safe = if d > 50.0 && !matches!(foe.doing, Doing::Act { .. }) { 1.0 } else { 0.6 };
                out.cast(i, None, (8.0 + 14.0 * (1.0 - me.hp / 100.0)) * safe, None);
            }
            // a heal's `damage` is what it heals
            SkillKind::Heal if me.hp < 50.0 => {
                let safe = if d > 60.0 && !matches!(foe.doing, Doing::Act { .. }) { 1.0 } else if busy > 1.0 { 0.8 } else { 0.2 };
                out.cast(i, None, s.damage * safe * (1.0 - me.hp / 100.0), None);
            }
            _ => {}
        }
    }
}

/// Moving to the distance and height I fight best at, meditating when it is quiet, or waiting.
fn positions(out: &mut Out) {
    let ctx = out.ctx;
    let (me, foe, body, r, arena) = (ctx.me, ctx.foe, ctx.body, ctx.reading, ctx.arena);
    let (d, dx) = (r.d, r.dx);
    let blade = ctx.blade();
    let off = d - r.want;
    let toward = sign(dx);
    let level = (foe.air - me.air).abs() < body.chest;
    out.push(Choice::Wait, Category::Wait, 0.4, None);
    // after an exchange, or with the foe behind a barrier, back off to spacing and let it come (footsies)
    let spacing = (blade + body.half_width + 6.0).max(r.want * 0.8);
    if (ctx.calm > 0.0 || foe.barrier) && d < spacing && level && foe.out_for == 0.0 && ctx.room(-toward) > 14.0 {
        out.walk(-toward, Category::Retreat, 2.4);
    }
    // inside the foe's sword reach with no blade of my own ready: get out of it
    if d < blade + 4.0 && me.air - foe.air < body.chest && ctx.ready_of(SkillKind::Melee).is_none() && ctx.room(-toward) > 14.0 {
        out.walk(-toward, Category::Retreat, 2.2);
    }
    if off.abs() > 8.0 {
        let dir = toward * sign(off);
        // backed against a wall: no step back
        if ctx.room(dir) >= 12.0 {
            let category = if dir == toward { Category::Approach } else { Category::Retreat };
            out.walk(dir, category, 0.6 + off.abs().min(60.0) / 40.0);
        }
    }
    // cornered with the foe inside my range: blink past it and take the open side
    let wall = (me.x - arena.left).min(arena.right - me.x);
    if wall < 16.0
        && off < -10.0
        && let Some(b) = ctx.ready_of(SkillKind::Blink)
    {
        let behind = foe.x + toward * 30.0;
        if behind > arena.left + 10.0 && behind < arena.right - 10.0 {
            out.cast(b, Some(behind), 2.5, Some(Category::Evade));
        }
    }
    // the height I want: my own while I have the qi to cast from it, the foe's when I must close in
    let casts_from_afar = ctx.holds(|k| crate::reading::is_ranged(k) || crate::reading::is_area(k));
    let mut want = if me.qi > 40.0 && casts_from_afar { ctx.profile.sky } else { 0.0 };
    if me.qi < 25.0 || d < 40.0 {
        want = if foe.air > 16.0 && me.qi > 20.0 { arena.top.min(foe.air) } else { want };
    }
    let flying = if me.flying { me.hover } else { 0.0 };
    if (flying - want).abs() > 8.0 {
        let category = if want > flying && want != foe.air { Category::Retreat } else { Category::Approach };
        out.push(Choice::Fly { height: want }, category, 1.2, None);
    }
    // light on its feet: on the ground and out of reach, it bounds about rather than walking flat
    if me.air == 0.0 && !me.flying && d > blade + 10.0 && ctx.roll < 0.5 {
        out.push(Choice::Jump, Category::Approach, 1.3, None);
    }
    let quiet = ctx.threats.is_empty() && !matches!(foe.doing, Doing::Act { .. });
    if me.qi < 45.0 && d > 70.0 && quiet && me.air == 0.0 && !me.flying {
        out.push(Choice::Meditate, Category::Meditate, 2.2 - me.qi / 40.0, None);
    }
}
