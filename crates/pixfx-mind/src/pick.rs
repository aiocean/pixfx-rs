//! What the mind has learnt of its own skills this round, and how it picks among candidates: rank, then weight
//! (Dill, Game AI Pro 2 ch. 3), with a small chance of the second best on purpose (Killer Instinct's Shadow AI),
//! so the best move is never a certainty to read.

use crate::candidates::Candidate;
use crate::rng::Rng;

/// Candidates within this share of the best stay in the draw.
const KEEP: f64 = 0.7;
/// The chance of taking the second best on purpose, if it is at least `SECOND` of the best.
const OFF_PICK: f64 = 0.08;
const SECOND: f64 = 0.5;
/// Seconds over which a punished skill is trusted again.
const SORE: f64 = 6.0;
/// How much a just-punished skill is worth less.
const PUNISHED: f64 = 0.5;

/// Per skill, this round: how often it was tried, how often it landed, when it was last punished.
#[derive(Clone, Debug, PartialEq)]
pub struct Read {
    tried: Vec<f64>,
    landed: Vec<f64>,
    punished: Vec<Option<f64>>,
}

impl Read {
    pub fn new(skills: usize) -> Read {
        Read {
            tried: vec![0.0; skills],
            landed: vec![0.0; skills],
            punished: vec![None; skills],
        }
    }

    pub fn tried(&mut self, i: usize) {
        if let Some(n) = self.tried.get_mut(i) {
            *n += 1.0;
        }
    }

    pub fn landed(&mut self, i: usize) {
        if let Some(n) = self.landed.get_mut(i) {
            *n += 1.0;
        }
    }

    pub fn punished(&mut self, i: usize, now: f64) {
        if let Some(t) = self.punished.get_mut(i) {
            *t = Some(now);
        }
    }

    /// The chance a skill lands: the moment's guess `base`, counted as `prior` tries, with what it has seen.
    pub fn chance(&self, i: usize, base: f64, prior: f64) -> f64 {
        let tried = self.tried.get(i).copied().unwrap_or(0.0);
        let landed = self.landed.get(i).copied().unwrap_or(0.0);
        (base * prior + landed) / (prior + tried).max(1e-9)
    }

    /// What a skill is worth after being punished for it: half just after, whole again `SORE` seconds later.
    pub fn penalty(&self, i: usize, now: f64) -> f64 {
        match self.punished.get(i).copied().flatten() {
            Some(t) => 1.0 - PUNISHED * (1.0 - (now - t) / SORE).clamp(0.0, 1.0),
            None => 1.0,
        }
    }

    /// The skill tried most this round.
    pub fn favourite(&self) -> Option<usize> {
        self.tried
            .iter()
            .enumerate()
            .filter(|(_, n)| **n > 0.0)
            .fold(None, |b: Option<(usize, f64)>, (i, n)| match b {
                Some((_, m)) if m >= *n => b,
                _ => Some((i, *n)),
            })
            .map(|(i, _)| i)
    }

    pub fn clear(&mut self) {
        *self = Read::new(self.tried.len());
    }
}

/// The chosen candidate's index, and every candidate's score, best first. `None` with nothing worth doing.
pub fn pick(
    cands: &[Candidate],
    weights: &[f64; 10],
    rng: &mut Rng,
) -> Option<(usize, Vec<(usize, f64)>)> {
    let mut ranked: Vec<(usize, f64)> = cands
        .iter()
        .enumerate()
        .map(|(i, c)| (i, c.base * weights[c.category as usize]))
        .filter(|(_, s)| s.is_finite() && *s > 0.0)
        .collect();
    ranked.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
    let (off, roll) = (rng.next(), rng.next());
    let best = ranked.first()?.1;
    if off < OFF_PICK
        && let Some(second) = ranked.get(1)
        && second.1 >= best * SECOND
    {
        return Some((second.0, ranked));
    }
    let kept: Vec<(usize, f64)> = ranked
        .iter()
        .copied()
        .take_while(|(_, s)| *s >= best * KEEP)
        .collect();
    let total: f64 = kept.iter().map(|(_, s)| s).sum();
    let mut at = roll * total;
    for (i, s) in &kept {
        if at < *s {
            return Some((*i, ranked));
        }
        at -= s;
    }
    Some((ranked[0].0, ranked))
}
