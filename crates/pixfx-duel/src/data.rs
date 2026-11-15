//! The duel's tables and numbers: skills, schools, stats, tiers, traits, luck, and the constants every rule reads.
//! Each is ported from `site/assets/duel.js` with the same values; `tests/data.rs` checks them against it.

use pixfx_draw::math::js_min;

/// Effects the arena itself shows when they are in the registry: the lamps and the night.
pub const DECOR: [&str; 2] = ["fire/brazier", "weather/fireflies"];

/// Flying: the highest a fighter goes, and the qi it costs a second.
pub struct Fly {
    pub top: f64,
    pub cost: f64,
}
pub const FLY: Fly = Fly { top: 96.0, cost: 3.0 };

/// Rage is a small whole-number pool: a point for each blow landed; when full, the school's ultimate.
pub const RAGE: f64 = 4.0;
/// Frozen: no moving, no acting, in seconds.
pub const FREEZE: f64 = 0.9;

/// Chill: each chilling hit adds a stack (at most `max`), all fading `time` seconds after the last; each slows
/// walking by `slow`, and the last freezes and clears them.
pub struct Chill {
    pub max: f64,
    pub time: f64,
    pub slow: f64,
}
pub const CHILL: Chill = Chill { max: 3.0, time: 2.5, slow: 0.85 };

/// Shock: each shocking hit adds a stack (at most `max`); a shocked fighter twitches every
/// `every × fast^(stacks − 1)` seconds, spending a stack, losing `share` of the caster's attack and held `hold` s.
pub struct Shock {
    pub max: f64,
    pub every: f64,
    pub fast: f64,
    pub share: f64,
    pub hold: f64,
}
pub const SHOCK: Shock = Shock { max: 5.0, every: 0.6, fast: 0.854, share: 0.03, hold: 0.12 };

/// While the foe carries a status, the share of draws that come from the skills that spend it.
pub const PAY_DRAW: f64 = 1.0 / 3.0;

/// A status a payoff skill spends.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Shock,
    Chill,
}

/// Skills that spend the foe's stacks of a status to hit harder: this much more power per stack.
pub const PAYOFF: &[(&str, Status, f64)] = &[
    ("palm", Status::Shock, 0.12),
    ("burst", Status::Shock, 0.12),
    ("shatter", Status::Chill, 0.15),
    ("lotus", Status::Chill, 0.15),
];

/// The payoff of spell `name`, if it is one.
pub fn payoff(name: &str) -> Option<(Status, f64)> {
    PAYOFF.iter().find(|(n, _, _)| *n == name).map(|&(_, s, per)| (s, per))
}

/// Grit: each hard control adds `per`; grit falls `fall` a second. A control lasts `seconds / (1 + grit)`, and from
/// grit `thaw` a freeze is only a chill stack.
pub struct Grit {
    pub per: f64,
    pub fall: f64,
    pub thaw: f64,
}
pub const GRIT: Grit = Grit { per: 1.0, fall: 0.4, thaw: 2.0 };

pub const DEFENCE: f64 = 500.0;
pub const SPREAD: f64 = 0.03;

/// Fury: each blow a fury fighter lands raises its attack by `per`, up to `max` stacks, for the rest of the round.
pub struct Fury {
    pub per: f64,
    pub max: f64,
}
pub const FURY: Fury = Fury { per: 0.04, max: 5.0 };

/// Defence broken by a spell: this much defence lost.
pub const DEF_DOWN: f64 = 0.3;

/// Burn: a share of the caster's attack every tick, ignoring defence.
pub struct Burn {
    pub time: f64,
    pub every: f64,
    pub share: f64,
}
pub const BURN: Burn = Burn { time: 3.0, every: 0.5, share: 0.05 };

/// A ward holds 1 to `charges` charges for `time` seconds; each erases one whole blow.
pub struct Ward {
    pub charges: f64,
    pub time: f64,
}
pub const WARD: Ward = Ward { charges: 3.0, time: 6.0 };

/// A skill's tier.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum Rarity {
    Common = 0,
    Rare = 1,
    Epic = 2,
    Legendary = 3,
}

impl Rarity {
    /// The tiers, lowest first: a duplicate draw lifts a skill one step, up to the last.
    pub const ALL: [Rarity; 4] = [Rarity::Common, Rarity::Rare, Rarity::Epic, Rarity::Legendary];

