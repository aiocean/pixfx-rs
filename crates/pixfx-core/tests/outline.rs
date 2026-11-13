//! Outlines as a host sees them: an instance drawn with `Options::outline` set, read back from its frame.

use pixfx_core::effect::{Effect, EffectState, Instance, Layer, Meta, Options};
use pixfx_core::outline::{DARK, Outline, PRESETS, Ring};
use pixfx_core::painter::Painter;
use pixfx_core::{Frame, Rgba};

const RED: Rgba = Rgba::hex24(0xff0000);
const WHITE: Rgba = Rgba::hex24(0xffffff);
const MINE: Rgba = Rgba::hex24(0x00ff00);

/// A red block on a 10x10 canvas: 2x2 at (4, 4), or `big` 6x6 at (2, 2); `alpha` below 1 dithers it as the
/// painter does (smoke is a sparse scatter of whole pixels). `own` draws a 1 px outline of its own round it.
struct Block {
    meta: Meta,
    own: bool,
    alpha: f64,
    big: bool,
}

struct Drawn {
    own: Option<Rgba>,
    alpha: f64,
    big: bool,
}

impl EffectState for Drawn {
    fn draw(&self, p: &mut Painter, _t: f64, layer: Layer) {
        if layer != Layer::Front {
            return;
        }
        if let Some(c) = self.own {
            p.rect(3.0, 3.0, 4.0, 4.0, c, 1.0);
        }
        if self.big {
            p.rect(2.0, 2.0, 6.0, 6.0, RED, self.alpha);
        } else {
            p.rect(4.0, 4.0, 2.0, 2.0, RED, self.alpha);
        }
    }
}

impl Effect for Block {
    fn meta(&self) -> &Meta {
        &self.meta
    }
    fn spawn(&self, opts: &Options, _seed: u64) -> Box<dyn EffectState> {
        let own = if self.own {
            opts.outline_color(MINE)
        } else {
            None
        };
        Box::new(Drawn {
            own,
            alpha: self.alpha,
            big: self.big,
        })
    }
    fn draws_outline(&self) -> bool {
        self.own
    }
}

const fn meta(id: &'static str) -> Meta {
    Meta {
        id,
        name: "Block",
        desc: "A block.",
        tags: &["test"],
        size: (10, 10),
        origin: (0, 0),
        duration: 1.0,
        looping: true,
        fps: 30,
        seed: 1,
        state: None,
        options: &[],
    }
}

static PLAIN: Block = Block {
    meta: meta("test/block"),
    own: false,
    alpha: 1.0,
    big: false,
};
static OWN: Block = Block {
    meta: meta("test/own-outline"),
    own: true,
    alpha: 1.0,
    big: false,
};
/// Smoke-faint: a 6x6 block a fifth as opaque, dithered to scattered pixels.
static FAINT: Block = Block {
    meta: meta("test/faint"),
    own: false,
    alpha: 0.2,
    big: true,
};
/// A 6x6 block half as opaque: a checkerboard, still a shape.
static HALF: Block = Block {
    meta: meta("test/half"),
    own: false,
    alpha: 0.5,
    big: true,
};

fn draw_into(effect: &'static Block, outline: Option<Outline>, frame: &mut Frame) {
    let inst = Instance::new(
        effect,
        Options {
            outline,
            ..Options::default()
        },
    );
    inst.draw(frame, Layer::Both, inst.placement());
}

fn draw(effect: &'static Block, outline: Option<Outline>) -> Frame {
    let mut f = Frame::new(10, 10, 0, 0);
    draw_into(effect, outline, &mut f);
    f
}

/// The pixels of colour `c` (any alpha), as (x, y).
fn of(f: &Frame, c: Rgba) -> Vec<(u32, u32)> {
    let rgb = c.pack() & 0x00ff_ffff;
    (0..f.h)
        .flat_map(|y| (0..f.w).map(move |x| (x, y)))
        .filter(|&(x, y)| {
            let w = f.color[(y * f.w + x) as usize];
            w >> 24 != 0 && w & 0x00ff_ffff == rgb
        })
        .collect()
}

fn on(c: Rgba, width: u32, corners: bool) -> Option<Outline> {
    Ring::new(c, width, corners).map(Outline::On)
}

