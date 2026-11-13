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

/// The body's height, in pixels above its feet.
pub const BODY: f64 = 34.0;
/// Where the chest is, in pixels above its feet: where blows and bumps are felt.
pub const CHEST: f64 = 20.0;
/// Half the body's width: how close to an arena's edge its middle can come.
pub const HALF_WIDTH: f64 = 6.0;
/// Walking speed, pixels a second.
pub const WALK: f64 = 52.0;
/// Walking away from the foe is slower than walking toward it, so no one can run away forever.
pub const BACKWALK: f64 = 0.75;
/// What a chill leaves of the walking speed.
pub const CHILL: f64 = 0.45;
pub const JUMP_SPEED: f64 = 220.0;
pub const GRAVITY: f64 = 520.0;
/// Near the top of an arc (rising or falling slower than this) gravity eases to `APEX_GRAVITY` of itself, so a
/// jump hangs a moment before it falls.
pub const APEX_SPEED: f64 = 45.0;
pub const APEX_GRAVITY: f64 = 0.45;
/// Flying: how fast a fighter climbs or sinks toward the height it wants, and the fastest it does.
pub const FLY_RATE: f64 = 7.0;
pub const FLY_MAX: f64 = 160.0;
/// A blow this strong or stronger throws the target up into the air (`LAUNCH_SPEED` plus `LAUNCH_PER_DAMAGE` a
/// point of power); one of `LAUNCH_SPIN` or more sends it head over heels. A lighter blow that catches a fighter in
/// the air only pops it up (`LAUNCH_POP`), and it lands on its feet.
pub const LAUNCH_DAMAGE: f64 = 12.0;
pub const LAUNCH_SPEED: f64 = 130.0;
pub const LAUNCH_PER_DAMAGE: f64 = 5.0;
pub const LAUNCH_POP: f64 = 70.0;
pub const LAUNCH_SPIN: f64 = 20.0;
/// A blow that keeps the target on its feet holds it reeling `HITSTUN_BASE` plus `HITSTUN_PER` a point of power.
pub const HITSTUN_BASE: f64 = 0.24;
pub const HITSTUN_PER: f64 = 0.012;
/// A thrown fighter can be hit `JUGGLE_LIMIT` more times before it lands, flying `JUGGLE_DECAY` as high each time.
pub const JUGGLE_DECAY: f64 = 0.7;
pub const JUGGLE_LIMIT: u32 = 1;
/// Thrown and landing: falling faster than `KNOCKDOWN_BOUNCE` it bounces once, keeping `KNOCKDOWN_KEEP` of its
/// speed; then it lies `KNOCKDOWN_LIE` and takes `KNOCKDOWN_RISE` to get up, or with chance `KNOCKDOWN_QUICK` flops
/// `KNOCKDOWN_FLOP` and rolls back `KNOCKDOWN_ROLL_DISTANCE` pixels over `KNOCKDOWN_ROLL`. Nothing touches it from
/// landing until `KNOCKDOWN_GRACE` after it stands.
pub const KNOCKDOWN_BOUNCE: f64 = 150.0;
pub const KNOCKDOWN_KEEP: f64 = 0.3;
pub const KNOCKDOWN_LIE: f64 = 0.5;
pub const KNOCKDOWN_RISE: f64 = 0.4;
pub const KNOCKDOWN_QUICK: f64 = 0.4;
pub const KNOCKDOWN_FLOP: f64 = 0.18;
pub const KNOCKDOWN_ROLL: f64 = 0.36;
pub const KNOCKDOWN_ROLL_DISTANCE: f64 = 26.0;
pub const KNOCKDOWN_GRACE: f64 = 0.1;
/// A jump crouches four frames before it leaves the ground.
pub const JUMP_SQUAT: f64 = 4.0 / 60.0;
/// A landing crouches a moment, longer from a fall.
pub const LANDING_SOFT: f64 = 5.0 / 60.0;
pub const LANDING_HARD: f64 = 0.14;
/// Ducking, a fighter is this tall.
pub const DUCK: f64 = 16.0;
/// A hit lights the target white this long.
pub const FLASH: f64 = 5.0 / 60.0;
/// Knocked into the arena's edge faster than this, a fighter bounces off it.
pub const BOUNCE: f64 = 80.0;
/// Falling faster than this when it lands, a fighter's landing is felt.
pub const HARD_LANDING: f64 = 140.0;
/// How far a blow that keeps the target on its feet pushes it, as a speed.
pub const KNOCKBACK: f64 = 130.0;
/// How close two bodies can stand: the space each takes up, and a little room between.
pub const PERSONAL: f64 = 2.0 * HALF_WIDTH + 6.0;