    pub fn name(self) -> &'static str {
        match self {
            Rarity::Common => "common",
            Rarity::Rare => "rare",
            Rarity::Epic => "epic",
            Rarity::Legendary => "legendary",
        }
    }

    /// One tier up; a legendary stays legendary.
    pub fn up(self) -> Rarity {
        match self {
            Rarity::Common => Rarity::Rare,
            Rarity::Rare => Rarity::Epic,
            Rarity::Epic | Rarity::Legendary => Rarity::Legendary,
        }
    }
}

/// How likely a draw is to bring a tier, how much more power, and how many traits it brings (a common one: `chance`
/// of one). A legendary is never drawn: a skill reaches it only by duplicates.
#[derive(Clone, Copy, Debug)]
pub struct RarityData {
    pub odds: f64,
    pub power: f64,
    pub traits: usize,
    pub chance: Option<f64>,
}
pub const RARITY: [RarityData; 4] = [
    RarityData { odds: 0.7, power: 1.0, traits: 0, chance: Some(0.4) },
    RarityData { odds: 0.22, power: 1.15, traits: 1, chance: None },
    RarityData { odds: 0.08, power: 1.3, traits: 2, chance: None },
    RarityData { odds: 0.0, power: 1.5, traits: 3, chance: None },
];

/// A trait a bar skill can be drawn with.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum Trait {
    Swift,
    Heavy,
    Piercing,
    Lingering,
    Cheap,
}

impl Trait {
    /// In `duel.js`'s order: the order traits are rolled from.
    pub const ALL: [Trait; 5] = [Trait::Swift, Trait::Heavy, Trait::Piercing, Trait::Lingering, Trait::Cheap];

    pub fn name(self) -> &'static str {
        match self {
            Trait::Swift => "swift",
            Trait::Heavy => "heavy",
            Trait::Piercing => "piercing",
            Trait::Lingering => "lingering",
            Trait::Cheap => "cheap",
        }
    }
}

/// What each trait does to the skill it is drawn on.
pub struct Traits {
    pub swift_cooldown: f64,
    pub heavy_power: f64,
    pub lingering_stacks: f64,
    pub cheap_cost: f64,
}
pub const TRAITS: Traits = Traits { swift_cooldown: 0.6, heavy_power: 1.3, lingering_stacks: 1.0, cheap_cost: 0.5 };

/// What luck does to a cast.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum Luck {
    /// Hits twice as hard.
    Double,
    /// Costs nothing.
    Free,
    /// Ready again at once.
    Again,
}

impl Luck {
    pub fn name(self) -> &'static str {
        match self {
            Luck::Double => "double",
            Luck::Free => "free",
            Luck::Again => "again",
        }
    }
}

/// A cast of a bar skill is lucky this often.
pub struct Lucky {
    pub chance: f64,
    pub kinds: [Luck; 3],
}
pub const LUCKY: Lucky = Lucky { chance: 0.08, kinds: [Luck::Double, Luck::Free, Luck::Again] };

/// The kinds of blow a guard can turn aside (a zone or a field falls past it).
pub const GUARDED: [Kind; 5] = [Kind::Melee, Kind::Blast, Kind::Dash, Kind::Bolt, Kind::Orb];

/// Whether `t` does anything to spell `name`: heavy needs a blow, piercing a blow a guard can stop, lingering a chill
/// or a shock.
pub fn fits(t: Trait, name: &str) -> bool {
    let Some(s) = spell(name) else { return false };
    let damage = s.damage.unwrap_or(0.0);
    match t {
        Trait::Heavy => damage > 0.0,
        Trait::Piercing => damage > 0.0 && GUARDED.contains(&s.kind),
        Trait::Lingering => s.chill || s.shock,
        Trait::Swift | Trait::Cheap => true,
    }
}

/// An evolved skill is this many times as strong.
pub const EVOLVED: f64 = 1.5;

/// The power a bar skill strikes with: its tier, heavy, and evolution.
pub fn slot_power(rarity: Rarity, traits: &[Trait], evolved: bool) -> f64 {
    RARITY[rarity as usize].power
        * (if traits.contains(&Trait::Heavy) { TRAITS.heavy_power } else { 1.0 })
        * (if evolved { EVOLVED } else { 1.0 })
}

/// How many of its school's pool a fighter plays in one fight.
pub const POOL_SIZE: usize = 8;

/// The rotating bar: `slots` skills at a time; an emptied slot counts down `wait + spread × roll` seconds.
pub struct Bar {
    pub slots: usize,
    pub wait: f64,
    pub spread: f64,
}
pub const BAR: Bar = Bar { slots: 3, wait: 1.5, spread: 1.0 };

