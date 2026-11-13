//! An attack's timing: the clock starts in the wind-up, a contact fires once and freezes time, phases follow
//! the bounds, dust puffs come in order.

use pixfx_core::act::{Act, ActEvent, Contact, Performance, Phase, Puff};

fn contact(t: f64, freeze: f64) -> Contact {
    Contact {
        t,
        freeze,
        shake: 0.0,
        amp: 0.0,
        push: 1.2,
        at: None,
    }
}

fn act(windup: f64, recovery: f64, contacts: Vec<Contact>, dust: Vec<Puff>) -> Act {
    Act {
        windup,
        recovery,
        on_body: false,
        targets: vec![66.0],
        contacts,
        dust,
    }
}

#[test]
fn starts_at_minus_windup() {
    let p = Performance::new(act(0.3, 0.2, vec![], vec![]), 0.5);
    assert_eq!(p.t(), -0.3);
    assert_eq!(p.phase(), Phase::Windup);
}

#[test]
fn contact_fires_once_at_t() {
    let mut p = Performance::new(act(0.0, 0.0, vec![contact(0.06, 0.0)], vec![]), 0.5);
    assert!(p.step(0.05).is_empty());
    assert_eq!(p.step(0.02), vec![ActEvent::Contact(0)]);
    assert!(p.step(0.1).is_empty());
}

#[test]
fn freeze_holds_time() {
    let mut p = Performance::new(act(0.0, 0.0, vec![contact(0.06, 0.14)], vec![]), 0.5);
    p.step(0.06);
    assert_eq!(p.t(), 0.06);
    assert!(p.frozen());
    p.step(0.1);
    assert_eq!(p.t(), 0.06);
    p.step(0.05);
    assert!(!p.frozen());
    p.step(0.05);
    assert!(p.t() > 0.06);
}

#[test]
fn phases_change_at_bounds() {
    let mut p = Performance::new(act(0.25, 0.25, vec![], vec![]), 0.5);
    assert_eq!(p.phase(), Phase::Windup);
    p.step(0.125);
    assert_eq!(p.phase(), Phase::Windup);
    p.step(0.125); // t = 0 exactly
    assert_eq!(p.phase(), Phase::Strike);
    p.step(0.375);
    assert_eq!(p.phase(), Phase::Strike);
    p.step(0.125); // t = duration exactly
    assert_eq!(p.phase(), Phase::Recover);
    p.step(0.125);
    assert_eq!(p.phase(), Phase::Recover);
    p.step(0.125); // t = duration + recovery exactly
    assert_eq!(p.phase(), Phase::Done);
}

#[test]
fn dust_events_in_order() {
    let puff = |t| Puff {
        t,
        x: 0.0,
        n: 6,
        dir: 1.0,
    };
    let mut p = Performance::new(
        act(
            0.0,
            0.0,
            vec![contact(0.1, 0.0)],
            vec![puff(0.05), puff(0.2)],
        ),
        0.5,
    );
    let ev = p.step(0.3);
    assert_eq!(
        ev,
        vec![ActEvent::Dust(0), ActEvent::Contact(0), ActEvent::Dust(1)]
    );
}