/// The floor a fight happens on: its edges and the highest a fighter flies.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Arena {
    pub left: f64,
    pub right: f64,
    pub top: f64,
}

/// Where a fighter off its feet is in going down and getting up.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
pub enum Event {
    /// Feet or back met the floor at `x`, `strength` 0 to 1.
    Land { x: f64, strength: f64 },
    /// Thrown into the arena's edge at (`x`, `y` above the ground).
    Bump { x: f64, y: f64, strength: f64 },
}

/// How a body moves up and down this step, as the rules have it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Motion {
    /// Gravity's: standing, jumping, thrown.
    Gravity,
    /// Flying toward height `hover`.
    Fly { hover: f64 },
    /// Diving straight down at `speed`, no lower than `floor` (the foe's height).
    Dive { speed: f64, floor: f64 },
}

/// What the rules have a fighter doing this step, for [`Body::step`].
#[derive(Clone, Copy, Debug, PartialEq)]
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
    /// Strong enough to throw a fighter off its feet (a freezing blow never does).
    pub fn heavy(&self) -> bool {
        self.power >= LAUNCH_DAMAGE && !self.freeze
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
    /// Time since the last hit, and its power.
    pub hit_t: f64,
    pub hit_power: f64,
    pub frozen: f64,
    pub shocked: f64,
    pub pose: Pose,
    /// Time in this pose.
    pub pose_t: f64,
}