/// Seconds a bar skill may sit ready and uncast before it leaves for another draw.
pub const STALE: f64 = 6.0;

/// The pairs that evolve, per school.
pub fn evolves(school: School) -> &'static [[&'static str; 2]] {
    match school {
        School::Thunder => &[["bolt", "seal"], ["palm", "step"], ["quake", "burst"]],
        School::Frost => &[["shard", "icicle"], ["shatter", "flurry"], ["lotus", "jet"]],
    }
}

/// A school.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum School {
    Thunder,
    Frost,
}

impl School {
    pub fn name(self) -> &'static str {
        match self {
            School::Thunder => "thunder",
            School::Frost => "frost",
        }
    }
}

/// Each school's fighter: full life, attack, the two defences, crit chance and damage, dodge, and its passives.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase", deny_unknown_fields))]
pub struct Stats {
    pub hp: f64,
    pub atk: f64,
    pub pdef: f64,
    pub mdef: f64,
    pub crit: f64,
    pub crit_damage: f64,
    pub dodge: f64,
    pub fury: bool,
    pub unyielding: bool,
    pub attuned: bool,
}

pub fn stats(school: School) -> &'static Stats {
    static THUNDER: Stats = Stats {
        hp: 2500.0, atk: 345.0, pdef: 130.0, mdef: 110.0, crit: 0.18, crit_damage: 1.6, dodge: 0.05,
        fury: true, unyielding: true, attuned: false,
    };
    static FROST: Stats = Stats {
        hp: 3300.0, atk: 520.0, pdef: 145.0, mdef: 165.0, crit: 0.15, crit_damage: 1.7, dodge: 0.11,
        fury: false, unyielding: false, attuned: true,
    };
    match school {
        School::Thunder => &THUNDER,
        School::Frost => &FROST,
    }
}

pub const QI_REGEN: f64 = 3.0;
pub const MEDITATE_REGEN: f64 = 16.0;

/// Each fighter's temperament for pixfx-mind (see `duel.js` TEMPERAMENT).
pub struct Temperament {
    pub spread: f64,
    pub risk: f64,
    pub floor: f64,
    pub lead: f64,
    pub prior: f64,
    pub reaction: f64,
    pub think: f64,
}
pub const TEMPERAMENT: Temperament =
    Temperament { spread: 0.8, risk: 0.6, floor: 0.3, lead: 0.8, prior: 6.0, reaction: 0.16, think: 0.12 };

/// Qi a block costs.
pub const GUARD_COST: f64 = 8.0;
/// The share of a blocked blow's damage that still lands.
pub const CHIP: f64 = 0.2;
/// Guard by the size of the blow: a guarded blow of `power` is turned aside with chance `min(0.5r, 0.5√r)`.
pub const GUARD: f64 = 20.0;

/// The chance a guard turns aside a blow of `power`.
pub fn block_chance(power: f64) -> f64 {
    // JS `!(power > 0)` holds for NaN too
    if power.is_nan() || power <= 0.0 {
        return 1.0;
    }
    let r = GUARD / power;
    js_min(js_min(1.0, 0.5 * r), 0.5 * libm::sqrt(r))
}

pub const KNOCKBACK: f64 = 130.0;
pub const ROUND_PAUSE: f64 = 3.2;
pub const CAST_CIRCLE: &str = "arcane/rune-circle";
/// How long a barrier flares after it turns a blow aside, in seconds.
pub const RING: f64 = 0.22;
/// How urgent a threat must be before a fighter spends its barrier on it.
pub const BARRIER_THREAT: f64 = 10.0;
pub const CLASH: &str = "impacts/parry";

/// What a spell does when it lands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum Kind {
    Melee,
    Dash,
    Blast,
    Bolt,
    Orb,
    Zone,
    Void,
    Heal,
    Ward,
    Barrier,
    Blink,
}

impl Kind {
    pub fn name(self) -> &'static str {
        match self {
            Kind::Melee => "melee",
            Kind::Dash => "dash",
            Kind::Blast => "blast",
            Kind::Bolt => "bolt",
            Kind::Orb => "orb",
            Kind::Zone => "zone",
            Kind::Void => "void",
            Kind::Heal => "heal",
            Kind::Ward => "ward",
            Kind::Barrier => "barrier",
            Kind::Blink => "blink",
        }
    }
}

/// Which defence a blow is cut by.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum Blow {
    Physical,
    Magic,
}

