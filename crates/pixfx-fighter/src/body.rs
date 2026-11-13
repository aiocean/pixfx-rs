//! A fighter's body: where it stands, how it moves, and how it takes a blow the way fighting games do (hitstun,
//! launch, one juggle, knockdown, getting up, a quick roll, ducking, landing recovery). Port of the body half of the
//! site's `duel.js`; every float operation runs in the same order as there, so a duel stepped through this body
//! plays the same fight step for step.
//!
//! The rules of a fight (qi, moves, damage, the AI) stay with the game. Each step the game calls, in this order:
//! [`Body::tick`] (the body's clocks and getting up), its own timers, [`Body::step`] (feet and flight), its own
//! action clock, then `set_pose(pose_of(..))`. Between rounds it calls [`Body::settle`] instead.
//!
//! Units are game pixels and seconds. x grows to the right; `air` is the height of the feet above the ground.

use crate::pose::Pose;

/// A body's tuning: its size, how it moves, and how it takes a blow. `Default` is the site's duel. A body carries
/// its spec ([`Body::with_spec`]), so two fighters can be built differently; every rule reads these numbers.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "ts", derive(tsify::Tsify))]
#[cfg_attr(
    feature = "serde",
    serde(rename_all = "camelCase", deny_unknown_fields)
)]
pub struct BodySpec {
    /// The body's height, in pixels above its feet.
    pub body: f64,
    /// Where the chest is, in pixels above its feet: where blows and bumps are felt.
    pub chest: f64,
    /// Half the body's width: how close to an arena's edge its middle can come.
    pub half_width: f64,
    /// Room it keeps on each side beyond its half width: two bodies stand no closer than their half widths and
    /// rooms together.
    pub room: f64,
    /// Walking speed, pixels a second.
    pub walk: f64,
    /// Walking away from the foe is slower than walking toward it, so no one can run away forever.
    pub backwalk: f64,
    /// What a chill leaves of the walking speed.
    pub chill: f64,
    /// The speed a jump leaves the ground at.
    pub jump_speed: f64,
    pub gravity: f64,
    /// Near the top of an arc (rising or falling slower than `apex_speed`) gravity eases to `apex_gravity` of
    /// itself, so a jump hangs a moment before it falls.
    pub apex_speed: f64,
    pub apex_gravity: f64,
    /// Flying: how fast a fighter climbs or sinks toward the height it wants, and the fastest it does.
    pub fly_rate: f64,
    pub fly_max: f64,
    /// A blow this strong or stronger throws the target up into the air (`launch_speed` plus `launch_per_damage` a
    /// point of power); one of `launch_spin` or more sends it head over heels. A lighter blow that catches a fighter
    /// in the air only pops it up (`launch_pop`), and it lands on its feet.
    pub launch_damage: f64,
    pub launch_speed: f64,
    pub launch_per_damage: f64,
    pub launch_pop: f64,
    pub launch_spin: f64,
    /// A blow that keeps the target on its feet holds it reeling `hitstun_base` plus `hitstun_per` a point of power.
    pub hitstun_base: f64,
    pub hitstun_per: f64,
    /// A thrown fighter can be hit `juggle_limit` more times before it lands, flying `juggle_decay` as high each
    /// time.
    pub juggle_decay: f64,
    pub juggle_limit: u32,
    /// Thrown and landing: falling faster than `knockdown_bounce` it bounces once, keeping `knockdown_keep` of its
    /// speed; then it lies `knockdown_lie` and takes `knockdown_rise` to get up, or with chance `knockdown_quick`
    /// flops `knockdown_flop` and rolls back `knockdown_roll_distance` pixels over `knockdown_roll`. Nothing touches
    /// it from landing until `knockdown_grace` after it stands.
    pub knockdown_bounce: f64,
    pub knockdown_keep: f64,
    pub knockdown_lie: f64,
    pub knockdown_rise: f64,
    pub knockdown_quick: f64,
    pub knockdown_flop: f64,
    pub knockdown_roll: f64,
    pub knockdown_roll_distance: f64,
    pub knockdown_grace: f64,
    /// How long a jump crouches before it leaves the ground.
    pub jump_squat: f64,
    /// A landing crouches a moment, longer from a fall.
    pub landing_soft: f64,
    pub landing_hard: f64,
    /// Ducking, a fighter is this tall.
    pub duck: f64,
    /// A hit lights the target white this long.
    pub flash: f64,
    /// After a blow lands, blows pass through the body this long and it blinks (0: never, so combos and juggles
    /// land; 0.5 to 1 for a player's character, so one mistake costs one hit).
    pub invincible: f64,
    /// Knocked into the arena's edge faster than this, a fighter bounces off it.
    pub bounce: f64,
    /// Falling faster than this when it lands, a fighter's landing is felt.
    pub hard_landing: f64,
    /// How far a blow that keeps the target on its feet pushes it, as a speed; a freezing blow pushes
    /// `freeze_knockback` of that, a throwing one `launch_knockback` of it.
    pub knockback: f64,
    pub freeze_knockback: f64,
    pub launch_knockback: f64,
    /// Bounced off the arena's edge, a body keeps this share of its speed.
    pub wall_keep: f64,
    /// Falling faster than this when it lands on its feet, a fighter crouches a moment (`landing_soft`).
    pub soft_landing: f64,
    /// How fast sliding along the floor dies away, a second (`exp(-slide * dt)`), and on its back.
    pub slide: f64,
    pub slide_down: f64,
}

