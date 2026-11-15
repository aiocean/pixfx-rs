//! The seats a duel can hold besides the brain: the trained policy and the scripted ladder rungs.

mod common;

use common::{all_effects, config};
use pixfx_duel::bar::ready;
use pixfx_duel::duel::Duel;
use pixfx_duel::fighter::State;
use pixfx_duel::scripted::Scripted;
use pixfx_duel::seat::{MakeSeat, PolicySeat, Seat, shipped};
use pixfx_mind::{Choice, Outcome};
use std::cell::Cell;
use std::rc::Rc;

/// A seat that checks every choice of the seat inside it was possible when it was made.
struct Checked {
    inner: Box<dyn Seat>,
    casts: Rc<Cell<u32>>,
}

impl Seat for Checked {
    fn think(&mut self, state: &State, me: usize, dt: f64) -> Option<Choice> {
        let c = self.inner.think(state, me, dt);
        match &c {
            Some(Choice::Cast { skill, .. }) => {
                assert!(ready(&state.fighters[me], skill), "{skill} cast while not ready at {}", state.time);
                self.casts.set(self.casts.get() + 1);
            }
            Some(Choice::Move { dir }) => assert!(*dir == 1.0 || *dir == -1.0),
            _ => {}
        }
        c
    }
    fn tell(&mut self, outcome: &Outcome) {
        self.inner.tell(outcome);
    }
    fn reset(&mut self) {
        self.inner.reset();
    }
    fn hold(&mut self, seconds: f64) {
        self.inner.hold(seconds);
    }
}

fn checked(make: MakeSeat, casts: &Rc<Cell<u32>>) -> MakeSeat {
    let casts = casts.clone();
    Box::new(move |state, me| Box::new(Checked { inner: make(state, me), casts }))
}

#[test]
fn a_policy_seat_only_ever_makes_possible_choices() {
    let net = shipped().map_err(|e| e.to_string());
    assert!(net.is_ok(), "the shipped weights load: {net:?}");
    let Ok(net) = net else { return };
    let casts = Rc::new(Cell::new(0));
    for seed in 1..=4 {
        let seats = [checked(PolicySeat::make(net.clone()), &casts), checked(PolicySeat::make(net.clone()), &casts)];
        let mut duel = Duel::new(config(seed, all_effects()), seats);
        for _ in 0..(60 * 60) {
            duel.step(1.0 / 60.0);
        }
    }
    assert!(casts.get() > 20, "it casts: {}", casts.get());
}

#[test]
fn scripted_seats_finish_rounds_with_possible_choices() {
    let casts = Rc::new(Cell::new(0));
    for seed in 1..=5 {
        let mut duel = Duel::new(config(seed, all_effects()), [checked(Scripted::aggressive(), &casts), checked(Scripted::turtle(), &casts)]);
        for _ in 0..(120 * 60) {
            duel.step(1.0 / 60.0);
            if duel.state.round > 1 {
                break;
            }
        }
        assert!(duel.state.round > 1, "seed {seed}: a round ends within 120 s");
    }
    assert!(casts.get() > 20, "they cast: {}", casts.get());
}
