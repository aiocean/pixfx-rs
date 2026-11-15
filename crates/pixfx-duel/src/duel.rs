//! The duel: two fighters and their seats, stepped in whole ticks, round after round.

use crate::act::{advance, begin, break_act, drop_barrier, knock_out, set_meditating, settle};
use crate::bar::{draw, fight_pool};
use crate::data::{BAR, CHILL, DECOR, FLY, GRIT, MEDITATE_REGEN, QI_REGEN, ROUND_PAUSE, STALE, powi, school, spell};
use crate::fighter::{Fighter, Phase, Slot, State, Told};
use crate::out::Out;
use crate::seat::{Brain, MakeSeat, Seat};
use crate::shot::{shot, stop_shot};
use crate::status::{burning, shocking};
use pixfx_draw::math::{js_max, js_min};
use pixfx_fighter::body::{Drive, Motion};
use pixfx_mind::Choice;

/// One tick of the rules, in seconds: a seed is one fight at any frame rate.
pub const TICK: f64 = 1.0 / 60.0;
/// The most one step runs, in seconds; the rest is dropped, as after a stall.
pub const STEP_MAX: f64 = 0.25;

/// Where a duel is fought: the arena from `left` to `right` (game pixels) standing on `ground`, how high a fighter
/// may fly (`sky`, below whatever hangs over the arena), and the effects in the registry.
#[derive(Clone, Debug, PartialEq)]
pub struct DuelConfig {
    pub seed: u32,
    pub left: f64,
    pub right: f64,
    pub ground: f64,
    pub sky: f64,
    pub effects: Vec<String>,
}

/// A duel and the two seats that decide for its fighters.
pub struct Duel {
    pub state: State,
    seats: [Box<dyn Seat>; 2],
}

impl Duel {
    /// A new duel with each fighter's seat made by `seats` once both fighters exist, its first round begun.
    pub fn new(config: DuelConfig, seats: [MakeSeat; 2]) -> Duel {
        let top = js_min(FLY.top, config.sky);
        let blank = |i, s| Fighter::new(i, s, Vec::new(), Vec::new());
        let schools = [crate::data::School::Thunder, crate::data::School::Frost];
        let mut state = State::new(config.seed, config.left, config.right, config.ground, top, config.effects, schools.map(|s| blank(s as usize, s)));
        let effects = state.effects.clone();
        let usable = |name: &str| spell(name).is_some_and(|sp| sp.fx.is_none_or(|fx| effects.iter().any(|e| e == fx)));
        for (i, s) in schools.into_iter().enumerate() {
            let pool = fight_pool(&mut state, s, &|n| usable(n));
            let d = school(s);
            let spells = d.basics.iter().copied().chain(pool.iter().copied()).chain([d.ultimate]).filter(|n| usable(n)).collect();
            state.fighters[i] = Fighter::new(i, s, spells, pool);
        }
        let [make0, make1] = seats;
        let seats = [make0(&state, 0), make1(&state, 1)];
        state.decor = DECOR.iter().any(|d| state.has(d));
        let mut duel = Duel { state, seats };
        duel.start_round();
        duel
    }

    /// A duel of two brains.
    pub fn with_brains(config: DuelConfig) -> Duel {
        Duel::new(config, [Brain::make(), Brain::make()])
    }

    /// Fighter `me`'s seat.
    pub fn seat(&self, me: usize) -> &dyn Seat {
        self.seats[me].as_ref()
    }

    /// Fighter `me`'s seat thinks nothing for `seconds`.
    pub fn hold(&mut self, me: usize, seconds: f64) {
        self.seats[me].hold(seconds);
    }