impl Default for BodySpec {
    /// The site's duel.
    fn default() -> Self {
        Self {
            body: 34.0,
            chest: 20.0,
            half_width: 6.0,
            room: 3.0,
            walk: 52.0,
            backwalk: 0.75,
            chill: 0.45,
            jump_speed: 220.0,
            gravity: 520.0,
            apex_speed: 45.0,
            apex_gravity: 0.45,
            fly_rate: 7.0,
            fly_max: 160.0,
            launch_damage: 12.0,
            launch_speed: 130.0,
            launch_per_damage: 5.0,
            launch_pop: 70.0,
            launch_spin: 20.0,
            hitstun_base: 0.24,
            hitstun_per: 0.012,
            juggle_decay: 0.7,
            juggle_limit: 1,
            knockdown_bounce: 150.0,
            knockdown_keep: 0.3,
            knockdown_lie: 0.5,
            knockdown_rise: 0.4,
            knockdown_quick: 0.4,
            knockdown_flop: 0.18,
            knockdown_roll: 0.36,
            knockdown_roll_distance: 26.0,
            knockdown_grace: 0.1,
            jump_squat: 4.0 / 60.0,
            landing_soft: 5.0 / 60.0,
            landing_hard: 0.14,
            duck: 16.0,
            flash: 5.0 / 60.0,
            invincible: 0.0,
            bounce: 80.0,
            hard_landing: 140.0,
            knockback: 130.0,
            freeze_knockback: 0.5,
            launch_knockback: 1.7,
            wall_keep: 0.5,
            soft_landing: 40.0,
            slide: 6.0,
            slide_down: 12.0,
        }
    }
}

/// The floor a fight happens on: its edges and the highest a fighter flies.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
#[cfg_attr(feature = "ts", derive(tsify::Tsify))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct Arena {
    pub left: f64,
    pub right: f64,
    pub top: f64,
}

/// Where a fighter off its feet is in going down and getting up.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "ts", derive(tsify::Tsify))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum Phase {
    /// Thrown, in the air.
    Air,
    /// Lying on the floor.
    Down,
    /// A quick rise, rolling away.
    Roll,
    /// Getting up: sit, kneel, stand.
    Rise,
}

/// A fighter off its feet.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "ts", derive(tsify::Tsify))]
pub struct Down {
    pub phase: Phase,
    /// Time in this phase.
    pub t: f64,
    /// How many times it was hit again while thrown.
    pub juggle: u32,
    /// Popped up by a light blow: it lands on its feet.
    pub light: bool,
    /// Head over heels.
    pub spin: bool,
    /// Bounced off the floor once already.
    pub bounced: bool,
    /// Getting up with a roll rather than sitting up.
    pub quick: bool,
    /// Which way it rolls: away from the foe (0 until it lands).
    pub dir: f64,
}

