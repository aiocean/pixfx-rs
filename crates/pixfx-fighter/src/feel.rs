//! What makes a fight's blows land: the screen's answer to what just happened in the fight (port of the site's
//! `duel-feel.js`). A blow freezes the fight for a few frames (hit-stop) while the one it struck shudders, and
//! shakes the screen; its damage floats up from where it struck; a body hitting the floor kicks up dust; a knockout
//! lingers in slow motion; an ultimate darkens the sky as it is cast. All of it is state over the fight's [`Cue`]s
//! and real time, so the game asks it each frame how fast the fight runs and where the screen sits. The shake
//! weighs a blow by the share of full life it took (`share`); the numbers show the real amounts.

use pixfx_core::color::Rgba;
use pixfx_core::frame::Frame;
use pixfx_core::math::{js_max, js_min, js_round, to_int32};
use pixfx_core::screen::Ripple;

/// Hit-stop, in frames at 60 a second, the way fighting games and Sakurai's Smash count it: both fighters freeze
/// together, longer the heavier the blow (Street Fighter 6 gives its mediums 11 frames, its heavies 13); a blow of
/// `medium_power` or more is a medium; a crit holds longer, a spell that struck from afar shorter, and no blow
/// freezes the fight past a cap.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Stop {
    pub light: f64,
    pub medium: f64,
    pub launch: f64,
    pub crit: f64,
    pub ranged: f64,
    pub cap: f64,
    pub medium_power: f64,
}

/// How hard a cue that is not a blow answers: a hit-stop (seconds) and trauma added (0 to 1).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Answer {
    pub stop: f64,
    pub trauma: f64,
}

/// Dust a body raises where it hits the floor: how long it hangs, and the least landing strength (0 to 1) that
/// raises any.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DustSpec {
    pub life: f64,
    pub least: f64,
}

/// A knockout: a longer stop, then the fight runs at `speed` for `slow` seconds; it adds `trauma`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct KoSpec {
    pub stop: f64,
    pub speed: f64,
    pub slow: f64,
    pub trauma: f64,
}

/// Damage numbers rise this many pixels over their life.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NumberSpec {
    pub rise: f64,
    pub life: f64,
}

/// An ultimate stops the fight `stop` seconds and dims the sky `depth`, fading over `time` seconds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DuskSpec {
    pub stop: f64,
    pub depth: f64,
    pub time: f64,
}

/// How the screen answers a fight. `Default` is the site's duel.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FeelSpec {
    pub stop: Stop,
    /// The one struck shudders through the hit-stop: this many pixels at first, dying away by its end.
    pub jolt: f64,
    /// Trauma, 0 to 1, a blow adds: its share of full life over `share_trauma`, plus `crit_trauma` for a crit.
    pub share_trauma: f64,
    pub crit_trauma: f64,
    /// Trauma drains this fast per second; the shake is trauma squared times `shake` game pixels.
    pub trauma_drain: f64,
    pub shake: f64,
    /// A fighter refusing to fall, a barrier ringing, two spells meeting.
    pub unyielding: Answer,
    pub block: Answer,
    pub clash: Answer,
    /// Trauma a body hitting the floor or a wall adds at full strength.
    pub land_trauma: f64,
    pub bump_trauma: f64,
    pub dust: DustSpec,
    pub ko: KoSpec,
    pub number: NumberSpec,
    /// A streak of qi from a caster to where its spell lands shows this long.
    pub streak: f64,
    pub dusk: DuskSpec,
}

impl Default for FeelSpec {
    /// The site's duel.
    fn default() -> Self {
        Self {
            stop: Stop {
                light: 6.0,
                medium: 8.0,
                launch: 11.0,
                crit: 3.0,
                ranged: -2.0,
                cap: 13.0,
                medium_power: 9.0,
            },
            jolt: 2.0,
            share_trauma: 25.0,
            crit_trauma: 0.2,
            trauma_drain: 1.6,
            shake: 3.0,
            unyielding: Answer {
                stop: 0.18,
                trauma: 0.4,
            },
            block: Answer {
                stop: 0.05,
                trauma: 0.15,
            },
            clash: Answer {
                stop: 0.06,
                trauma: 0.25,
            },
            land_trauma: 0.15,
            bump_trauma: 0.15,
            dust: DustSpec {
                life: 0.4,
                least: 0.3,
            },
            ko: KoSpec {
                stop: 0.25,
                speed: 0.35,
                slow: 1.2,
                trauma: 0.8,
            },
            number: NumberSpec {
                rise: 14.0,
                life: 0.8,
            },
            streak: 0.28,
            dusk: DuskSpec {
                stop: 0.1,
                depth: 0.55,
                time: 0.9,
            },
        }
    }
}

