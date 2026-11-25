//! The painter methods and shapes ported last (`arc`, `beam`, `polyline`, `text`, `measure`, `sprite` from strings,
//! `mask`, `outline`, `star`, `ngon`) paint the frames pixelfx paints. Each expected value is what pixelfx's own
//! painter gives for the same calls in node: an FNV-1a hash over the frame's colour then light words.

use pixfx_draw::color::{Rgba, WHITE, hex};
use pixfx_draw::shapes::{ngon, star, trail};
use pixfx_draw::{Align, BIG, Cells, Frame, MINI, Painter, SpriteOpts, TextOpts};

const A: Rgba = Rgba::hex24(0xff8a2a);
const B: Rgba = Rgba::hex24(0x3cc8ff);
const C: Rgba = Rgba::hex24(0x7a6cff);
const PLAIN: (f64, f64, bool, f64) = (0.0, 0.0, false, 0.0);
const TURNED: (f64, f64, bool, f64) = (3.5, -2.0, true, 0.3);

fn fnv(f: &Frame) -> u32 {
    let mut h: u32 = 0x811c_9dc5;
    for w in f.color.iter().chain(f.light.iter()) {
        for s in [0, 8, 16, 24] {
            h ^= (w >> s) & 255;
            h = h.wrapping_mul(0x0100_0193);
        }
    }
    h
}

fn run(place: (f64, f64, bool, f64), draw: impl FnOnce(&mut Painter)) -> u32 {
    let mut f = Frame::new(96, 64, -48, -32);
    {
        let mut p = Painter::new(&mut f);
        p.place(place.0, place.1, place.2, place.3);
        draw(&mut p);
    }
    fnv(&f)
}

fn arcs(p: &mut Painter) {
    p.arc(
        0.0,
        0.0,
        20.0,
        5.0,
        0.4,
        3.9,
        |u: f64, v: f64| if u > 0.3 && v > 0.5 { A } else { B },
        1.0,
    );
    p.arc(
        4.0,
        2.0,
        14.0,
        |u: f64| 1.0 + 4.0 * libm::sin(core::f64::consts::PI * u),
        -2.0,
        1.5,
        C,
        |u: f64, _: f64| 1.0 - u,
    );
}

fn beams(p: &mut Painter) {
    p.beam(
        -20.0,
        -10.0,
        25.0,
        12.0,
        5.0,
        |u: f64, v: f64| {
            if v < 0.45 {
                WHITE
            } else if u > 0.8 {
                A
            } else {
                B
            }
        },
        1.0,
    );
    p.beam(3.0, 3.0, 3.0, 3.0, 3.0, C, 0.5);
}

fn polylines(p: &mut Painter) {
    let pts = [(-30.0, -5.0), (-10.0, 12.0), (5.0, -14.0), (28.0, 9.0)];
    p.polyline(&pts, |u: f64| if u < 0.4 { WHITE } else { A }, 0.6);
}

fn texts(p: &mut Painter) {
    let centred = TextOpts {
        align: Align::Center,
        outline: Some(C),
        shadow: Some(B),
        ..TextOpts::default()
    };
    p.text("Lv 12!", 0.0, -20.0, A, &centred);
    let big = TextOpts {
        font: BIG,
        scale: 2.0,
        align: Align::Right,
        a: 0.8,
        ..TextOpts::default()
    };
    p.text(
        "POW",
        30.0,
        0.0,
        |_: f64, gy: f64| if gy < 4.0 { A } else { B },
        &big,
    );
    p.text("ok?", -40.4, 10.5, WHITE, &TextOpts::default());
}

fn sprites(p: &mut Painter) {
    let rows = ["..##..", ".#oo#.", "#oxxo#", ".#oo#."];
    let pal = [
        ('#', hex("#222034").unwrap_or(Rgba::NONE)),
        ('o', A),
        ('x', Rgba::NONE),
    ];
    p.sprite_rows(&rows, &pal, -10.0, -8.0, &SpriteOpts::default());
    let big = SpriteOpts {
        scale: 2.0,
        a: 0.7,
        flip: true,
        flip_y: true,
        tint: None,
    };
    p.sprite_rows(&rows, &pal, 5.0, 2.0, &big);
    let tinted = SpriteOpts {
        tint: Some(B),
        ..SpriteOpts::default()
    };
    p.sprite_rows(&rows, &pal, -30.0, 10.0, &tinted);
}