/// Something the body did that the screen should feel.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "ts", derive(tsify::Tsify))]
#[cfg_attr(feature = "serde", serde(tag = "type", rename_all = "lowercase"))]
#[cfg_attr(feature = "ts", tsify(rename = "BodyEvent"))]
pub enum Event {
    /// Feet or back met the floor at `x`, `strength` 0 to 1.
    Land { x: f64, strength: f64 },
    /// Thrown into the arena's edge at (`x`, `y` above the ground).
    Bump { x: f64, y: f64, strength: f64 },
}

/// How a body moves up and down this step, as the rules have it.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(from = "MotionWire"))]
#[cfg_attr(
    feature = "ts",
    derive(tsify::Tsify),
    tsify(type = "\"gravity\" | { fly: number } | { dive: number; floor: number }")
)]
pub enum Motion {
    /// Gravity's: standing, jumping, thrown.
    Gravity,
    /// Flying toward height `hover`.
    Fly { hover: f64 },
    /// Diving straight down at `speed`, no lower than `floor` (the foe's height).
    Dive { speed: f64, floor: f64 },
}

/// [`Motion`] as JS passes it: `"gravity"`, `{fly: hover}` or `{dive: speed, floor}`.
#[cfg(feature = "serde")]
#[derive(serde::Deserialize)]
#[serde(untagged)]
enum MotionWire {
    Named(MotionName),
    Fly(FlyWire),
    Dive(DiveWire),
}

#[cfg(feature = "serde")]
#[derive(serde::Deserialize)]
#[serde(rename_all = "lowercase")]
enum MotionName {
    Gravity,
}

#[cfg(feature = "serde")]
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct FlyWire {
    fly: f64,
}

#[cfg(feature = "serde")]
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct DiveWire {
    dive: f64,
    floor: f64,
}

#[cfg(feature = "serde")]
impl From<MotionWire> for Motion {
    fn from(w: MotionWire) -> Self {
        match w {
            MotionWire::Named(MotionName::Gravity) => Motion::Gravity,
            MotionWire::Fly(FlyWire { fly }) => Motion::Fly { hover: fly },
            MotionWire::Dive(DiveWire { dive, floor }) => Motion::Dive { speed: dive, floor },
        }
    }
}

/// What the rules have a fighter doing this step, for [`Body::step`].
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
#[cfg_attr(feature = "ts", derive(tsify::Tsify))]
#[cfg_attr(
    feature = "serde",
    serde(rename_all = "camelCase", deny_unknown_fields)
)]
pub struct Drive {
    /// Life above zero.
    pub alive: bool,
    /// In the middle of an action (winding up or recovering).
    pub acting: bool,
    pub meditating: bool,
    /// Holding a block.
    pub guarding: bool,
    pub chilled: bool,
    /// Which way it wants to walk, -1 to 1.
    pub intent: f64,
    pub motion: Motion,
    pub foe_x: f64,
}

/// What the rules have a fighter doing, for its pose, in their order: an action, then a guard, meditating, flying,
/// walking. The body's own state (thrown, down, stunned, ducking, landing, in the air) comes first where it should.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Doing {
    /// Winding up a blade blow (`melee`) or a spell.
    Windup {
        melee: bool,
    },
    /// Following through.
    Recover {
        melee: bool,
    },
    Guard,
    Meditate,
    Fly,
    Walk,
    Idle,
}

/// A blow that landed: its `power`, the way it pushes (`dir`, -1 or 1), whether it freezes rather than throws, and
/// whether the target was flying when it struck.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Blow {
    pub power: f64,
    pub dir: f64,
    pub freeze: bool,
    pub flying: bool,
}

impl Blow {
    /// Strong enough to throw a fighter built to `spec` off its feet (a freezing blow never does).
    pub fn heavy(&self, spec: &BodySpec) -> bool {
        self.power >= spec.launch_damage && !self.freeze
    }
}