/// Dust specks: the light and the dark one, alternating.
const DUST_LIGHT: Rgba = Rgba::hex24(0xa89a88);
const DUST_DARK: Rgba = Rgba::hex24(0x6e6458);

/// Something the fight just did that the screen answers. Heights (`y`) are up from the ground; `by` is the index
/// of the fighter who caused it.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
#[cfg_attr(feature = "ts", derive(tsify::Tsify))]
#[cfg_attr(feature = "serde", serde(tag = "type", rename_all = "lowercase"))]
pub enum Cue {
    /// A blow landed. `power` weighs it (9 and up is a medium), `launch` threw the one struck, `ranged` struck from
    /// afar (a bolt or an orb), `share` is the share of full life it took (0 to 100), `damage` the real amount.
    Hit {
        by: usize,
        power: f64,
        launch: bool,
        crit: bool,
        ranged: bool,
        share: f64,
        damage: f64,
        x: f64,
        y: f64,
    },
    /// Damage over time.
    Tick {
        by: usize,
        damage: f64,
        x: f64,
        y: f64,
    },
    Heal {
        by: usize,
        amount: f64,
        x: f64,
        y: f64,
    },
    Ward {
        by: usize,
        amount: f64,
        x: f64,
        y: f64,
    },
    /// A blow missed where it would have struck.
    Dodge {
        x: f64,
        y: f64,
    },
    /// A fighter refused to fall.
    Unyielding,
    /// A barrier rang.
    Block,
    /// Two spells met.
    Clash,
    /// A body hit the floor, `strength` 0 to 1.
    Land {
        x: f64,
        strength: f64,
    },
    /// A body hit a wall, `strength` 0 to 1.
    Bump {
        strength: f64,
    },
    Ko,
    /// Qi from a caster to where its spell lands.
    Streak {
        x0: f64,
        y0: f64,
        x1: f64,
        y1: f64,
        by: usize,
    },
    Ultimate,
}

impl Cue {
    /// The `type` of each cue as data, so a host that mixes cues with its own events can tell which are cues.
    pub const TYPES: [&str; 13] = [
        "hit",
        "tick",
        "heal",
        "ward",
        "dodge",
        "unyielding",
        "block",
        "clash",
        "land",
        "bump",
        "ko",
        "streak",
        "ultimate",
    ];
}

/// What a floating number says.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "ts", derive(tsify::Tsify))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum NumberKind {
    Hit,
    Crit,
    Tick,
    Heal,
    Ward,
    Dodge,
}

/// A floating number as drawn this frame: where it is (height above the ground), how faded (`alpha`), its text,
/// who caused it (none for a dodge) and its kind.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "ts", derive(tsify::Tsify))]
#[cfg_attr(feature = "ts", tsify(rename = "DamageNumber", missing_as_null))]
pub struct Number {
    pub x: f64,
    pub y: f64,
    pub alpha: f64,
    pub text: String,
    pub by: Option<usize>,
    pub kind: NumberKind,
}

/// A puff of dust: where, how strong, and how far through its life (`k`, 0 new, 1 gone).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Dust {
    pub x: f64,
    pub strength: f64,
    pub k: f64,
}

/// A qi streak: from where, to where (heights up from the ground), whose, and how faded (1 new, 0 gone).
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "ts", derive(tsify::Tsify))]
pub struct Streak {
    pub x0: f64,
    pub y0: f64,
    pub x1: f64,
    pub y1: f64,
    pub by: usize,
    pub fade: f64,
}

#[derive(Clone, Debug)]
struct Floating {
    x: f64,
    y: f64,
    text: String,
    by: Option<usize>,
    kind: NumberKind,
    t: f64,
}

#[derive(Clone, Copy, Debug)]
struct Qi {
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    by: usize,
    t: f64,
}