    /// Advance by `dt` seconds (at most `STEP_MAX`; a step that is not a positive number moves nothing); returns the
    /// effect commands and events of this step. The rules run in whole ticks: what is left of `dt` waits for the next
    /// call.
    pub fn step(&mut self, dt: f64) -> Vec<Out> {
        self.state.out.clear();
        let dt = if dt > 0.0 { js_min(dt, STEP_MAX) } else { 0.0 };
        self.state.carry += dt;
        // a hair under a whole tick counts as one, so 1/60 steps at 60 Hz never drift a tick behind
        while self.state.carry >= TICK - 1e-9 {
            self.tick(TICK);
            self.state.carry = js_max(0.0, self.state.carry - TICK);
        }
        core::mem::take(&mut self.state.out)
    }

    /// Hand each seat what the rules told it, in order.
    fn flush(&mut self) {
        for (me, told) in core::mem::take(&mut self.state.told) {
            match told {
                Told::Outcome(o) => self.seats[me].tell(&o),
                Told::Reset => self.seats[me].reset(),
            }
        }
    }

    fn start_round(&mut self) {
        let state = &mut self.state;
        let span = state.right - state.left;
        state.round += 1;
        state.pause = 0.0;
        state.winner = None;
        for s in core::mem::take(&mut state.shots) {
            stop_shot(state, &s);
        }
        for f in 0..2 {
            // what the last round left standing goes with it
            break_act(state, f);
            drop_barrier(state, f);
            state.fighters[f].refresh();
            state.tell(f, Told::Reset);
            state.fighters[f].bar.clear();
            let at = if f == 0 { 0.22 } else { 0.78 };
            state.fighters[f].body.start_round(state.left + span * at);
            if let Some(key) = state.fighters[f].aura.take() {
                state.emit(Out::Stop { key });
            }
        }
        // the draws come once both fighters are fresh: nothing the last round left on a foe steers them
        for f in 0..2 {
            for i in 0..BAR.slots {
                draw(state, f, i, i == 0, None);
            }
        }
        self.flush();
    }

    fn tick(&mut self, dt: f64) {
        self.state.time += dt;
        if self.state.winner.is_some() {
            self.state.pause += dt;
            for f in 0..2 {
                settle(&mut self.state, f, dt);
            }
            if self.state.pause >= ROUND_PAUSE {
                self.start_round();
            }
            return;
        }
        for me in 0..2 {
            self.flush();
            if let Some(choice) = self.seats[me].think(&self.state, me, dt) {
                apply(&mut self.state, me, &choice);
            }
            body(&mut self.state, me, dt);
        }
        let [a, b] = &mut self.state.fighters;
        if a.hp > 0.0 && b.hp > 0.0 {
            let arena = pixfx_fighter::body::Arena { left: self.state.left, right: self.state.right, top: self.state.top };
            pixfx_fighter::body::separate(&mut a.body, &mut b.body, &arena);
        }
        // the shots in flight at the start of the tick run, but not one another shot ended this tick
        let ids: Vec<u32> = self.state.shots.iter().map(|s| s.id).collect();
        for id in ids {
            shot(&mut self.state, id, dt);
        }
        for f in 0..2 {
            if self.state.fighters[f].hp <= 0.0 && self.state.winner.is_none() {
                knock_out(&mut self.state, f);
            }
        }
        self.flush();
    }
}