/// A fighter's body. Plain fields: the game reads and writes them, and sets `frozen`, `shocked`, `squat` + `leap`
/// (a jump) and `duck` itself.
#[derive(Clone, Debug, PartialEq)]
pub struct Body {
    pub x: f64,
    /// Height of the feet above the ground.
    pub air: f64,
    pub vx: f64,
    pub vy: f64,
    /// 1 facing right, -1 facing left.
    pub face: f64,
    /// Hitstun left.
    pub stun: f64,
    /// Off its feet.
    pub down: Option<Down>,
    /// Grace left after getting up.
    pub wake: f64,
    /// Landing recovery left.
    pub land: f64,
    /// Crouch left before a jump leaves the ground, and the speed it leaves at.
    pub squat: f64,
    pub leap: f64,
    /// Ducking left.
    pub duck: f64,
    /// White flash left from the last hit.
    pub flash: f64,
    /// Invincibility left from the last hit ([`BodySpec::invincible`]).
    pub invincible: f64,
    /// Time since the last hit, and its power.
    pub hit_t: f64,
    pub hit_power: f64,
    pub frozen: f64,
    pub shocked: f64,
    pub pose: Pose,
    /// Time in this pose.
    pub pose_t: f64,
    /// How it is built: every rule it moves and reels by reads these numbers.
    pub spec: BodySpec,
}

impl Body {
    /// A body built as the site's duel builds one ([`BodySpec::default`]), standing at `x`, facing right, last hit
    /// long ago.
    pub fn new(x: f64) -> Self {
        Self::with_spec(x, BodySpec::default())
    }

    /// A body built to `spec`, standing at `x`, facing right, last hit long ago.
    pub fn with_spec(x: f64, spec: BodySpec) -> Self {
        Self {
            x,
            air: 0.0,
            vx: 0.0,
            vy: 0.0,
            face: 1.0,
            stun: 0.0,
            down: None,
            wake: 0.0,
            land: 0.0,
            squat: 0.0,
            leap: 0.0,
            duck: 0.0,
            flash: 0.0,
            invincible: 0.0,
            hit_t: 9.0,
            hit_power: 0.0,
            frozen: 0.0,
            shocked: 0.0,
            pose: Pose::Idle,
            pose_t: 0.0,
            spec,
        }
    }

    /// A new round: back on its feet at `x`, every reaction over. Facing, pose and build carry over.
    pub fn start_round(&mut self, x: f64) {
        *self = Self {
            x,
            face: self.face,
            pose: self.pose,
            pose_t: self.pose_t,
            ..Self::with_spec(x, self.spec)
        };
    }

    /// The body's clocks run on, and a fighter that is down lies, rolls or gets up. `alive`: life above zero.
    pub fn tick(&mut self, dt: f64, alive: bool) {
        self.flash = max(0.0, self.flash - dt);
        self.invincible = max(0.0, self.invincible - dt);
        self.pose_t += dt;
        self.hit_t += dt;
        self.stun = max(0.0, self.stun - dt);
        self.wake = max(0.0, self.wake - dt);
        self.duck = max(0.0, self.duck - dt);
        if self.down.is_some() {
            self.downed(dt, alive);
        }
        self.frozen = max(0.0, self.frozen - dt);
        self.shocked = max(0.0, self.shocked - dt);
    }