#[derive(Clone, Copy, Debug)]
struct Puff {
    x: f64,
    strength: f64,
    t: f64,
}

#[derive(Clone, Copy, Debug)]
struct Jolt {
    who: usize,
    left: f64,
    full: f64,
    air: bool,
}

/// The screen's answer to a fight: feed it [`Cue`]s with [`Feel::react`], let time pass with [`Feel::advance`],
/// and read where the screen sits and what floats over the fight.
#[derive(Clone, Debug, Default)]
pub struct Feel {
    spec: FeelSpec,
    stop: f64,
    slow: f64,
    trauma: f64,
    dusk: f64,
    numbers: Vec<Floating>,
    streaks: Vec<Qi>,
    dust: Vec<Puff>,
    jolt: Option<Jolt>,
    tick: u64,
    ripples: Vec<Ripple>,
    flash: f64,
}

fn hit_stop(stop: &Stop, power: f64, launch: bool, crit: bool, ranged: bool) -> f64 {
    let mut frames = if launch {
        stop.launch
    } else if power >= stop.medium_power {
        stop.medium
    } else {
        stop.light
    };
    if crit {
        frames += stop.crit;
    }
    if ranged {
        frames += stop.ranged;
    }
    js_min(stop.cap, frames) / 60.0
}

impl Feel {
    /// The site's duel's answer ([`FeelSpec::default`]).
    pub fn new() -> Self {
        Self::default()
    }

    /// Answering as `spec` has it.
    pub fn with_spec(spec: FeelSpec) -> Self {
        Self {
            spec,
            ..Self::default()
        }
    }

    /// How it answers.
    pub fn spec(&self) -> &FeelSpec {
        &self.spec
    }

    /// How shaken the screen is, 0 to 1.
    pub fn trauma(&self) -> f64 {
        self.trauma
    }

    /// The screen ripples alive now, for [`pixfx_core::screen::ripple`].
    pub fn ripples(&self) -> &[Ripple] {
        &self.ripples
    }

    /// How white the screen is lifted, 0 to 1, for [`pixfx_core::screen::flash`].
    pub fn flash(&self) -> f64 {
        self.flash
    }

    /// Take in what effects asked of the game: shake adds trauma, a hitstop joins the one freeze the fight's
    /// blows share (under the same cap), ripples and flash go to the screen passes. Sounds are the game's own.
    pub fn take(&mut self, cues: &[pixfx_core::cue::TimedCue]) {
        use pixfx_core::cue::Cue as Fx;
        for c in cues {
            match c.cue {
                Fx::Shake { trauma } => self.trauma = js_min(1.0, self.trauma + trauma),
                Fx::Hitstop { dur } => self.stop = js_max(self.stop, js_min(self.spec.stop.cap / 60.0, dur)),
                Fx::Ripple {
                    x,
                    y,
                    r0,
                    r1,
                    dur,
                    amp,
                } => self.ripples.push(Ripple {
                    x,
                    y,
                    r0,
                    r1,
                    dur,
                    amp,
                    age: 0.0,
                }),
                Fx::Flash { k, .. } => self.flash = js_max(self.flash, k),
                Fx::Sound { .. } => {}
            }
        }
    }