impl Blow {
    pub fn name(self) -> &'static str {
        match self {
            Blow::Physical => "physical",
            Blow::Magic => "magic",
        }
    }
}

/// A golden bell's answer to a close blow.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Counter {
    pub damage: f64,
    pub stun: f64,
}

/// A spell: what it costs, how long the fighter is committed before (`windup`) and after (`recover`) it lands, its
/// cooldown, and its own numbers. Fields `duel.js` leaves out are `None` or `false`.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct Spell {
    #[cfg_attr(feature = "serde", serde(skip))]
    pub name: &'static str,
    pub kind: Kind,
    #[cfg_attr(feature = "serde", serde(rename = "type", skip_serializing_if = "Option::is_none"))]
    pub blow: Option<Blow>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub fx: Option<&'static str>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub hit_fx: Option<&'static str>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub end_fx: Option<&'static str>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub slam_fx: Option<&'static str>,
    pub cost: f64,
    pub windup: f64,
    pub recover: f64,
    pub cooldown: f64,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub range: Option<f64>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub distance: Option<f64>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub damage: Option<f64>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub sigil: Option<f64>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub impact: Option<f64>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub radius: Option<f64>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub notice: Option<f64>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub def_down: Option<f64>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub life: Option<f64>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub height: Option<f64>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub pull: Option<f64>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub reach: Option<f64>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub under: Option<f64>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub tick: Option<f64>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub freeze: Option<f64>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub speed: Option<f64>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub silence: Option<f64>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub pull_until: Option<f64>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub steal_rage: Option<f64>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub strikes: Option<f64>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub every: Option<f64>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub aim_ahead: Option<f64>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub amount: Option<f64>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub time: Option<f64>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub spray: Option<f64>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "is_false"))]
    pub shock: bool,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "is_false"))]
    pub chill: bool,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "is_false"))]
    pub dive: bool,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "is_false"))]
    pub burn: bool,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "is_false"))]
    pub grounded: bool,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "is_false"))]
    pub ultimate: bool,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "is_false"))]
    pub seals: bool,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "is_false"))]
    pub reflect: bool,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "is_false"))]
    pub feet: bool,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub counter: Option<Counter>,
}

const BASE: Spell = Spell {
    name: "",
    kind: Kind::Melee,
    blow: None,
    fx: None,
    hit_fx: None,
    end_fx: None,
    slam_fx: None,
    cost: 0.0,
    windup: 0.0,
    recover: 0.0,
    cooldown: 0.0,
    range: None,
    distance: None,
    damage: None,
    sigil: None,
    impact: None,
    radius: None,
    notice: None,
    def_down: None,
    life: None,
    height: None,
    pull: None,
    reach: None,
    under: None,
    tick: None,
    freeze: None,
    speed: None,
    silence: None,
    pull_until: None,
    steal_rage: None,
    strikes: None,
    every: None,
    aim_ahead: None,
    amount: None,
    time: None,
    spray: None,
    shock: false,
    chill: false,
    dive: false,
    burn: false,
    grounded: false,
    ultimate: false,
    seals: false,
    reflect: false,
    feet: false,
    counter: None,
};

use Blow::{Magic, Physical};

