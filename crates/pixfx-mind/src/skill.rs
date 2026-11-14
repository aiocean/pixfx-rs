//! What a fighter is, handed to the mind once: its skills as data, its body's sizes and its temperament.

/// What a skill does, which is all the mind knows of it. The game maps its own skills onto these.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[cfg_attr(feature = "ts", derive(tsify::Tsify))]
pub enum SkillKind {
    /// A blade at arm's length.
    Melee,
    /// A blade from above, onto the foe below.
    Dive,
    /// A rush through the foe and out the other side.
    Dash,
    /// A burst from the palm at short range.
    Blast,
    /// A shot that flies level.
    Bolt,
    /// A slow ball that hunts the foe.
    Seeker,
    /// Something that lands on a spot after a while (a seal, an icicle, a collapse).
    Area,
    /// Something that stays on a spot and hurts whoever stands in it (a fire, a vortex).
    Field,
    Heal,
    /// A shell that takes blows for the caster.
    Ward,
    /// A dome that turns every blow aside for a moment.
    Barrier,
    /// Gone from here, there at once.
    Blink,
}

/// What a skill does to the one it lands on, besides damage.
#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "serde",
    serde(rename_all = "camelCase", deny_unknown_fields, default)
)]
#[cfg_attr(feature = "ts", derive(tsify::Tsify))]
pub struct Inflicts {
    /// Seconds frozen.
    pub freeze: f64,
    pub shock: bool,
    /// Seconds silenced.
    pub silence: f64,
    /// Seconds of broken defence.
    pub def_down: f64,
    pub burn: bool,
    pub chill: bool,
    /// How hard it drags the foe in, game pixels a second.
    pub pull: f64,
}

/// One skill a fighter holds. Times in seconds, lengths in game pixels.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "serde",
    serde(rename_all = "camelCase", deny_unknown_fields)
)]
#[cfg_attr(feature = "ts", derive(tsify::Tsify))]
pub struct Skill {
    /// The game's name for it, handed back as is in `Choice::Cast`.
    pub name: String,
    pub kind: SkillKind,
    pub cost: f64,
    pub windup: f64,
    pub recover: f64,
    pub damage: f64,
    /// Range of a blade or blast, distance of a dash or blink, reach of an area or field.
    pub reach: f64,
    pub radius: f64,
    pub speed: f64,
    /// From release until an area or field lands its blow.
    pub lands: f64,
    /// Blows it lands one after another (a barrage), at least 1.
    pub strikes: u32,
    pub ultimate: bool,
    /// Hits only a foe on the floor.
    pub grounded: bool,
    #[cfg_attr(feature = "serde", serde(default))]
    pub inflicts: Inflicts,
}

/// The fighter's sizes the mind reasons with, from the game's body.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "serde",
    serde(rename_all = "camelCase", deny_unknown_fields)
)]
#[cfg_attr(feature = "ts", derive(tsify::Tsify))]
pub struct BodySize {
    pub half_width: f64,
    /// Height of the chest over the feet.
    pub chest: f64,
    /// Height of the whole body.
    pub body: f64,
    /// Height of the body ducking.
    pub duck: f64,
    /// Walking speed, game pixels a second.
    pub walk: f64,
}

/// A fighter's temperament: how it picks its moments. A school of the game is one profile.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "serde",
    serde(rename_all = "camelCase", deny_unknown_fields)
)]
#[cfg_attr(feature = "ts", derive(tsify::Tsify))]
pub struct Profile {
    /// How much it values damage dealt.
    pub aggression: f64,
    /// Seconds it takes a breath in neutral after an attack.
    pub tempo: f64,
    /// How much the breath varies, as a share of it.
    pub spread: f64,
    /// The distance it likes to fight at with spells.
    pub range: f64,
    /// The distance it keeps when its qi runs low.
    pub dry: f64,
    /// The height it likes to hover at while it has the qi.
    pub sky: f64,
    /// How much a miss's cost weighs against an attack.
    pub risk: f64,
    /// The share of an attack's worth that its risk never takes away.
    pub floor: f64,
    /// How much more it weighs a miss's cost when ahead on life.
    pub lead: f64,
    /// How many tries a skill's first guess at landing counts as.
    pub prior: f64,
    /// Seconds before it sees what the foe does.
    pub reaction: f64,
    /// Seconds between thoughts, on average.
    pub think: f64,
    /// Each posture's score is multiplied by this, in `Posture::ALL` order.
    pub posture_scale: [f64; 8],
}