    /// Feet and flight for one step: turning to the foe, leaving the ground at the end of a jump's crouch, walking
    /// (only when free), knockback and its bounce off the arena's edge, then up and down and landing. `roll` gives
    /// numbers in [0, 1); it is called once at most, when a thrown fighter lands and may roll.
    pub fn step(
        &mut self,
        dt: f64,
        drive: &Drive,
        arena: &Arena,
        roll: &mut dyn FnMut() -> f64,
    ) -> Vec<Event> {
        let mut out = Vec::new();
        self.land = max(0.0, self.land - dt);
        if !drive.acting && !self.reeling() {
            self.face = if drive.foe_x >= self.x { 1.0 } else { -1.0 };
        }
        // the crouch before a jump ends: up it goes
        if self.squat > 0.0 {
            self.squat = max(0.0, self.squat - dt);
            if self.squat == 0.0 {
                self.vy = self.leap;
            }
        }

        let free = !drive.acting
            && !self.reeling()
            && !drive.meditating
            && self.frozen == 0.0
            && self.shocked == 0.0
            && self.squat == 0.0
            && self.duck == 0.0;
        let away = js_sign(drive.intent) == -js_sign(drive.foe_x - self.x);
        let walk = if free && !drive.guarding {
            drive.intent
                * self.spec.walk
                * (if drive.chilled { self.spec.chill } else { 1.0 })
                * (if away { self.spec.backwalk } else { 1.0 })
        } else {
            0.0
        };
        // a body sliding along the floor on its back stops sooner
        self.vx *= libm::exp(self.slide() * dt);
        let x = self.x + (walk + self.vx) * dt;
        let lo = arena.left + self.spec.half_width;
        let hi = arena.right - self.spec.half_width;
        if (x < lo || x > hi) && self.vx.abs() > self.spec.bounce {
            // thrown into the edge of the arena: bounce off it
            self.vx = -self.vx * self.spec.wall_keep;
            out.push(Event::Bump {
                x: if x < lo { lo } else { hi },
                y: self.air + self.spec.chest,
                strength: min(1.0, self.vx.abs() / 120.0),
            });
        }
        self.x = clamp(x, lo, hi);
        match drive.motion {
            Motion::Dive { speed, floor } => {
                self.air = max(floor, self.air - speed * dt);
                self.vy = -speed;
            }
            Motion::Fly { hover } => {
                self.vy = clamp(
                    (hover - self.air) * self.spec.fly_rate,
                    -self.spec.fly_max,
                    self.spec.fly_max,
                );
                self.air = clamp(self.air + self.vy * dt, 0.0, arena.top);
            }
            Motion::Gravity => {
                if self.air > 0.0 || self.vy > 0.0 {
                    let g = if self.vy.abs() < self.spec.apex_speed {
                        self.spec.gravity * self.spec.apex_gravity
                    } else {
                        self.spec.gravity
                    };
                    let falling = -self.vy;
                    self.vy -= g * dt;
                    self.air = max(0.0, self.air + self.vy * dt);
                    if self.air == 0.0 {
                        self.vy = 0.0;
                        if self.thrown() {
                            self.thrown_lands(
                                falling,
                                drive.alive,
                                drive.foe_x,
                                arena,
                                roll,
                                &mut out,
                            );
                        } else if falling > self.spec.hard_landing {
                            self.land = self.spec.landing_hard;
                            out.push(Event::Land {
                                x: self.x,
                                strength: min(1.0, falling / 300.0),
                            });
                        } else if falling > self.spec.soft_landing {
                            self.land = self.spec.landing_soft;
                        }
                    }
                }
            }
        }
        out
    }

    /// The body takes a blow that landed: it flashes, and a heavy blow (or any blow that catches it off the
    /// ground, unless it freezes) throws it; a lighter one staggers it where it stands and pushes it back. Inside the
    /// invincibility a blow before it left ([`BodySpec::invincible`]), nothing happens.
    pub fn hit(&mut self, blow: Blow) {
        if self.invincible > 0.0 {
            return;
        }
        self.invincible = self.spec.invincible;
        let heavy = blow.heavy(&self.spec);
        let thrown = heavy || (!blow.freeze && (self.air > 0.0 || blow.flying || self.thrown()));
        self.flash = self.spec.flash;
        self.hit_t = 0.0;
        self.hit_power = blow.power;
        self.squat = 0.0;
        self.duck = 0.0;
        if thrown {
            self.launch(blow.power, blow.dir, heavy);
        } else {
            self.stun = self.spec.hitstun_base + blow.power * self.spec.hitstun_per;
            self.vx += blow.dir
                * self.spec.knockback
                * (if blow.freeze {
                    self.spec.freeze_knockback
                } else {
                    1.0
                });
        }
    }

    /// Throw it off its feet: up into the air from a heavy blow, a little pop from a light one that caught it in
    /// the air. Each hit while it is still thrown (a juggle) throws it less high.
    pub fn launch(&mut self, power: f64, dir: f64, heavy: bool) {
        let juggle = match self.down {
            Some(d) if d.phase == Phase::Air => d.juggle + 1,
            _ => 0,
        };
        let k = libm::pow(self.spec.juggle_decay, f64::from(juggle));
        self.vx += dir
            * self.spec.knockback
            * (if heavy {
                self.spec.launch_knockback
            } else {
                1.0
            })
            * k;
        self.vy = max(
            self.vy,
            (if heavy {
                self.spec.launch_speed + power * self.spec.launch_per_damage
            } else {
                self.spec.launch_pop
            }) * k,
        );
        // a light pop lands on its feet; once thrown by a heavy blow it stays thrown
        let light = !heavy && !self.down.is_some_and(|d| !d.light);
        self.down = Some(Down {
            phase: Phase::Air,
            t: 0.0,
            juggle,
            light,
            spin: heavy && power >= self.spec.launch_spin,
            bounced: false,
            quick: false,
            dir: 0.0,
        });
        self.stun = 0.0;
    }

