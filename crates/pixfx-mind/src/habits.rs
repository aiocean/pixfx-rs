//! The foe's habits (n-grams, Game AI Pro ch. 48; Killer Instinct's Shadow AI): what it tends to do next after
//! what it just did, counted per posture it seems to be in. Counts fade with a half-life, so an old habit stops
//! counting, and a prediction's confidence grows with what has been seen (a wrong read is worse than none).

use crate::choice::Posture;
use crate::reading::{is_area, is_close, is_ranged};
use crate::sense::{Doing, Foe};
use crate::skill::SkillKind;

/// Something the foe started doing, as the habits count it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FoeMove {
    Approach,
    Retreat,
    Guard,
    Close,
    Ranged,
    Area,
    Jump,
    Fly,
    Meditate,
    Blink,
    Idle,
}

const MOVES: usize = 11;
/// Seconds for a count to halve.
const HALF_LIFE: f64 = 4.0;
/// Sightings a read needs before it is half trusted.
const PRIOR: f64 = 3.0;

impl FoeMove {
    const ALL: [FoeMove; MOVES] = [
        FoeMove::Approach,
        FoeMove::Retreat,
        FoeMove::Guard,
        FoeMove::Close,
        FoeMove::Ranged,
        FoeMove::Area,
        FoeMove::Jump,
        FoeMove::Fly,
        FoeMove::Meditate,
        FoeMove::Blink,
        FoeMove::Idle,
    ];

    fn index(self) -> usize {
        self as usize
    }
}

/// What the foe started doing between two looks at it, if it started something. `me_x` says which way is toward.
pub fn foe_move(before: &Foe, after: &Foe, me_x: f64) -> Option<FoeMove> {
    if before.air == 0.0 && after.air > 0.0 && !after.flying {
        return Some(FoeMove::Jump);
    }
    let started = match (&before.doing, &after.doing) {
        (Doing::Act { t: t0, kind: k0, .. }, Doing::Act { t: t1, kind: k1, .. }) => t1 < t0 || k0 != k1,
        (Doing::Move { dir: d0 }, Doing::Move { dir: d1 }) => d0 != d1,
        (a, b) => std::mem::discriminant(a) != std::mem::discriminant(b),
    };
    if !started {
        return None;
    }
    Some(match after.doing {
        Doing::Act { kind, .. } if is_close(kind) => FoeMove::Close,
        Doing::Act { kind, .. } if is_ranged(kind) => FoeMove::Ranged,
        Doing::Act { kind, .. } if is_area(kind) => FoeMove::Area,
        Doing::Act { kind: SkillKind::Blink, .. } => FoeMove::Blink,
        Doing::Act { .. } | Doing::Guard => FoeMove::Guard,
        Doing::Move { dir } if dir != 0.0 && dir.signum() == (me_x - after.x).signum() => FoeMove::Approach,
        Doing::Move { .. } => FoeMove::Retreat,
        Doing::Meditate => FoeMove::Meditate,
        Doing::Fly => FoeMove::Fly,
        Doing::Idle => FoeMove::Idle,
    })
}

/// A count that fades: its value at `time`.
#[derive(Clone, Copy, Debug, Default)]
struct Count {
    value: f64,
    time: f64,
}

impl Count {
    fn at(self, now: f64) -> f64 {
        self.value * libm::exp2(-(now - self.time).max(0.0) / HALF_LIFE)
    }

    fn add(&mut self, now: f64) {
        self.value = self.at(now) + 1.0;
        self.time = now;
    }
}

/// The foe's habits: n = 1 per posture it seems in, and n = 2.
#[derive(Clone, Debug)]
pub struct Habits {
    one: Vec<Count>,
    two: Vec<Count>,
    last: Option<FoeMove>,
    before: Option<FoeMove>,
}

impl Default for Habits {
    fn default() -> Self {
        Self::new()
    }
}

impl Habits {
    pub fn new() -> Habits {
        Habits {
            one: vec![Count::default(); Posture::ALL.len() * MOVES * MOVES],
            two: vec![Count::default(); MOVES * MOVES * MOVES],
            last: None,
            before: None,
        }
    }

    fn row_one(posture: Posture, last: FoeMove) -> usize {
        (posture as usize * MOVES + last.index()) * MOVES
    }

    fn row_two(before: FoeMove, last: FoeMove) -> usize {
        (before.index() * MOVES + last.index()) * MOVES
    }

    /// The foe started `m` while it seemed to be in `posture`.
    pub fn see(&mut self, posture: Posture, m: FoeMove, now: f64) {
        if let Some(last) = self.last {
            self.one[Habits::row_one(posture, last) + m.index()].add(now);
            if let Some(before) = self.before {
                self.two[Habits::row_two(before, last) + m.index()].add(now);
            }
        }
        self.before = self.last;
        self.last = Some(m);
    }

    /// What the foe will most likely start next, and how far to trust it (0 to 1).
    pub fn predict(&self, posture: Posture, now: f64) -> Option<(FoeMove, f64)> {
        let last = self.last?;
        let read = |row: &[Count]| -> Option<(FoeMove, f64)> {
            let counts: Vec<f64> = row.iter().map(|c| c.at(now)).collect();
            let seen: f64 = counts.iter().sum();
            let (best, most) = counts.iter().enumerate().fold((0, 0.0), |b, (i, c)| if *c > b.1 { (i, *c) } else { b });
            (seen > 1e-6).then(|| (FoeMove::ALL[best], most / seen * seen / (seen + PRIOR)))
        };
        let at = Habits::row_one(posture, last);
        let one = read(&self.one[at..at + MOVES]);
        let two = self.before.and_then(|before| {
            let at = Habits::row_two(before, last);
            read(&self.two[at..at + MOVES])
        });
        match (one, two) {
            (Some(a), Some(b)) => Some(if b.1 > a.1 { b } else { a }),
            (a, b) => a.or(b),
        }
    }

    pub fn clear(&mut self) {
        *self = Habits::new();
    }
}
