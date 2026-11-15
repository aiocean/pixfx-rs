//! The first mind makes the choices it was ported with (`firstMind` in the fixture): the same choices on the same ticks.

mod common;

use common::{all_effects, config, norm};
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
fn first_mind_matches_js_choices() {
    let f: Value = serde_json::from_str(include_str!("fixtures/js_numbers.json")).unwrap_or_default();
    for seed in 1..=5_u32 {
        let want = f["firstMind"][seed.to_string()].as_array().cloned().unwrap_or_default();
        assert_eq!(want.len(), 600, "seed {seed}");
        let log: Log = Rc::default();
        let tick = Rc::new(Cell::new(0));
        let mut duel = Duel::new(config(seed, all_effects()), [recorder(&log, &tick), recorder(&log, &tick)]);
        while log.borrow().len() < 600 && tick.get() < 600 * 60 {
            duel.step(1.0 / 60.0);
            tick.set(tick.get() + 1);
        }
        let got = log.borrow();
        for (i, (g, w)) in got.iter().zip(&want).enumerate() {
            assert_eq!(norm(g), norm(w), "seed {seed}, choice {i}");
        }
        assert!(got.len() >= 600, "seed {seed}: {} choices", got.len());
    }
}