    /// The knockout throws the loser off its feet, if the blow has not already; one on the floor stays there.
    pub fn knock_out(&mut self, foe_x: f64) {
        match &mut self.down {
            None => self.launch(self.spec.launch_spin, sign_or_one(self.x - foe_x), true),
            Some(d) => {
                if d.phase != Phase::Air {
                    d.phase = Phase::Down;
                }
            }
        }
        self.stun = 0.0;
    }

    /// Between rounds: the body slides, falls and lies where it lands; the loser keeps its pose from the floor,
    /// the winner comes down and stands over it. `roll` as in [`Body::step`].
    pub fn settle(
        &mut self,
        dt: f64,
        won: bool,
        alive: bool,
        foe_x: f64,
        arena: &Arena,
        roll: &mut dyn FnMut() -> f64,
    ) -> Vec<Event> {
        let mut out = Vec::new();
        self.pose_t += dt;
        self.hit_t += dt;
        self.flash = max(0.0, self.flash - dt);
        self.invincible = max(0.0, self.invincible - dt);
        if let Some(d) = &mut self.down {
            d.t += dt;
        }
        self.vx *= libm::exp(self.slide() * dt);
        self.x = clamp(
            self.x + self.vx * dt,
            arena.left + self.spec.half_width,
            arena.right - self.spec.half_width,
        );
        if self.air > 0.0 || self.vy > 0.0 {
            let falling = -self.vy;
            self.vy -= self.spec.gravity * dt;
            self.air = max(0.0, self.air + self.vy * dt);
            if self.air == 0.0 {
                self.vy = 0.0;
                if self.thrown() {
                    self.thrown_lands(
                        falling + self.spec.gravity * dt,
                        alive,
                        foe_x,
                        arena,
                        roll,
                        &mut out,
                    );
                }
            }
        }
        // the loser is knocked out, so its pose comes from the body alone
        let pose = if !won {
            self.pose_of(Doing::Idle, alive)
        } else if self.air > 0.0 {
            Pose::Jump
        } else {
            Pose::Win
        };
        self.set_pose(pose);
        out
    }

    /// Reeling from a blow or off its feet: it cannot act.
    pub fn reeling(&self) -> bool {
        self.stun > 0.0 || self.down.is_some()
    }

    /// On the floor or getting up from it (or just up): nothing touches it.
    pub fn floored(&self) -> bool {
        self.down.is_some_and(|d| d.phase != Phase::Air) || self.wake > 0.0
    }

    /// Nothing touches it: on the floor, thrown and already juggled as often as a throw allows, or invincible after
    /// a blow.
    pub fn untouchable(&self) -> bool {
        self.invincible > 0.0
            || self.floored()
            || self
                .down
                .is_some_and(|d| d.phase == Phase::Air && d.juggle >= self.spec.juggle_limit)
    }

    /// How long until it can act again: its stagger, or the rest of its fall, lying and getting up.
    pub fn out_for(&self) -> f64 {
        let Some(d) = self.down else { return self.stun };
        let ground = if d.quick {
            self.spec.knockdown_flop + self.spec.knockdown_roll
        } else {
            self.spec.knockdown_lie + self.spec.knockdown_rise
        };
        match d.phase {
            Phase::Air => {
                (if d.light { 0.0 } else { ground })
                    + max(
                        0.1,
                        (self.vy
                            + libm::sqrt(max(
                                0.0,
                                self.vy * self.vy + 2.0 * self.spec.gravity * self.air,
                            )))
                            / self.spec.gravity,
                    )
            }
            Phase::Down => ground - d.t,
            Phase::Roll => self.spec.knockdown_roll - d.t,
            Phase::Rise => self.spec.knockdown_rise - d.t,
        }
    }

    /// How long until a blow can touch it: none while it is up or in the air, the rest of its time on the floor,
    /// and the rest of its invincibility.
    pub fn open_in(&self) -> f64 {
        if !self.untouchable() {
            return 0.0;
        }
        let floor = if self.down.is_some() {
            self.out_for() + self.spec.knockdown_grace
        } else {
            self.wake
        };
        max(floor, self.invincible)
    }