/// Every spell, in `duel.js`'s order.
static SPELLS: [Spell; 27] = [
    Spell { name: "slash", kind: Kind::Melee, blow: Some(Physical), cost: 0.0, windup: 0.2, recover: 0.32, cooldown: 0.7, range: Some(24.0), damage: Some(5.0), ..BASE },
    // Thunder: a flash of lightning that carries the body through the foe and out the other side
    Spell { name: "step", kind: Kind::Dash, blow: Some(Physical), fx: Some("spawn/poof"), cost: 24.0, windup: 0.26, recover: 0.32, cooldown: 3.4, distance: Some(72.0), damage: Some(15.0), ..BASE },
    // Thunder: a gold seal opens on the ground under the foe, and a thunderclap bursts from it
    Spell {
        name: "seal", kind: Kind::Zone, blow: Some(Magic), fx: Some("explosions/plasma-explosion"), cost: 30.0, windup: 0.3, recover: 0.3, cooldown: 2.4,
        sigil: Some(0.4), impact: Some(0.08), radius: Some(16.0), damage: Some(22.0), notice: Some(0.0), shock: true, ..BASE
    },
    // Frost: a lotus of ice shards blooms from the palm at close range, and cracks the foe's guard
    Spell { name: "lotus", kind: Kind::Blast, blow: Some(Magic), fx: Some("holy/feather-burst"), cost: 24.0, windup: 0.3, recover: 0.3, cooldown: 2.2, range: Some(40.0), damage: Some(18.0), def_down: Some(3.0), ..BASE },
    // Frost: a pale fire that does not warm, lit under the foe; it burns whoever stands in it
    Spell {
        name: "frostfire", kind: Kind::Void, blow: Some(Magic), fx: Some("fire/spirit-flame"), cost: 33.0, windup: 0.4, recover: 0.3, cooldown: 5.0,
        life: Some(2.6), height: Some(0.0), pull: Some(0.0), reach: Some(18.0), under: Some(18.0), tick: Some(5.0), damage: Some(8.0), chill: true, ..BASE
    },
    // Frost: a blue seal under the foe that shatters into a burst of cold, freezing it
    Spell {
        name: "shatter", kind: Kind::Zone, blow: Some(Magic), fx: Some("explosions/plasma-explosion"), cost: 33.0, windup: 0.35, recover: 0.3, cooldown: 3.0,
        sigil: Some(0.35), impact: Some(0.08), radius: Some(16.0), damage: Some(25.0), notice: Some(0.0), freeze: Some(0.6), ..BASE
    },
    // from above: drop onto the foe sword first, and crack the ground if it lands there
    Spell { name: "dive", kind: Kind::Melee, blow: Some(Physical), dive: true, cost: 12.0, windup: 0.22, recover: 0.4, cooldown: 1.6, range: Some(26.0), damage: Some(11.0), speed: Some(260.0), slam_fx: Some("explosions/dirt-explosion"), ..BASE },
    Spell { name: "bolt", kind: Kind::Bolt, blow: Some(Magic), fx: Some("projectiles/laser-bolt"), cost: 18.0, windup: 0.28, recover: 0.22, cooldown: 1.3, speed: Some(230.0), damage: Some(13.0), shock: true, ..BASE },
    Spell {
        name: "orb", kind: Kind::Orb, blow: Some(Magic), fx: Some("lightning/ball-lightning"), hit_fx: Some("explosions/plasma-explosion"), burn: true, shock: true,
        cost: 45.0, windup: 0.6, recover: 0.3, cooldown: 4.5, speed: Some(26.0), life: Some(5.5), damage: Some(14.0), radius: Some(10.0), ..BASE
    },
    // icicle-fall drops its icicle at 0.944 s and only starts to tremble, which is when a fighter notices it, at 0.3 s
    Spell { name: "icicle", kind: Kind::Zone, blow: Some(Magic), def_down: Some(4.0), fx: Some("ice/icicle-fall"), cost: 33.0, windup: 0.42, recover: 0.28, cooldown: 2.2, impact: Some(0.944), radius: Some(16.0), damage: Some(28.0), notice: Some(0.3), grounded: true, freeze: Some(FREEZE), ..BASE },
    // implosion sucks in until 0.8 s and pops at 1.02 s
    Spell {
        name: "collapse", kind: Kind::Zone, blow: Some(Magic), silence: Some(1.5), fx: Some("explosions/implosion"), cost: 39.0, windup: 0.5, recover: 0.3, cooldown: 2.4,
        impact: Some(1.02), radius: Some(18.0), damage: Some(25.0), notice: Some(0.15), pull: Some(60.0), reach: Some(52.0), pull_until: Some(0.8), ..BASE
    },
    // a thunderclap from the open palm, at arm's length
    Spell { name: "palm", kind: Kind::Blast, blow: Some(Physical), steal_rage: Some(1.0), fx: Some("explosions/plasma-explosion"), cost: 27.0, windup: 0.32, recover: 0.35, cooldown: 2.8, range: Some(38.0), damage: Some(20.0), shock: true, ..BASE },
    // ultimates, cast with full rage. Thunder: three thunderclaps fall one after another where the foe will be.
    Spell {
        name: "tribulation", kind: Kind::Zone, blow: Some(Magic), fx: Some("explosions/plasma-explosion"), ultimate: true, cost: 0.0, windup: 0.7, recover: 0.5, cooldown: 0.0,
        impact: Some(0.08), radius: Some(12.0), damage: Some(14.0), notice: Some(0.0), strikes: Some(3.0), every: Some(0.7), aim_ahead: Some(0.5), ..BASE
    },
    // Frost: a great collapse that drags the foe in from far off and freezes it where it pops.
    Spell {
        name: "prison", kind: Kind::Zone, blow: Some(Magic), silence: Some(2.0), fx: Some("explosions/implosion"), ultimate: true, cost: 0.0, windup: 0.7, recover: 0.5, cooldown: 0.0,
        impact: Some(1.02), radius: Some(26.0), damage: Some(31.0), notice: Some(0.1), pull: Some(70.0), reach: Some(110.0), pull_until: Some(0.9), seals: true, freeze: Some(FREEZE * 1.4), ..BASE
    },
    // void-orb hangs 62 px over its origin and loops every 3.2 s
    Spell {
        name: "void", kind: Kind::Void, blow: Some(Magic), fx: Some("shadow/void-orb"), end_fx: Some("explosions/implosion"),
        cost: 60.0, windup: 0.9, recover: 0.45, cooldown: 11.0, life: Some(3.2), height: Some(62.0), pull: Some(34.0), reach: Some(90.0), under: Some(22.0), tick: Some(4.0), damage: Some(14.0), ..BASE
    },
    Spell { name: "heal", kind: Kind::Heal, fx: Some("holy/feather-burst"), cost: 35.0, windup: 0.85, recover: 0.3, cooldown: 9.0, amount: Some(20.0), ..BASE },
    // a shell of frost qi that takes blows for the caster until it breaks
    Spell { name: "ward", kind: Kind::Ward, fx: Some("holy/feather-burst"), cost: 45.0, windup: 0.5, recover: 0.25, cooldown: 8.0, ..BASE },
    // the defences: a dome of qi for a moment that turns every blow aside
    Spell { name: "bell", kind: Kind::Barrier, fx: Some("arcane/rune-circle"), cost: 18.0, windup: 0.06, recover: 0.1, cooldown: 6.5, time: Some(2.0), counter: Some(Counter { damage: 9.0, stun: 0.6 }), ..BASE },
    Spell { name: "mirror", kind: Kind::Barrier, fx: Some("arcane/rune-circle"), cost: 18.0, windup: 0.06, recover: 0.1, cooldown: 6.5, time: Some(2.2), reflect: true, ..BASE },
    Spell { name: "blink", kind: Kind::Blink, fx: Some("spawn/poof"), cost: 20.0, windup: 0.06, recover: 0.12, cooldown: 1.8, distance: Some(56.0), ..BASE },
    // Thunder: a quick spark off the fingertips, light but fast
    Spell { name: "spark", kind: Kind::Bolt, blow: Some(Magic), fx: Some("projectiles/laser-bolt"), cost: 10.0, windup: 0.18, recover: 0.18, cooldown: 0.9, speed: Some(300.0), damage: Some(8.0), shock: true, ..BASE },
    // Thunder: a palm that bursts with everything the foe's shocks have gathered
    Spell { name: "burst", kind: Kind::Blast, blow: Some(Physical), fx: Some("explosions/plasma-explosion"), cost: 30.0, windup: 0.36, recover: 0.4, cooldown: 3.0, range: Some(30.0), damage: Some(24.0), ..BASE },
    // Thunder: the ground under the foe cracks and blows up
    Spell {
        name: "quake", kind: Kind::Zone, blow: Some(Physical), fx: Some("explosions/small-explosion"), cost: 26.0, windup: 0.3, recover: 0.3, cooldown: 2.6,
        sigil: Some(0.3), impact: Some(0.05), radius: Some(14.0), damage: Some(18.0), notice: Some(0.0), grounded: true, ..BASE
    },
    // Thunder: a dome of hex cells that takes blows for the caster; it stands on the caster's feet
    Spell { name: "hex", kind: Kind::Ward, fx: Some("arcane/hex-shield"), feet: true, cost: 40.0, windup: 0.45, recover: 0.25, cooldown: 8.0, ..BASE },
    // Frost: a sliver of ice, light and quick, that chills where it strikes
    Spell { name: "shard", kind: Kind::Bolt, blow: Some(Magic), fx: Some("projectiles/laser-bolt"), cost: 10.0, windup: 0.18, recover: 0.18, cooldown: 0.9, speed: Some(300.0), damage: Some(7.0), chill: true, ..BASE },
    // Frost: a jet of freezing water from the palm, at middle reach; it sprays for `spray` seconds, then stops
    Spell { name: "jet", kind: Kind::Blast, blow: Some(Magic), fx: Some("water/water-jet"), spray: Some(0.6), cost: 22.0, windup: 0.3, recover: 0.3, cooldown: 2.4, range: Some(56.0), damage: Some(14.0), chill: true, ..BASE },
    // Frost: a short flurry of snow under the foe: two ticks that chill whoever stands in it, then its blow
    Spell {
        name: "flurry", kind: Kind::Void, blow: Some(Magic), fx: Some("weather/snow"), cost: 30.0, windup: 0.4, recover: 0.3, cooldown: 5.0,
        life: Some(1.2), height: Some(0.0), pull: Some(0.0), reach: Some(22.0), under: Some(22.0), tick: Some(3.0), damage: Some(10.0), chill: true, ..BASE
    },
];