#[test]
fn unset_draws_the_effect_as_designed() {
    let f = draw(&PLAIN, None);
    assert_eq!(of(&f, RED).len(), 4);
    assert!(
        f.color.iter().filter(|&&w| w != 0).count() == 4,
        "nothing but the block"
    );
}

#[test]
fn a_thin_ring_hugs_the_block_without_corners() {
    let f = draw(&PLAIN, on(WHITE, 1, false));
    let ring = of(&f, WHITE);
    assert_eq!(ring.len(), 8, "two pixels on each side of a 2x2 block");
    assert!(ring.contains(&(4, 3)) && ring.contains(&(6, 5)));
    assert!(!ring.contains(&(3, 3)), "the corners are cut");
    assert_eq!(of(&f, RED).len(), 4, "the block itself is untouched");
    assert!(
        ring.iter()
            .all(|&(x, y)| f.color[(y * 10 + x) as usize] >> 24 == 255),
        "as opaque as what it rings"
    );
}

#[test]
fn corners_fill_a_square_ring_and_width_reaches_further() {
    let square = of(&draw(&PLAIN, on(WHITE, 1, true)), WHITE);
    assert_eq!(square.len(), 12);
    assert!(square.contains(&(3, 3)));
    let wide = of(&draw(&PLAIN, on(WHITE, 2, false)), WHITE);
    assert!(wide.contains(&(4, 2)), "two pixels out");
    assert!(!wide.contains(&(2, 2)), "a rounded ring cuts its corners");
    assert!(wide.len() > square.len());
}

#[test]
fn only_what_the_instance_drew_is_ringed() {
    let mut f = Frame::new(10, 10, 0, 0);
    // something already on the frame, far from the block
    f.color[0] = MINE.pack();
    draw_into(&PLAIN, on(WHITE, 1, false), &mut f);
    let ring = of(&f, WHITE);
    assert_eq!(ring.len(), 8);
    assert!(!ring.contains(&(1, 0)) && !ring.contains(&(0, 1)));
}

#[test]
fn off_or_unset_rings_nothing() {
    assert!(of(&draw(&PLAIN, Some(Outline::Off)), WHITE).is_empty());
    assert_eq!(draw(&PLAIN, Some(Outline::Off)), draw(&PLAIN, None));
}

#[test]
fn an_effect_with_its_own_outline_recolours_or_drops_it_and_gets_no_second_ring() {
    let designed = draw(&OWN, None);
    assert_eq!(
        of(&designed, MINE).len(),
        12,
        "its own outline in its own colour"
    );
    let white = draw(&OWN, on(WHITE, 2, true));
    assert_eq!(
        of(&white, WHITE).len(),
        12,
        "the same outline, recoloured, not widened"
    );
    assert!(of(&white, MINE).is_empty());
    let off = draw(&OWN, Some(Outline::Off));
    assert!(of(&off, MINE).is_empty() && of(&off, WHITE).is_empty());
    assert_eq!(of(&off, RED).len(), 4);
}

#[test]
fn a_one_word_outline_is_off_a_preset_or_a_colour() {
    assert_eq!(Outline::parse("off"), Ok(Outline::Off));
    assert_eq!(Outline::parse("dark"), Ok(Outline::On(Ring::thin(DARK))));
    assert_eq!(Outline::parse("#f00"), Ok(Outline::On(Ring::thin(RED))));
    for (name, ring) in PRESETS {
        assert_eq!(Outline::parse(name), Ok(Outline::On(ring)));
    }
    let err = Outline::parse("gold")
        .err()
        .map(|e| e.to_string())
        .unwrap_or_default();
    assert!(err.contains("gold") && err.contains("thick"), "{err}");
}

#[test]
fn a_ring_is_one_to_three_pixels_wide() {
    assert!(Ring::new(WHITE, 0, false).is_none());
    assert!(Ring::new(WHITE, 4, false).is_none());
    assert!(Ring::new(WHITE, 3, true).is_some());
}

#[test]
fn scattered_pixels_such_as_smoke_get_no_ring_and_a_dithered_shape_still_does() {
    let faint = draw(&FAINT, on(WHITE, 1, false));
    assert!(!of(&faint, RED).is_empty(), "the faint block is drawn, as scattered pixels");
    assert!(of(&faint, WHITE).is_empty(), "but not ringed");
    let half = draw(&HALF, on(WHITE, 1, false));
    assert!(!of(&half, WHITE).is_empty(), "a checkerboard still reads as a shape");
}