    /// The pose for this step: the body's own state first (thrown, down, getting up, knocked out, frozen, shocked,
    /// stunned, ducking, a jump's crouch), then what the rules have it `doing`, with landing and the air between.
    pub fn pose_of(&self, doing: Doing, alive: bool) -> Pose {
        if let Some(d) = self.down {
            return match d.phase {
                Phase::Air if d.spin && !d.bounced => Pose::Tumble,
                Phase::Air if self.vy > 0.0 && !d.bounced => Pose::Launched,
                Phase::Air => Pose::Fall,
                Phase::Down => Pose::Down,
                Phase::Roll => Pose::Roll,
                // getting up: sit up, kneel, then stand from a crouch
                Phase::Rise => {
                    let k = d.t / self.spec.knockdown_rise;
                    if k < 0.4 {
                        Pose::Sit
                    } else if k < 0.75 {
                        Pose::Kneel
                    } else {
                        Pose::Crouch
                    }
                }
            };
        }
        // struck down this very step: the knockout throws it next
        if !alive {
            return Pose::Hurt;
        }
        if self.frozen > 0.0 {
            return Pose::Frozen;
        }
        if self.shocked > 0.0 {
            return Pose::Shocked;
        }
        if self.stun > 0.0 {
            return if self.air > 4.0 {
                Pose::Fall
            } else {
                Pose::Hurt
            };
        }
        if self.duck > 0.0 {
            return Pose::Duck;
        }
        if self.squat > 0.0 {
            return Pose::Crouch;
        }
        match doing {
            Doing::Windup { melee: true } => Pose::Draw,
            Doing::Windup { melee: false } => Pose::Channel,
            Doing::Recover { melee: true } => Pose::Slash,
            Doing::Recover { melee: false } => Pose::Cast,
            Doing::Guard => Pose::Guard,
            Doing::Meditate => Pose::Meditate,
            Doing::Fly | Doing::Walk | Doing::Idle => {
                if self.land > 0.0 {
                    Pose::Land
                } else if doing == Doing::Fly {
                    Pose::Fly
                } else if self.air > 0.0 {
                    Pose::Jump
                } else if doing == Doing::Walk {
                    Pose::Walk
                } else {
                    Pose::Idle
                }
            }
        }
    }

    /// A new pose starts its own clock, so each one animates from its first frame.
    pub fn set_pose(&mut self, pose: Pose) {
        if pose != self.pose {
            self.pose_t = 0.0;
        }
        self.pose = pose;
    }

    fn thrown(&self) -> bool {
        self.down.is_some_and(|d| d.phase == Phase::Air)
    }

    /// How fast sliding dies away: sooner on its back.
    fn slide(&self) -> f64 {
        if self.down.is_some_and(|d| d.phase == Phase::Down) {
            -self.spec.slide_down
        } else {
            -self.spec.slide
        }
    }

    /// Thrown and meeting the floor falling at `falling`: it bounces once, lands on its feet from a pop, or goes
    /// down, maybe to roll away from the foe (unless a wall is behind it).
    fn thrown_lands(
        &mut self,
        falling: f64,
        alive: bool,
        foe_x: f64,
        arena: &Arena,
        roll: &mut dyn FnMut() -> f64,
        out: &mut Vec<Event>,
    ) {
        out.push(Event::Land {
            x: self.x,
            strength: min(1.0, falling / 260.0),
        });
        let x = self.x;
        let Some(d) = &mut self.down else { return };
        if !d.bounced && falling > self.spec.knockdown_bounce {
            d.bounced = true;
            self.vy = falling * self.spec.knockdown_keep;
            return;
        }
        if d.light && alive {
            self.down = None;
            self.land = self.spec.landing_hard;
            return;
        }
        let away = sign_or_one(x - foe_x);
        let room = if away < 0.0 {
            x - arena.left
        } else {
            arena.right - x
        };
        d.phase = Phase::Down;
        d.t = 0.0;
        d.quick = alive
            && room > self.spec.knockdown_roll_distance + 8.0
            && roll() < self.spec.knockdown_quick;
        d.dir = away;
    }