/// One step of fighter `me`: its body's clocks, the rules' own timers, then its body moves as the rules drive it,
/// and its cast moves on.
fn body(state: &mut State, me: usize, dt: f64) {
    let foe = 1 - me;
    {
        let f = &mut state.fighters[me];
        f.body.tick(dt, f.hp > 0.0);
        f.sealed = js_max(0.0, f.sealed - dt);
        f.def_down = js_max(0.0, f.def_down - dt);
        f.silence = js_max(0.0, f.silence - dt);
        f.dodged = js_max(0.0, f.dodged - dt);
        f.grit = js_max(0.0, f.grit - GRIT.fall * dt);
        f.chill_t = js_max(0.0, f.chill_t - dt);
        if f.chill_t == 0.0 {
            f.chill = 0.0;
        }
    }
    shocking(state, me, dt);
    {
        let f = &mut state.fighters[me];
        f.shield_t = js_max(0.0, f.shield_t - dt);
        if f.shield_t == 0.0 {
            f.charges = 0.0;
        }
        f.rang = js_max(0.0, f.rang - dt);
    }
    if state.fighters[me].barrier > 0.0 {
        let f = &mut state.fighters[me];
        f.barrier = js_max(0.0, f.barrier - dt);
        if f.barrier == 0.0 {
            drop_barrier(state, me);
        }
    }
    burning(state, me, dt);
    {
        let f = &mut state.fighters[me];
        f.guard = js_max(0.0, f.guard - dt);
        for (_, left) in &mut f.cool {
            *left = js_max(0.0, *left - dt);
        }
    }
    for i in 0..state.fighters[me].bar.len() {
        let f = &mut state.fighters[me];
        let cooling = f.bar[i].spell().is_some_and(|n| f.cool(n) > 0.0);
        let redraw = match &mut f.bar[i] {
            Slot::Held { .. } if cooling => None,
            Slot::Held { spell, idle, .. } => {
                *idle += dt;
                (*idle >= STALE).then_some(Some(*spell))
            }
            Slot::Empty { left, .. } => {
                *left = js_max(0.0, *left - dt);
                (*left == 0.0).then_some(None)
            }
        };
        if let Some(gone) = redraw {
            draw(state, me, i, false, gone);
        }
    }
    let f = &mut state.fighters[me];
    let regen = if f.medit {
        MEDITATE_REGEN
    } else if f.fly {
        -FLY.cost
    } else {
        QI_REGEN
    };
    f.qi = js_min(100.0, f.qi + regen * dt);
    // out of qi, or knocked out of the sky: flight ends and it falls
    if f.fly && (f.qi <= 1.0 || f.body.reeling()) {
        f.fly = false;
    }
    // a dive drops straight at the foe's height
    let diving = f.act.is_some_and(|a| a.phase == Phase::Windup && spell(a.spell).is_some_and(|s| s.dive));
    let foe_air = state.fighters[foe].body.air;
    let foe_x = state.fighters[foe].body.x;
    let f = &state.fighters[me];
    let motion = if diving {
        Motion::Dive { speed: spell("dive").and_then(|s| s.speed).unwrap_or(0.0), floor: foe_air }
    } else if f.fly {
        Motion::Fly { hover: f.hover }
    } else {
        Motion::Gravity
    };
    let drive = Drive {
        alive: f.hp > 0.0,
        acting: f.act.is_some(),
        meditating: f.medit,
        guarding: f.guard > 0.0,
        chilled: false,
        intent: f.intent * powi(CHILL.slow, f.chill),
        motion,
        foe_x,
    };
    let arena = state.arena();
    let State { rng, fighters, .. } = state;
    let events = fighters[me].body.step(dt, &drive, &arena, &mut || rng.next());
    for e in events {
        state.emit(e.into());
    }
    advance(state, me, dt);
}

/// Fighter `me` does what its seat chose.
pub fn apply(state: &mut State, me: usize, choice: &Choice) {
    state.fighters[me].intent = 0.0;
    if !matches!(choice, Choice::Meditate) {
        set_meditating(state, me, false);
    }
    let f = &mut state.fighters[me];
    match choice {
        Choice::Move { dir } => f.intent = *dir,
        // crouch first, then spring
        Choice::Jump => {
            if f.body.air == 0.0 {
                f.body.squat = f.body.spec.jump_squat;
                f.body.leap = f.body.spec.jump_speed;
            }
        }
        Choice::Guard { hold } => f.guard = *hold,
        Choice::Duck { hold } => f.body.duck = *hold,
        Choice::Fly { height } => {
            f.fly = *height > 0.0;
            f.hover = *height;
            if f.fly && f.body.air == 0.0 {
                f.body.vy = f.body.spec.jump_speed * 0.6;
            }
        }
        Choice::Meditate => set_meditating(state, me, true),
        Choice::Cast { skill, target } => {
            if let Some(name) = spell(skill).map(|s| s.name) {
                begin(state, me, name, *target);
            }
        }
        Choice::Wait => {}
    }
}
