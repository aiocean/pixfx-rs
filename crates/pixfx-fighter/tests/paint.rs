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
        out: false,
        flash: false,
        frozen: false,
        shocked: false,
        hit_t: 9.0,
        hit_power: 0.0,
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
