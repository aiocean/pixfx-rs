//! Yomi (Sirlin, Playing to Win ch. 7): reading the foe at one of three levels. 0 answers what it does; 1 what
//! it is predicted to do; 2 its usual answer to my usual move. Which level to play is drawn from the seed, leaning
//! toward the level that paid off lately, so the mind is never the same read twice.

use crate::rng::Rng;

const START: [f64; 3] = [0.5, 0.35, 0.15];
const STEP: f64 = 0.1;
const FLOOR: f64 = 0.05;

#[derive(Clone, Debug, PartialEq)]
pub struct Yomi {
    weights: [f64; 3],
}

impl Default for Yomi {
    fn default() -> Self {
        Self::new()
    }
}

impl Yomi {
    pub fn new() -> Yomi {
        Yomi { weights: START }
    }

    /// A level to play, 0 to 2.
    pub fn draw(&self, rng: &mut Rng) -> u8 {
        let mut roll = rng.next() * self.weights.iter().sum::<f64>();
        for (level, w) in (0u8..).zip(self.weights) {
            if roll < w {
                return level;
            }
            roll -= w;
        }
        2
    }

    /// Playing `level` paid `payoff`: 1 a blow landed, -1 I was punished.
    pub fn paid(&mut self, level: u8, payoff: f64) {
        let Some(w) = self.weights.get_mut(usize::from(level)) else {
            return;
        };
        *w = (*w + STEP * payoff).max(FLOOR);
        let total: f64 = self.weights.iter().sum();
        for w in &mut self.weights {
            *w /= total;
        }
    }

    pub fn clear(&mut self) {
        *self = Yomi::new();
    }
}
