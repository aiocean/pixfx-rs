//! The first mind's first 600 choices of seeds 1-5, each with its tick and its fighter, pinned (`goldens/first_mind.json`).

mod common;

use common::{all_effects, config, pinned};
use pixfx_duel::duel::Duel;
use pixfx_duel::fighter::State;
use pixfx_duel::first_mind::FirstMind;
use pixfx_duel::seat::{MakeSeat, Seat};
use pixfx_mind::{Choice, Outcome};
use serde_json::{Value, json};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

type Log = Rc<RefCell<Vec<Value>>>;

/// A first mind that writes each choice it makes, with the tick it made it on.
struct Recorder {
    mind: FirstMind,
    log: Log,
    tick: Rc<Cell<usize>>,
}

impl Seat for Recorder {
    fn think(&mut self, state: &State, me: usize, dt: f64) -> Option<Choice> {
        let c = self.mind.think(state, me, dt);
        if let Some(c) = &c {
            self.log.borrow_mut().push(json!([self.tick.get(), me, as_js(c)]));
        }
        c
    }
    fn tell(&mut self, outcome: &Outcome) {
        self.mind.tell(outcome);
    }
    fn reset(&mut self) {
        self.mind.reset();
    }
    fn hold(&mut self, seconds: f64) {
        self.mind.hold(seconds);
    }
}

/// A choice in the shape the JS mind hands back.
fn as_js(c: &Choice) -> Value {
    match c {
        Choice::Move { dir } => json!({"type": "move", "dir": dir}),
        Choice::Jump => json!({"type": "jump"}),
        Choice::Guard { hold } => json!({"type": "guard", "hold": hold}),
        Choice::Duck { hold } => json!({"type": "duck", "hold": hold}),
        Choice::Fly { height } => json!({"type": "fly", "height": height}),
        Choice::Meditate => json!({"type": "meditate"}),
        Choice::Wait => json!({"type": "wait"}),
        Choice::Cast { skill, target } => json!({"type": "cast", "skill": skill, "target": target}),
    }
}

fn recorder(log: &Log, tick: &Rc<Cell<usize>>) -> MakeSeat {
    let (log, tick) = (log.clone(), tick.clone());
    Box::new(move |state, me| Box::new(Recorder { mind: FirstMind::new(state.seed, me), log, tick }))
}

#[test]
fn first_mind_choices_are_pinned() {
    let mut all = serde_json::Map::new();
    for seed in 1..=5_u32 {
        let log: Log = Rc::default();
        let tick = Rc::new(Cell::new(0));
        let mut duel = Duel::new(config(seed, all_effects()), [recorder(&log, &tick), recorder(&log, &tick)]);
        while log.borrow().len() < 600 && tick.get() < 600 * 60 {
            duel.step(1.0 / 60.0);
            tick.set(tick.get() + 1);
        }
        let got: Vec<Value> = log.borrow().iter().take(600).cloned().collect();
        assert_eq!(got.len(), 600, "seed {seed}");
        all.insert(seed.to_string(), Value::Array(got));
    }
    pinned("first_mind", &Value::Object(all));
}