impl Body {
    /// A body standing at `x`, facing right, last hit long ago.
    pub fn new(x: f64) -> Self {
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
            hit_t: 9.0,
            hit_power: 0.0,
            frozen: 0.0,
            shocked: 0.0,
            pose: Pose::Idle,
            pose_t: 0.0,
        }
    }

    /// A new round: back on its feet at `x`, every reaction over. Facing and pose carry over.
    pub fn start_round(&mut self, x: f64) {
        *self = Self {
            x,
            face: self.face,
            pose: self.pose,
            pose_t: self.pose_t,
            ..Self::new(x)
        };
    }

    /// The body's clocks run on, and a fighter that is down lies, rolls or gets up. `alive`: life above zero.
    pub fn tick(&mut self, dt: f64, alive: bool) {
        self.flash = max(0.0, self.flash - dt);
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
                * WALK
                * (if drive.chilled { CHILL } else { 1.0 })
                * (if away { BACKWALK } else { 1.0 })
        } else {
            0.0
        };
        // a body sliding along the floor on its back stops sooner
        self.vx *= libm::exp(self.slide() * dt);
        let x = self.x + (walk + self.vx) * dt;
        let lo = arena.left + HALF_WIDTH;
        let hi = arena.right - HALF_WIDTH;
        if (x < lo || x > hi) && self.vx.abs() > BOUNCE {
            // thrown into the edge of the arena: bounce off it
            self.vx = -self.vx * 0.5;
            out.push(Event::Bump {
                x: if x < lo { lo } else { hi },
                y: self.air + CHEST,
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
                self.vy = clamp((hover - self.air) * FLY_RATE, -FLY_MAX, FLY_MAX);
                self.air = clamp(self.air + self.vy * dt, 0.0, arena.top);
            }
            Motion::Gravity => {
                if self.air > 0.0 || self.vy > 0.0 {
                    let g = if self.vy.abs() < APEX_SPEED {
                        GRAVITY * APEX_GRAVITY
                    } else {
                        GRAVITY
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
                        } else if falling > HARD_LANDING {
                            self.land = LANDING_HARD;
                            out.push(Event::Land {
                                x: self.x,
                                strength: min(1.0, falling / 300.0),
                            });
                        } else if falling > 40.0 {
                            self.land = LANDING_SOFT;
                        }
                    }
                }
            }
        }
        out
    }

    /// The body takes a blow that landed: it flashes, and a heavy blow (or any blow that catches it off the
    /// ground, unless it freezes) throws it; a lighter one staggers it where it stands and pushes it back.
    pub fn hit(&mut self, blow: Blow) {
        let heavy = blow.heavy();
        let thrown = heavy || (!blow.freeze && (self.air > 0.0 || blow.flying || self.thrown()));
        self.flash = FLASH;
        self.hit_t = 0.0;
        self.hit_power = blow.power;
        self.squat = 0.0;
        self.duck = 0.0;
        if thrown {
            self.launch(blow.power, blow.dir, heavy);
        } else {
            self.stun = HITSTUN_BASE + blow.power * HITSTUN_PER;
            self.vx += blow.dir * KNOCKBACK * (if blow.freeze { 0.5 } else { 1.0 });
        }
    }

    /// Throw it off its feet: up into the air from a heavy blow, a little pop from a light one that caught it in
    /// the air. Each hit while it is still thrown (a juggle) throws it less high.
    pub fn launch(&mut self, power: f64, dir: f64, heavy: bool) {
        let juggle = match self.down {
            Some(d) if d.phase == Phase::Air => d.juggle + 1,
            _ => 0,
        };
        let k = libm::pow(JUGGLE_DECAY, f64::from(juggle));
        self.vx += dir * KNOCKBACK * (if heavy { 1.7 } else { 1.0 }) * k;
        self.vy = max(
            self.vy,
            (if heavy {
                LAUNCH_SPEED + power * LAUNCH_PER_DAMAGE
            } else {
                LAUNCH_POP
            }) * k,
        );
        // a light pop lands on its feet; once thrown by a heavy blow it stays thrown
        let light = !heavy && !self.down.is_some_and(|d| !d.light);
        self.down = Some(Down {
            phase: Phase::Air,
            t: 0.0,
            juggle,
            light,
            spin: heavy && power >= LAUNCH_SPIN,
            bounced: false,
            quick: false,
            dir: 0.0,
        });
        self.stun = 0.0;
    }

    /// The knockout throws the loser off its feet, if the blow has not already; one on the floor stays there.
    pub fn knock_out(&mut self, foe_x: f64) {
        match &mut self.down {
            None => self.launch(LAUNCH_SPIN, sign_or_one(self.x - foe_x), true),
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
        if let Some(d) = &mut self.down {
            d.t += dt;
        }
        self.vx *= libm::exp(self.slide() * dt);
        self.x = clamp(
            self.x + self.vx * dt,
            arena.left + HALF_WIDTH,
            arena.right - HALF_WIDTH,
        );
        if self.air > 0.0 || self.vy > 0.0 {
            let falling = -self.vy;
            self.vy -= GRAVITY * dt;
            self.air = max(0.0, self.air + self.vy * dt);
            if self.air == 0.0 {
                self.vy = 0.0;
                if self.thrown() {
                    self.thrown_lands(falling + GRAVITY * dt, alive, foe_x, arena, roll, &mut out);
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

    /// Nothing touches it: on the floor, or thrown and already juggled as often as a throw allows.
    pub fn untouchable(&self) -> bool {
        self.floored()
            || self
                .down
                .is_some_and(|d| d.phase == Phase::Air && d.juggle >= JUGGLE_LIMIT)
    }

    /// How long until it can act again: its stagger, or the rest of its fall, lying and getting up.
    pub fn out_for(&self) -> f64 {
        let Some(d) = self.down else { return self.stun };
        let ground = if d.quick {
            KNOCKDOWN_FLOP + KNOCKDOWN_ROLL
        } else {
            KNOCKDOWN_LIE + KNOCKDOWN_RISE
        };
        match d.phase {
            Phase::Air => {
                (if d.light { 0.0 } else { ground })
                    + max(
                        0.1,
                        (self.vy
                            + libm::sqrt(max(0.0, self.vy * self.vy + 2.0 * GRAVITY * self.air)))
                            / GRAVITY,
                    )
            }
            Phase::Down => ground - d.t,
            Phase::Roll => KNOCKDOWN_ROLL - d.t,
            Phase::Rise => KNOCKDOWN_RISE - d.t,
        }
    }

    /// How long until a blow can touch it: none while it is up or in the air, the rest of its time on the floor.
    pub fn open_in(&self) -> f64 {
        if !self.untouchable() {
            return 0.0;
        }
        if self.down.is_some() {
            self.out_for() + KNOCKDOWN_GRACE
        } else {
            self.wake
        }
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
                    let k = d.t / KNOCKDOWN_RISE;
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
            -12.0
        } else {
            -6.0
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
        if !d.bounced && falling > KNOCKDOWN_BOUNCE {
            d.bounced = true;
            self.vy = falling * KNOCKDOWN_KEEP;
            return;
        }
        if d.light && alive {
            self.down = None;
            self.land = LANDING_HARD;
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
        d.quick = alive && room > KNOCKDOWN_ROLL_DISTANCE + 8.0 && roll() < KNOCKDOWN_QUICK;
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
                    KNOCKDOWN_FLOP
                } else {
                    KNOCKDOWN_LIE
                })
        {
            d.phase = if d.quick { Phase::Roll } else { Phase::Rise };
            d.t = 0.0;
        } else if d.phase == Phase::Roll {
            let done = d.t >= KNOCKDOWN_ROLL;
            self.vx = (d.dir * KNOCKDOWN_ROLL_DISTANCE) / KNOCKDOWN_ROLL;
            if done {
                self.stand();
            }
        } else if d.phase == Phase::Rise && d.t >= KNOCKDOWN_RISE {
            self.stand();
        }
    }

    fn stand(&mut self) {
        self.down = None;
        self.vx = 0.0;
        self.wake = KNOCKDOWN_GRACE;
        self.land = LANDING_SOFT;
    }
}

/// Two bodies cannot stand in one place: overlapping at the same height each is pushed half the overlap apart,
/// and neither keeps walking into the other. One far enough above the other passes over it, and one on the floor
/// is stepped over. The game calls it only while both fighters have life left.
pub fn separate(a: &mut Body, b: &mut Body, arena: &Arena) {
    if a.floored() || b.floored() || (a.air - b.air).abs() >= BODY - 6.0 {
        return;
    }
    let dx = b.x - a.x;
    let overlap = PERSONAL - dx.abs();
    if overlap <= 0.0 {
        return;
    }
    // exactly on top of each other: part them along the way they face
    let dir = if dx != 0.0 { js_sign(dx) } else { a.face };
    let lo = arena.left + HALF_WIDTH;
    let hi = arena.right - HALF_WIDTH;
    a.x = clamp(a.x - (dir * overlap) / 2.0, lo, hi);
    b.x = clamp(b.x + (dir * overlap) / 2.0, lo, hi);
    // against a wall one side cannot give: the other takes the rest
    let left = PERSONAL - (b.x - a.x).abs();
    if left > 0.0 {
        if a.x == lo || a.x == hi {
            b.x = clamp(b.x + dir * left, lo, hi);
        } else {
            a.x = clamp(a.x - dir * left, lo, hi);
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
    if s == 0.0 || s.is_nan() {
        1.0
    } else {
        s
    }
}

/// JS `Math.max(a, b)`, signed zeros included (+0 beats -0).
fn max(a: f64, b: f64) -> f64 {
    if a == 0.0 && b == 0.0 {
        return if a.is_sign_positive() { a } else { b };
    }
    pixfx_core::math::js_max(a, b)
}

/// JS `Math.min(a, b)`, signed zeros included (-0 beats +0).
fn min(a: f64, b: f64) -> f64 {
    if a == 0.0 && b == 0.0 {
        return if a.is_sign_negative() { a } else { b };
    }
    pixfx_core::math::js_min(a, b)
}

/// The duel's `clamp`: `Math.max(lo, Math.min(hi, v))`.
fn clamp(v: f64, lo: f64, hi: f64) -> f64 {
    max(lo, min(hi, v))
}