/// Every spell, in `duel.js`'s order.
pub fn spells() -> &'static [Spell] {
    &SPELLS
}

/// Whether `b` is false: a spell's flag JS leaves out when it is off.
#[cfg(feature = "serde")]
fn is_false(b: &bool) -> bool {
    !*b
}

/// The spell called `name`.
pub fn spell(name: &str) -> Option<&'static Spell> {
    SPELLS.iter().find(|s| s.name == name)
}

/// How a school tints an effect its spells play.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tint {
    pub palette: Option<&'static [&'static str]>,
    pub accent: Option<&'static [&'static str]>,
    pub stars: Option<&'static [&'static str]>,
    pub fire: Option<&'static [&'static str]>,
    pub glow: Option<&'static str>,
    pub width: Option<f64>,
    pub height: Option<f64>,
}

const TINT: Tint = Tint { palette: None, accent: None, stars: None, fire: None, glow: None, width: None, height: None };

/// A school: its spells (`basics`, the `pool` its bar draws from, its `ultimate`), its temperament for the brain, and
/// its colours.
pub struct SchoolData {
    pub name: &'static str,
    pub basics: [&'static str; 3],
    pub pool: [&'static str; 11],
    pub ultimate: &'static str,
    pub signature: Kind,
    pub postures: [f64; 8],
    pub range: f64,
    pub dry: f64,
    pub aggression: f64,
    pub tempo: f64,
    pub sky: f64,
    pub bolt: [&'static str; 8],
    pub circle: [&'static str; 7],
    pub tint: &'static [(&'static str, Tint)],
}

const THUNDER_CIRCLE: [&str; 7] = ["#2a1a08", "#6a4410", "#b07a1a", "#e8b030", "#ffd84a", "#fff6c4", "#ffffff"];
const FROST_CIRCLE: [&str; 7] = ["#151a3d", "#212f6b", "#2c58aa", "#3a8bd8", "#62c8f4", "#dcfbff", "#ffffff"];

static THUNDER: SchoolData = SchoolData {
    name: "Thunder",
    basics: ["slash", "dive", "blink"],
    pool: ["bolt", "palm", "step", "seal", "orb", "bell", "heal", "spark", "burst", "quake", "hex"],
    ultimate: "tribulation",
    signature: Kind::Blast,
    postures: [1.0; 8],
    range: 64.0,
    // slash's range less 4: Thunder presses in to sword's length while its qi gathers
    dry: 20.0,
    aggression: 1.15,
    tempo: 0.8,
    sky: 22.0,
    bolt: ["#2a1a08", "#6a4410", "#b07a1a", "#e8b030", "#ffd84a", "#ffe98a", "#fff6c4", "#ffffff"],
    circle: THUNDER_CIRCLE,
    tint: &[
        (
            "explosions/plasma-explosion",
            Tint {
                palette: Some(&["#1a0e04", "#4a2a08", "#9a5a10", "#e0a020", "#ffd84a", "#fff3b0", "#ffffff"]),
                accent: Some(&["#2a1408", "#7a3a10", "#e07a1a", "#ffc23d", "#fff0b0"]),
                ..TINT
            },
        ),
        (
            "spawn/poof",
            Tint {
                palette: Some(&["#3a2a10", "#7a5a20", "#c09040", "#ffd84a", "#fff3b0", "#ffffff"]),
                stars: Some(&["#ffd84a", "#fff3b0", "#ffffff"]),
                ..TINT
            },
        ),
        ("holy/feather-burst", Tint { palette: Some(&["#6a4410", "#b07a1a", "#e8b030", "#ffd84a", "#fff6c4", "#ffffff"]), ..TINT }),
        (
            "explosions/dirt-explosion",
            Tint { fire: Some(&["#4a2a08", "#9a5a10", "#e0a020", "#ffc23d", "#ffd84a", "#ffe98a", "#fff6c4", "#ffffff"]), ..TINT },
        ),
        // a school's barrier rings in its circle's colours, and so does a blow it turns aside
        ("arcane/rune-circle", Tint { palette: Some(&THUNDER_CIRCLE), ..TINT }),
        ("impacts/parry", Tint { palette: Some(&["#b07a1a", "#e8b030", "#ffd84a", "#fff6c4", "#ffffff"]), ..TINT }),
    ],
};

static FROST: SchoolData = SchoolData {
    name: "Frost",
    basics: ["slash", "dive", "blink"],
    pool: ["lotus", "shatter", "icicle", "collapse", "frostfire", "void", "mirror", "ward", "shard", "jet", "flurry"],
    ultimate: "prison",
    signature: Kind::Zone,
    postures: [1.0; 8],
    range: 100.0,
    dry: 80.0,
    aggression: 1.1,
    tempo: 2.0,
    sky: 48.0,
    bolt: ["#151a3d", "#212f6b", "#2c58aa", "#3a8bd8", "#62c8f4", "#a6ecff", "#dcfbff", "#ffffff"],
    circle: FROST_CIRCLE,
    tint: &[
        (
            "explosions/plasma-explosion",
            Tint {
                palette: Some(&["#0a1430", "#16306a", "#2c58aa", "#4a9ae0", "#8fd8ff", "#dcfbff", "#ffffff"]),
                accent: Some(&["#0a1830", "#1a4a80", "#3a8bd8", "#a6ecff", "#ffffff"]),
                ..TINT
            },
        ),
        ("explosions/implosion", Tint { palette: Some(&["#060a18", "#10224a", "#1e4a8a", "#3a8bd8", "#8fd8ff", "#dcfbff", "#ffffff"]), ..TINT }),
        (
            "holy/feather-burst",
            Tint { palette: Some(&["#1e3f80", "#2c58aa", "#62c8f4", "#a6ecff", "#dcfbff", "#ffffff"]), glow: Some("#a6ecff"), ..TINT },
        ),
        (
            "fire/spirit-flame",
            Tint { palette: Some(&["#0a1430", "#16306a", "#2c58aa", "#3a8bd8", "#62c8f4", "#a6ecff", "#dcfbff", "#ffffff"]), ..TINT },
        ),
        (
            "spawn/poof",
            Tint {
                palette: Some(&["#16306a", "#2c58aa", "#62c8f4", "#a6ecff", "#dcfbff", "#ffffff"]),
                stars: Some(&["#a6ecff", "#dcfbff", "#ffffff"]),
                ..TINT
            },
        ),
        // a flurry falls on a patch round the foe, not the whole sky
        ("weather/snow", Tint { width: Some(48.0), height: Some(72.0), ..TINT }),
        // a school's barrier rings in its circle's colours, and so does a blow it turns aside
        ("arcane/rune-circle", Tint { palette: Some(&FROST_CIRCLE), ..TINT }),
        ("impacts/parry", Tint { palette: Some(&["#2c58aa", "#3a8bd8", "#62c8f4", "#dcfbff", "#ffffff"]), ..TINT }),
    ],
};

pub fn school(s: School) -> &'static SchoolData {
    match s {
        School::Thunder => &THUNDER,
        School::Frost => &FROST,
    }
}

/// How school `s` tints effect `fx`, if it does.
pub fn tint(s: School, fx: &str) -> Option<&'static Tint> {
    school(s).tint.iter().find(|(name, _)| *name == fx).map(|(_, t)| t)
}

/// ball-lightning's ball relative to its origin at time t (pixelfx lightning/ball-lightning.js `at`, P = 4).
pub fn orb_offset(t: f64) -> (f64, f64) {
    let a = (t / 4.0) * core::f64::consts::PI * 2.0;
    (
        38.0 * libm::sin(a) + 9.0 * libm::sin(2.0 * a + 1.0),
        -42.0 + 13.0 * libm::sin(2.0 * a + 0.4) + 6.0 * libm::sin(3.0 * a + 0.5),
    )
}

/// `base` to the whole power `n` (n ≥ 0), by multiplying, as `duel.js`'s `powi`.
pub fn powi(base: f64, n: f64) -> f64 {
    let mut out = 1.0;
    let mut i = 0.0;
    while i < n {
        out *= base;
        i += 1.0;
    }
    out
}

/// Seconds between shock ticks with `stacks` stacks.
pub fn shock_interval(stacks: f64) -> f64 {
    SHOCK.every * powi(SHOCK.fast, stacks - 1.0)
}
