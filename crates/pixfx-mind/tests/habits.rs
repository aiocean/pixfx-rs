//! What the foe tends to do next (n-grams), and which level of reading pays (yomi).

mod common;
use common::*;
use pixfx_mind::*;

fn pattern(h: &mut Habits, times: usize, now: f64) {
    for _ in 0..times {
        h.see(Posture::Pressure, FoeMove::Approach, now);
        h.see(Posture::Pressure, FoeMove::Guard, now);
    }
    h.see(Posture::Pressure, FoeMove::Approach, now);
}

#[test]
fn a_repeated_pattern_is_predicted() {
    let mut h = Habits::new();
    pattern(&mut h, 5, 10.0);
    let Some((m, confidence)) = h.predict(Posture::Pressure, 10.0) else { unreachable!() };
    assert_eq!(m, FoeMove::Guard);
    assert!(confidence > 0.6, "{confidence}");
}

#[test]
fn a_thin_read_barely_counts() {
    let mut h = Habits::new();
    pattern(&mut h, 1, 10.0);
    let (_, confidence) = h.predict(Posture::Pressure, 10.0).unwrap_or((FoeMove::Idle, 0.0));
    assert!(confidence < 0.3, "{confidence}");
}

#[test]
fn old_habits_fade() {
    let mut h = Habits::new();
    pattern(&mut h, 5, 10.0);
    let (_, confidence) = h.predict(Posture::Pressure, 30.0).unwrap_or((FoeMove::Idle, 0.0));
    assert!(confidence < 0.2, "{confidence}");
}

#[test]
fn the_longer_pattern_wins_when_it_knows_more() {
    // after Approach the foe mostly guards (8 to 4); after Retreat then Approach it always blinks
    let mut h = Habits::new();
    for _ in 0..8 {
        h.see(Posture::Neutral, FoeMove::Idle, 1.0);
        h.see(Posture::Neutral, FoeMove::Approach, 1.0);
        h.see(Posture::Neutral, FoeMove::Guard, 1.0);
    }
    for _ in 0..4 {
        h.see(Posture::Neutral, FoeMove::Retreat, 1.0);
        h.see(Posture::Neutral, FoeMove::Approach, 1.0);
        h.see(Posture::Neutral, FoeMove::Blink, 1.0);
    }
    h.see(Posture::Neutral, FoeMove::Retreat, 1.0);
    h.see(Posture::Neutral, FoeMove::Approach, 1.0);
    assert_eq!(h.predict(Posture::Neutral, 1.0).map(|p| p.0), Some(FoeMove::Blink));
}

#[test]
fn clear_forgets() {
    let mut h = Habits::new();
    pattern(&mut h, 5, 10.0);
    h.clear();
    assert_eq!(h.predict(Posture::Pressure, 10.0), None);
}

#[test]
fn foe_moves_are_read_from_what_changed() {
    let a = foe(100.0);
    let mut b = foe(100.0);
    b.doing = Doing::Move { dir: -1.0 };
    assert_eq!(foe_move(&a, &b, 40.0), Some(FoeMove::Approach));
    assert_eq!(foe_move(&b, &b, 40.0), None);
    let mut c = foe(100.0);
    c.doing = blade_windup(0.0);
    assert_eq!(foe_move(&b, &c, 40.0), Some(FoeMove::Close));
    let mut later = c.clone();
    later.doing = blade_windup(0.1);
    assert_eq!(foe_move(&c, &later, 40.0), None);
    let mut d = foe(100.0);
    d.air = 4.0;
    assert_eq!(foe_move(&a, &d, 40.0), Some(FoeMove::Jump));
}

#[test]
fn yomi_leans_toward_what_paid() {
    let mut y = Yomi::new();
    for _ in 0..10 {
        y.paid(1, 1.0);
    }
    let mut rng = Rng::new(5);
    let mut counts = [0; 3];
    for _ in 0..1000 {
        counts[usize::from(y.draw(&mut rng))] += 1;
    }
    assert!(counts[1] > counts[0] && counts[1] > counts[2], "{counts:?}");
}

#[test]
fn yomi_never_drops_a_level() {
    let mut y = Yomi::new();
    for _ in 0..50 {
        y.paid(2, -1.0);
    }
    let mut rng = Rng::new(5);
    assert!((0..1000).any(|_| y.draw(&mut rng) == 2));
}
