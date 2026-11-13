use pixfx_core::{Frame, Rgba};
use pixfx_fighter::look::Look;
use pixfx_fighter::paint::{Act, Appearance, Windup, draw};
use pixfx_fighter::pose::{Gather, Pose};

const W: u32 = 96;
const GROUND: f64 = 70.0;

fn standing(pose: Pose) -> Appearance {
    Appearance {
        pose,
        pose_t: 0.0,
        face: 1.0,
        x: 48.0,
        air: 0.0,
        vx: 0.0,
        vy: 0.0,
        out: false,
        flash: false,
        frozen: false,
        shocked: false,
        hit_t: 9.0,
        hit_power: 0.0,
        untouchable: false,
        dodged: 0.0,
        act: None,
        jolt: [0, 0],
    }
}

fn drawn(look: &Look, a: &Appearance) -> Frame {
    let mut frame = Frame::new(W, 80, 0, 0);
    draw(&mut frame, look, a, GROUND);
    frame
}

fn at(f: &Frame, x: u32, y: u32) -> Rgba {
    Rgba::unpack(f.color.get((y * f.w + x) as usize).copied().unwrap_or(0))
}

fn row_has(f: &Frame, y: u32, c: Rgba) -> bool {
    (0..f.w).any(|x| at(f, x, y) == c)
}

#[test]
fn a_fighter_stands_on_the_ground_rises_with_air_and_turns_to_face_its_way() {
    let look = Look::thunder();
    let idle = standing(Pose::Idle);
    let on_ground = drawn(&look, &idle);
    assert!(
        row_has(&on_ground, 70, look.boot),
        "boots on the ground row"
    );
    assert!(
        !row_has(&on_ground, 71, look.boot),
        "nothing but outline below the ground"
    );

    let up = drawn(&look, &Appearance { air: 10.0, ..idle });
    assert!(
        row_has(&up, 60, look.boot) && !row_has(&up, 70, look.boot),
        "ten pixels up, boots ten rows higher"
    );

    let left = drawn(&look, &Appearance { face: -1.0, ..idle });
    for y in 0..80 {
        for x in 1..W {
            assert_eq!(
                at(&left, x, y),
                at(&on_ground, W - x, y),
                "facing left mirrors about its feet at ({x}, {y})"
            );
        }
    }
}

#[test]
fn only_a_gathering_fighter_spills_a_translucent_aura() {
    let look = Look::frost();
    let translucent = |f: &Frame| {
        f.color
            .iter()
            .filter(|&&c| (1..255).contains(&(c >> 24)))
            .count()
    };
    assert_eq!(translucent(&drawn(&look, &standing(Pose::Idle))), 0);

    let act = Act {
        how: Gather::Summon,
        dive: false,
        windup: Some(Windup {
            progress: 0.5,
            long: true,
        }),
    };
    let gathering = drawn(
        &look,
        &Appearance {
            act: Some(act),
            ..standing(Pose::Channel)
        },
    );
    assert!(translucent(&gathering) > 200, "an aura round the caster");
}

#[test]
fn an_untouchable_fighter_blinks_and_a_touchable_one_does_not() {
    let look = Look::thunder();
    let at_t = |untouchable: bool, pose_t: f64| {
        let a = Appearance {
            untouchable,
            pose_t,
            ..standing(Pose::Idle)
        };
        drawn(&look, &a).color
    };
    // one fifteenth of a second in, a blink is on its faint beat
    let beat = 1.5 / 15.0;
    assert_eq!(
        at_t(false, beat),
        at_t(false, beat),
        "drawing is deterministic"
    );
    assert_ne!(
        at_t(true, beat),
        at_t(false, beat),
        "untouchable: faint on the off beat"
    );
    assert_eq!(
        at_t(true, 0.5 / 15.0),
        at_t(false, 0.5 / 15.0),
        "untouchable: whole on the on beat"
    );
    let out = |untouchable: bool| {
        let a = Appearance {
            untouchable,
            out: true,
            pose_t: beat,
            ..standing(Pose::Down)
        };
        drawn(&look, &a).color
    };
    assert_eq!(
        out(true),
        out(false),
        "a knocked-out fighter does not blink"
    );
}

/// The drawn body's top row, bottom row and width, from where it covers the frame.
fn extent(f: &Frame) -> (u32, u32, u32) {
    let covered = |x: u32, y: u32| f.color.get((y * f.w + x) as usize).is_some_and(|&c| c != 0);
    let rows: Vec<u32> = (0..f.h)
        .filter(|&y| (0..f.w).any(|x| covered(x, y)))
        .collect();
    let cols: Vec<u32> = (0..f.w)
        .filter(|&x| (0..f.h).any(|y| covered(x, y)))
        .collect();
    let span = |v: &[u32]| {
        (
            v.first().copied().unwrap_or(0),
            v.last().copied().unwrap_or(0),
        )
    };
    let ((top, bottom), (left, right)) = (span(&rows), span(&cols));
    (top, bottom, right - left + 1)
}

#[test]
fn a_body_stretches_as_it_flies_fast_and_squashes_as_it_lands_feet_on_the_floor() {
    let look = Look::thunder();
    let jump = |vy: f64| {
        extent(&drawn(
            &look,
            &Appearance {
                vy,
                ..standing(Pose::Jump)
            },
        ))
    };
    let (slow_top, slow_bottom, slow_w) = jump(0.0);
    let (fast_top, fast_bottom, fast_w) = jump(220.0);
    assert!(
        fast_top < slow_top,
        "taller when fast: {fast_top} vs {slow_top}"
    );
    assert!(
        fast_w <= slow_w,
        "no wider when stretched: {fast_w} vs {slow_w}"
    );
    assert_eq!(fast_bottom, slow_bottom, "feet stay where they were");
    assert_eq!(
        jump(-220.0),
        (fast_top, fast_bottom, fast_w),
        "falling stretches as rising does"
    );

    let land = |pose_t: f64| {
        extent(&drawn(
            &look,
            &Appearance {
                pose_t,
                ..standing(Pose::Land)
            },
        ))
    };
    let (hit_top, hit_bottom, hit_w) = land(0.0);
    let (after_top, after_bottom, after_w) = land(0.3);
    assert!(
        hit_top > after_top,
        "shorter as it lands: {hit_top} vs {after_top}"
    );
    assert!(hit_w > after_w, "wider as it lands: {hit_w} vs {after_w}");
    assert_eq!(
        hit_bottom, after_bottom,
        "the squash keeps its feet on the floor"
    );
}
