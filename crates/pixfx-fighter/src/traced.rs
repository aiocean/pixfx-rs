//! Poses traced from art drawn elsewhere (an artist's, or an image model's): the picture cut out, matched to a
//! look's colours, shrunk to the sprite's pixels and merged into rects ([`trace`]). A pose with a traced table
//! ([`pose`]) is painted from it, rect by rect in the look's colour for each rect's [`Role`], so one table serves
//! every look with the same hairdo, and a hit's flash, ice and lightning tint it like any other pose.
//!
//! The tables in `traced/` are written by `pixfx trace`; trace the art again rather than editing them by hand.

use pixfx_draw::Rgba;
use pixfx_draw::trace::{Map, RgbaImage, nearest, rects, shrink};

use crate::look::{Hairdo, Look};
use crate::pose::Pose;

/// Which of a look's colours a traced pixel is painted in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Role {
    Robe0,
    Robe1,
    Robe2,
    Inner0,
    Inner1,
    Trim0,
    Trim1,
    Hair0,
    Hair1,
    Skin0,
    Skin1,
    Boot,
    Blade0,
    Blade1,
    Blade2,
    Jade,
    Ribbon0,
    Ribbon1,
    Outline,
}

impl Role {
    /// Every role, in the order [`trace`] matches colours against them.
    pub const ALL: [Role; 19] = [
        Role::Robe0,
        Role::Robe1,
        Role::Robe2,
        Role::Inner0,
        Role::Inner1,
        Role::Trim0,
        Role::Trim1,
        Role::Hair0,
        Role::Hair1,
        Role::Skin0,
        Role::Skin1,
        Role::Boot,
        Role::Blade0,
        Role::Blade1,
        Role::Blade2,
        Role::Jade,
        Role::Ribbon0,
        Role::Ribbon1,
        Role::Outline,
    ];

    /// Its colour in `look`.
    pub fn color(self, look: &Look) -> Rgba {
        let [r0, r1, r2] = look.robe;
        let [i0, i1] = look.inner;
        let [t0, t1] = look.trim;
        let [h0, h1] = look.hair;
        let [s0, s1] = look.skin;
        let [b0, b1, b2] = look.blade;
        let [ri0, ri1] = look.ribbon;
        match self {
            Role::Robe0 => r0,
            Role::Robe1 => r1,
            Role::Robe2 => r2,
            Role::Inner0 => i0,
            Role::Inner1 => i1,
            Role::Trim0 => t0,
            Role::Trim1 => t1,
            Role::Hair0 => h0,
            Role::Hair1 => h1,
            Role::Skin0 => s0,
            Role::Skin1 => s1,
            Role::Boot => look.boot,
            Role::Blade0 => b0,
            Role::Blade1 => b1,
            Role::Blade2 => b2,
            Role::Jade => look.jade,
            Role::Ribbon0 => ri0,
            Role::Ribbon1 => ri1,
            Role::Outline => look.outline,
        }
    }
}

/// A rect of a traced pose in the painter's frame: `dx` forward from the body's middle, `dy` up from the feet to
/// its bottom edge, `w` x `h` pixels, painted in `role`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Traced {
    pub dx: i16,
    pub dy: i16,
    pub w: u16,
    pub h: u16,
    pub role: Role,
}

/// Trace `art` (a fighter facing right, its background already cut away) drawn at `px` of its pixels to a sprite
/// pixel, in `look`'s colours, standing `lift` pixels above the feet, centred on the body's middle. The dark
/// outline round the figure is left to the painter's own outline, which rings every pose alike; dark pixels inside
/// it stay.
pub fn trace(art: &RgbaImage, look: &Look, px: f64, lift: i32) -> Vec<Traced> {
    let palette = Role::ALL.map(|r| r.color(look));
    let mut map = shrink(&nearest(art, &palette), art.w, px);
    let outline = Role::ALL.iter().position(|&r| r == Role::Outline);
    let edge: Vec<usize> = (0..map.cells.len())
        .filter(|&i| {
            let (x, y) = ((i % map.w as usize) as i64, (i / map.w as usize) as i64);
            map.get(x, y).is_some()
                && map.get(x, y) == outline
                && [(-1, 0), (1, 0), (0, -1), (0, 1)]
                    .iter()
                    .any(|(ox, oy)| map.get(x + ox, y + oy).is_none())
        })
        .collect();
    for i in edge {
        if let Some(cell) = map.cells.get_mut(i) {
            *cell = None;
        }
    }
    placed(&map, lift)
}

/// The rects of `map`, its bottom row `lift` pixels above the feet and its middle on the body's middle.
fn placed(map: &Map, lift: i32) -> Vec<Traced> {
    let mid = map.w as i32 / 2;
    rects(map)
        .into_iter()
        .filter_map(|r| {
            Some(Traced {
                dx: i16::try_from(r.x as i32 - mid).ok()?,
                dy: i16::try_from(lift + map.h as i32 - (r.y + r.h) as i32).ok()?,
                w: u16::try_from(r.w).ok()?,
                h: u16::try_from(r.h).ok()?,
                role: *Role::ALL.get(r.value)?,
            })
        })
        .collect()
}

/// The traced poses: a pose and hairdo, and its rects.
const TABLES: &[(Pose, Hairdo, &[Traced])] = &[
    (
        Pose::Launched,
        Hairdo::Topknot,
        include!("traced/launched_topknot.rs"),
    ),
    (
        Pose::Launched,
        Hairdo::Flowing,
        include!("traced/launched_flowing.rs"),
    ),
    (
        Pose::Tumble,
        Hairdo::Topknot,
        include!("traced/tumble_topknot.rs"),
    ),
    (
        Pose::Tumble,
        Hairdo::Flowing,
        include!("traced/tumble_flowing.rs"),
    ),
    (
        Pose::Fall,
        Hairdo::Topknot,
        include!("traced/fall_topknot.rs"),
    ),
    (
        Pose::Fall,
        Hairdo::Flowing,
        include!("traced/fall_flowing.rs"),
    ),
    (
        Pose::Down,
        Hairdo::Topknot,
        include!("traced/down_topknot.rs"),
    ),
    (
        Pose::Down,
        Hairdo::Flowing,
        include!("traced/down_flowing.rs"),
    ),
];

/// The traced rects fighter poses with `hairdo` are painted from in `pose`, if that pose has been traced.
pub fn pose(pose: Pose, hairdo: Hairdo) -> Option<&'static [Traced]> {
    TABLES
        .iter()
        .find(|(p, h, _)| *p == pose && *h == hairdo)
        .map(|&(_, _, rects)| rects)
}