    /// Take in what the fight just did.
    pub fn react(&mut self, cues: &[Cue]) {
        let s = self.spec;
        for &c in cues {
            match c {
                Cue::Hit {
                    by,
                    power,
                    launch,
                    crit,
                    ranged,
                    share,
                    damage,
                    x,
                    y,
                } => {
                    let stop = hit_stop(&s.stop, power, launch, crit, ranged);
                    self.stop = js_max(self.stop, stop);
                    // grounded, it shudders side to side; in the air, up and down (Sakurai)
                    self.jolt = Some(Jolt {
                        who: 1usize.wrapping_sub(by),
                        left: stop,
                        full: stop,
                        air: y > 24.0,
                    });
                    self.trauma = js_min(
                        1.0,
                        self.trauma
                            + share / s.share_trauma
                            + if crit { s.crit_trauma } else { 0.0 },
                    );
                    let kind = if crit {
                        NumberKind::Crit
                    } else {
                        NumberKind::Hit
                    };
                    self.number(x, y, Some(by), kind, js_text(damage));
                }
                Cue::Tick { by, damage, x, y } => {
                    self.number(x, y, Some(by), NumberKind::Tick, js_text(damage))
                }
                Cue::Heal { by, amount, x, y } => {
                    self.number(
                        x,
                        y,
                        Some(by),
                        NumberKind::Heal,
                        format!("+{}", js_text(amount)),
                    );
                }
                Cue::Ward { by, amount, x, y } => {
                    self.number(
                        x,
                        y,
                        Some(by),
                        NumberKind::Ward,
                        format!("+{}", js_text(amount)),
                    );
                }
                // a dodge shows as a dash where the blow would have struck
                Cue::Dodge { x, y } => self.number(x, y, None, NumberKind::Dodge, "-".to_string()),
                Cue::Unyielding => self.answer(s.unyielding),
                // a barrier rings: a short stop, a little shake
                Cue::Block => self.answer(s.block),
                Cue::Clash => self.answer(s.clash),
                Cue::Land { x, strength } => {
                    self.trauma = js_min(1.0, self.trauma + s.land_trauma * strength);
                    if strength > s.dust.least {
                        self.dust.push(Puff {
                            x,
                            strength,
                            t: 0.0,
                        });
                    }
                }
                Cue::Bump { strength } => {
                    self.trauma = js_min(1.0, self.trauma + s.bump_trauma * strength)
                }
                Cue::Ko => {
                    self.stop = js_max(self.stop, s.ko.stop);
                    self.slow = s.ko.slow;
                    self.trauma = js_min(1.0, self.trauma + s.ko.trauma);
                }
                Cue::Streak { x0, y0, x1, y1, by } => self.streaks.push(Qi {
                    x0,
                    y0,
                    x1,
                    y1,
                    by,
                    t: 0.0,
                }),
                Cue::Ultimate => {
                    self.stop = js_max(self.stop, s.dusk.stop);
                    self.dusk = s.dusk.time;
                }
            }
        }
    }

    fn answer(&mut self, a: Answer) {
        self.stop = js_max(self.stop, a.stop);
        self.trauma = js_min(1.0, self.trauma + a.trauma);
    }

    fn number(&mut self, x: f64, y: f64, by: Option<usize>, kind: NumberKind, text: String) {
        // numbers from one spot step aside so a barrage stays readable
        let near = self
            .numbers
            .iter()
            .filter(|n| (n.x - x).abs() < 12.0 && n.t < 0.3)
            .count() as f64;
        let side = if near % 2.0 != 0.0 { 7.0 } else { -7.0 };
        self.numbers.push(Floating {
            x: x + side * js_min(near, 1.0),
            y: y + near * 6.0,
            text,
            by,
            kind,
            t: 0.0,
        });
    }

    /// Let `dt` real seconds pass. Returns how many seconds the fight should run for it: none during a hit-stop, a
    /// fraction in a knockout's slow motion.
    pub fn advance(&mut self, dt: f64) -> f64 {
        self.trauma = js_max(0.0, self.trauma - self.spec.trauma_drain * dt);
        self.dusk = js_max(0.0, self.dusk - dt);
        // the screen's own clock: ripples and flash run on through a hit-stop
        self.flash = js_max(0.0, self.flash - dt * 4.0);
        for r in &mut self.ripples {
            r.age += dt;
        }
        self.ripples.retain(|r| r.age <= r.dur);
        for n in &mut self.numbers {
            n.t += dt;
        }
        let s = self.spec;
        self.numbers.retain(|n| n.t < s.number.life);
        for k in &mut self.streaks {
            k.t += dt;
        }
        self.streaks.retain(|k| k.t < s.streak);
        for d in &mut self.dust {
            d.t += dt;
        }
        self.dust.retain(|d| d.t < s.dust.life);
        self.tick = self.tick.wrapping_add(1);
        if let Some(j) = &mut self.jolt {
            j.left -= dt;
            if j.left <= 0.0 {
                self.jolt = None;
            }
        }
        if self.stop > 0.0 {
            // counted in whole frames: what is left after the last one is rounding, not another frame
            self.stop = if self.stop - dt > 1e-6 {
                self.stop - dt
            } else {
                0.0
            };
            return 0.0;
        }
        if self.slow > 0.0 {
            self.slow = js_max(0.0, self.slow - dt);
            return dt * s.ko.speed;
        }
        dt
    }