    /// Lying, then getting up: at the end it stands with a moment's grace. A knocked-out fighter stays down.
    fn downed(&mut self, dt: f64, alive: bool) {
        let Some(d) = &mut self.down else { return };
        d.t += dt;
        if d.phase == Phase::Down
            && alive
            && d.t
                >= (if d.quick {
                    self.spec.knockdown_flop
                } else {
                    self.spec.knockdown_lie
                })
        {
            d.phase = if d.quick { Phase::Roll } else { Phase::Rise };
            d.t = 0.0;
        } else if d.phase == Phase::Roll {
            let done = d.t >= self.spec.knockdown_roll;
            self.vx = (d.dir * self.spec.knockdown_roll_distance) / self.spec.knockdown_roll;
            if done {
                self.stand();
            }
        } else if d.phase == Phase::Rise && d.t >= self.spec.knockdown_rise {
            self.stand();
        }
    }

    fn stand(&mut self) {
        self.down = None;
        self.vx = 0.0;
        self.wake = self.spec.knockdown_grace;
        self.land = self.spec.landing_soft;
    }
}

/// Two bodies cannot stand in one place: overlapping at the same height each is pushed half the overlap apart,
/// and neither keeps walking into the other. One far enough above the other (all but the last 6 pixels of the lower
/// one's height) passes over it, and one on the floor is stepped over. Each keeps its own half width and room, and
/// its own distance from the arena's edges. The game calls it only while both fighters have life left.
pub fn separate(a: &mut Body, b: &mut Body, arena: &Arena) {
    let lower = if a.air <= b.air { &a.spec } else { &b.spec };
    if a.floored() || b.floored() || (a.air - b.air).abs() >= lower.body - 6.0 {
        return;
    }
    let personal = a.spec.half_width + a.spec.room + b.spec.half_width + b.spec.room;
    let dx = b.x - a.x;
    let overlap = personal - dx.abs();
    if overlap <= 0.0 {
        return;
    }
    // exactly on top of each other: part them along the way they face
    let dir = if dx != 0.0 { js_sign(dx) } else { a.face };
    let (a_lo, a_hi) = (
        arena.left + a.spec.half_width,
        arena.right - a.spec.half_width,
    );
    let (b_lo, b_hi) = (
        arena.left + b.spec.half_width,
        arena.right - b.spec.half_width,
    );
    a.x = clamp(a.x - (dir * overlap) / 2.0, a_lo, a_hi);
    b.x = clamp(b.x + (dir * overlap) / 2.0, b_lo, b_hi);
    // against a wall one side cannot give: the other takes the rest
    let left = personal - (b.x - a.x).abs();
    if left > 0.0 {
        if a.x == a_lo || a.x == a_hi {
            b.x = clamp(b.x + dir * left, b_lo, b_hi);
        } else {
            a.x = clamp(a.x - dir * left, a_lo, a_hi);
        }
    }
    if js_sign(a.vx) == dir {
        a.vx = 0.0;
    }
    if js_sign(b.vx) == -dir {
        b.vx = 0.0;
    }
}

/// JS `Math.sign`: NaN and signed zeros pass through.
fn js_sign(x: f64) -> f64 {
    if x > 0.0 {
        1.0
    } else if x < 0.0 {
        -1.0
    } else {
        x
    }
}

/// JS `Math.sign(x) || 1`: the sign, or 1 for zero and NaN.
fn sign_or_one(x: f64) -> f64 {
    let s = js_sign(x);
    if s == 0.0 || s.is_nan() { 1.0 } else { s }
}

/// JS `Math.max(a, b)`, signed zeros included (+0 beats -0).
fn max(a: f64, b: f64) -> f64 {
    if a == 0.0 && b == 0.0 {
        return if a.is_sign_positive() { a } else { b };
    }
    pixfx_draw::math::js_max(a, b)
}

/// JS `Math.min(a, b)`, signed zeros included (-0 beats +0).
fn min(a: f64, b: f64) -> f64 {
    if a == 0.0 && b == 0.0 {
        return if a.is_sign_negative() { a } else { b };
    }
    pixfx_draw::math::js_min(a, b)
}

/// The duel's `clamp`: `Math.max(lo, Math.min(hi, v))`.
fn clamp(v: f64, lo: f64, hi: f64) -> f64 {
    max(lo, min(hi, v))
}