fn masks(p: &mut Painter) {
    #[rustfmt::skip]
    let data = vec![
        0, 1, 1, 1, 0, 0, 0,
        1, 1, 1, 1, 1, 0, 0,
        1, 1, 0, 1, 1, 1, 0,
        0, 1, 1, 1, 1, 1, 1,
        0, 0, 1, 1, 1, 0, 0,
        0, 0, 0, 1, 0, 0, 0,
    ];
    let m = Cells { w: 7, h: 6, data };
    p.mask(
        &m,
        -12.0,
        -8.0,
        |mx: f64, my: f64| if (mx + my) % 2.0 != 0.0 { A } else { B },
        |mx: f64, _: f64| if mx < 5.0 { 1.0 } else { 0.4 },
    );
    p.outline(&m, -12.0, -8.0, C, 1.0);
    p.outline(
        &m,
        10.0,
        4.0,
        WHITE,
        |_: f64, my: f64| if my < 3.0 { 1.0 } else { 0.3 },
    );
}

#[test]
fn arc_matches_pixelfx() {
    assert_eq!(run(PLAIN, arcs), 686_759_659);
    assert_eq!(run(TURNED, arcs), 73_147_895);
}

#[test]
fn beam_matches_pixelfx() {
    assert_eq!(run(PLAIN, beams), 466_807_117);
    assert_eq!(run(TURNED, beams), 1_952_092_325);
}

#[test]
fn polyline_matches_pixelfx() {
    assert_eq!(run(PLAIN, polylines), 1_251_454_037);
    assert_eq!(run(TURNED, polylines), 1_036_395_171);
}

#[test]
fn text_matches_pixelfx() {
    assert_eq!(run(PLAIN, texts), 2_471_436_067);
    assert_eq!(run(TURNED, texts), 2_279_682_399);
    assert_eq!(Painter::measure("Lv 12!", MINI, 1.0), (20.0, 5.0));
    assert_eq!(Painter::measure("POW!", BIG, 3.0), (57.0, 21.0));
}

#[test]
fn sprite_rows_match_pixelfx() {
    assert_eq!(run(PLAIN, sprites), 2_995_436_121);
    assert_eq!(run(TURNED, sprites), 3_052_521_982);
}

#[test]
fn mask_and_outline_match_pixelfx() {
    assert_eq!(run(PLAIN, masks), 684_140_957);
    assert_eq!(run(TURNED, masks), 1_707_015_971);
}

#[test]
fn star_and_ngon_match_pixelfx() {
    #[rustfmt::skip]
    let js_star = [
        (1.0000000000000004, -5.0), (2.763355756877419, -0.42705098312484235),
        (7.657395614066075, -0.16311896062463171), (3.8531695488854605, 2.9270509831248424),
        (5.114496766047312, 7.663118960624632), (1.0000000000000002, 5.0),
        (-3.114496766047311, 7.663118960624632), (-1.8531695488854605, 2.9270509831248424),
        (-5.6573956140660755, -0.16311896062463083), (-0.7633557568774196, -0.4270509831248419),
    ];
    assert_eq!(
        star(1.0, 2.0, 3.0, 7.0, 5, -core::f64::consts::PI / 2.0),
        js_star
    );
    #[rustfmt::skip]
    let js_ngon = [
        (7.720211795395803, 2.7266356332907065), (-0.42296268912993673, 9.48148250245269),
        (-9.363583124501265, 3.8242258225875183), (-6.746015949546889, -6.426997957813526),
        (3.8123499677822856, -7.105346000517388),
    ];
    assert_eq!(ngon(-1.0, 0.5, 9.0, 5, 0.25), js_ngon);
    assert_eq!(
        trail(|t| (t, -t), 0.25, 4, 0.1),
        [
            (0.25, -0.25),
            (0.15, -0.15),
            (0.04999999999999999, -0.04999999999999999),
            (0.0, -0.0)
        ]
    );
}

#[test]
fn save_restore_and_nest_keep_placement() {
    let mut f = Frame::new(8, 8, 0, 0);
    let mut p = Painter::new(&mut f);
    p.place(2.0, 3.0, true, 0.0);
    let s = p.save();
    p.nest(1.0, 1.0, true);
    // mirrored twice: +x again; the origin moved one pixel left (mirrored) and one down
    assert_eq!((p.to_x(1.0, 0.0), p.to_y(0.0, 0.0)), (2.0, 4.0));
    p.fade = 0.2;
    p.restore(s);
    assert_eq!(
        (p.to_x(1.0, 0.0), p.to_y(0.0, 0.0), p.fade),
        (1.0, 3.0, 1.0)
    );
}