    /// Where the screen sits this frame, in whole game pixels; `roll` gives numbers in [0, 1), x first.
    pub fn shake(&self, roll: &mut dyn FnMut() -> f64) -> (i32, i32) {
        let a = self.trauma * self.trauma * self.spec.shake;
        let x = to_int32(js_round((roll() * 2.0 - 1.0) * a));
        let y = to_int32(js_round((roll() * 2.0 - 1.0) * a));
        (x, y)
    }

    /// How far each fighter is jolted off its place this frame, in whole game pixels: (dx, dy) per fighter index.
    pub fn jolts(&self) -> [(i32, i32); 2] {
        let mut out = [(0, 0); 2];
        let Some(j) = self.jolt else {
            return out;
        };
        // flips side every frame, smaller as the stop runs out
        let a = js_round(self.spec.jolt * (j.left / j.full) + 0.4)
            * if self.tick.is_multiple_of(2) {
                -1.0
            } else {
                1.0
            };
        let a = to_int32(a);
        if let Some(slot) = out.get_mut(j.who) {
            *slot = if j.air { (0, a) } else { (a, 0) };
        }
        out
    }

    /// Each puff of dust.
    pub fn dust(&self) -> Vec<Dust> {
        self.dust
            .iter()
            .map(|d| Dust {
                x: d.x,
                strength: d.strength,
                k: d.t / self.spec.dust.life,
            })
            .collect()
    }

    /// How dark the sky is, 0 to the spec's `dusk.depth`.
    pub fn dusk(&self) -> f64 {
        let d = self.spec.dusk;
        let k = self.dusk / d.time;
        // in fast, out slow
        d.depth * js_min(1.0, k * 4.0) * js_min(1.0, k * 1.6)
    }

    /// Each floating number.
    pub fn numbers(&self) -> Vec<Number> {
        let spec = self.spec.number;
        self.numbers
            .iter()
            .map(|n| Number {
                x: n.x,
                y: n.y + 16.0 + spec.rise * libm::sqrt(n.t / spec.life),
                alpha: js_min(1.0, 2.5 * (1.0 - n.t / spec.life)),
                text: n.text.clone(),
                by: n.by,
                kind: n.kind,
            })
            .collect()
    }

    /// Each qi streak.
    pub fn streaks(&self) -> Vec<Streak> {
        self.streaks
            .iter()
            .map(|k| Streak {
                x0: k.x0,
                y0: k.y0,
                x1: k.x1,
                y1: k.y1,
                by: k.by,
                fade: 1.0 - k.t / self.spec.streak,
            })
            .collect()
    }

    /// Draw the dust into `frame`'s colour layer, `ground` the screen row of the floor: specks thrown out to both
    /// sides along the ground, rising a little and settling as they fade, each blended over what is there.
    pub fn draw_dust(&self, frame: &mut Frame, ground: i32) {
        let ground = f64::from(ground);
        for d in self.dust() {
            let n = 2.0 + js_round(3.0 * d.strength);
            let alpha = 0.8 * (1.0 - d.k);
            let mut i = 0.0;
            while i < n {
                let out = js_round((3.0 + i * 3.0) * libm::sqrt(d.k) * (0.6 + d.strength));
                let rise = js_round(
                    libm::sin(core::f64::consts::PI * d.k)
                        * (1.0 + (i * 7.0) % 3.0)
                        * d.strength
                        * 2.0,
                );
                let c = if i % 2.0 != 0.0 {
                    DUST_LIGHT
                } else {
                    DUST_DARK
                };
                let w = if i < 2.0 { 2.0 } else { 1.0 };
                for side in [-1.0, 1.0] {
                    let x = js_round(d.x + side * out);
                    let y = ground - 1.0 - rise;
                    pixfx_core::blend::fill(frame, x as i32, y as i32, w as i32, 1, c, alpha);
                }
                i += 1.0;
            }
        }
    }
}

/// A number as JS `String(n)` writes a whole amount (the amounts the fight shows are whole).
fn js_text(v: f64) -> String {
    if v == libm::trunc(v) && v.abs() < 1e15 {
        format!("{}", v as i64)
    } else {
        format!("{v}")
    }
}
