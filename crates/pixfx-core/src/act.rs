//! Attack timing: when an attack winds up, where it makes contact (and how hard the game should freeze, shake
//! and push), when it kicks up dust, and how long it takes to recover. Port of pixelfx `actTiming`
//! (`src/core/act.js`) without the pose track: the attacker is drawn by the game (pixfx-fighter's poses follow
//! the [`Phase`]s). Times are seconds of the effect's own playback, which starts at 0 at the wind-up's end.
//!
//! An effect describes its attack with [`crate::effect::Effect::act`]; a [`Performance`] plays it (pixelfx `createAct.update`).

/// The moment a blow lands.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Contact {
    /// When it lands.
    pub t: f64,
    /// How long time stands still, in seconds.
    pub freeze: f64,
    /// How long the picture shakes, in seconds.
    pub shake: f64,
    /// How hard the picture shakes, in px.
    pub amp: f64,
    /// How hard a target is pushed back; 0 when the blow does not move a target.
    pub push: f64,
    /// The index (0, 1, ...) of the only target it hits; every target when `None`.
    pub at: Option<f64>,
}

/// A puff of dust kicked up at the feet.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Puff {
    pub t: f64,
    /// Where, in px from the feet along the attack's direction.
    pub x: f64,
    /// How many specks.
    pub n: u32,
    /// Which way they fly: -1 or 1.
    pub dir: f64,
}

/// An attack's timing.
#[derive(Clone, Debug, PartialEq)]
pub struct Act {
    /// Seconds of wind-up before the effect's t = 0.
    pub windup: f64,
    /// Seconds of recovery after the effect ends (0 for a loop).
    pub recovery: f64,
    /// The effect is drawn on the attacker's own body.
    pub on_body: bool,
    /// How far in front of the attacker each target stands, in px (default 66).
    pub targets: Vec<f64>,
    pub contacts: Vec<Contact>,
    pub dust: Vec<Puff>,
}

/// Where an attack is in its course.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Windup,
    Strike,
    Recover,
    Done,
}

/// Something that happens as a performance plays; the index points into [`Act::contacts`] or [`Act::dust`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActEvent {
    Contact(usize),
    Dust(usize),
}

/// An act playing: a clock from the wind-up through the effect to the recovery, held still during a contact's
/// freeze.
#[derive(Clone, Debug)]
pub struct Performance {
    act: Act,
    duration: f64,
    t: f64,
    stop: f64,
    next_contact: usize,
    next_dust: usize,
}

impl Performance {
    /// A performance of `act` for an effect lasting `duration` seconds. Contacts and dust fire in time order
    /// whatever order the act lists them in.
    pub fn new(mut act: Act, duration: f64) -> Self {
        act.contacts.sort_by(|a, b| a.t.total_cmp(&b.t));
        act.dust.sort_by(|a, b| a.t.total_cmp(&b.t));
        Self {
            t: -act.windup,
            act,
            duration,
            stop: 0.0,
            next_contact: 0,
            next_dust: 0,
        }
    }

    /// Advance by `dt` seconds. A freeze consumes `dt` without moving the clock. Returns the events that
    /// fired, in time order (a contact before a dust puff at the same time).
    pub fn step(&mut self, dt: f64) -> Vec<ActEvent> {
        if self.stop > 0.0 {
            self.stop -= dt;
            return Vec::new();
        }
        self.t += dt;
        let mut fired: Vec<(f64, ActEvent)> = Vec::new();
        while let Some(c) = self.act.contacts.get(self.next_contact) {
            if c.t > self.t {
                break;
            }
            self.stop = self.stop.max(c.freeze);
            fired.push((c.t, ActEvent::Contact(self.next_contact)));
            self.next_contact += 1;
        }
        while let Some(d) = self.act.dust.get(self.next_dust) {
            if d.t > self.t {
                break;
            }
            fired.push((d.t, ActEvent::Dust(self.next_dust)));
            self.next_dust += 1;
        }
        // stable: a contact stays ahead of a puff at the same time
        fired.sort_by(|a, b| a.0.total_cmp(&b.0));
        fired.into_iter().map(|(_, e)| e).collect()
    }

    /// The clock: starts at `-windup`, 0 when the effect begins.
    pub fn t(&self) -> f64 {
        self.t
    }

    pub fn phase(&self) -> Phase {
        if self.t < 0.0 {
            Phase::Windup
        } else if self.t < self.duration {
            Phase::Strike
        } else if self.t < self.duration + self.act.recovery {
            Phase::Recover
        } else {
            Phase::Done
        }
    }

    /// Time is standing still for a contact's freeze.
    pub fn frozen(&self) -> bool {
        self.stop > 0.0
    }

    pub fn act(&self) -> &Act {
        &self.act
    }
}
