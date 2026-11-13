use pixfx_fighter::body::{Arena, Blow, Body, BodySpec, Doing, Drive, Event, Motion, separate};
use pixfx_fighter::pose::Pose;

const ARENA: Arena = Arena {
    left: 0.0,
    right: 200.0,
    top: 96.0,
};
const DT: f64 = 1.0 / 60.0;

fn standing(foe_x: f64) -> Drive {
    Drive {
        alive: true,
        acting: false,
        meditating: false,
        guarding: false,
        chilled: false,
        intent: 0.0,
        motion: Motion::Gravity,
        foe_x,
    }
}

/// One step the way the duel runs it; returns the pose and what the screen should feel.
fn run(body: &mut Body, roll: &mut dyn FnMut() -> f64) -> (Pose, Vec<Event>) {
    body.tick(DT, true);
    let events = body.step(DT, &standing(150.0), &ARENA, roll);
    let pose = body.pose_of(Doing::Idle, true);
    body.set_pose(pose);
    (pose, events)
}

#[test]
fn a_heavy_blow_throws_a_fighter_who_bounces_lies_and_gets_up_untouchable_until_just_after() {
    let mut body = Body::new(100.0);
    body.hit(Blow {
        power: 22.0,
        dir: -1.0,
        freeze: false,
        flying: false,
    });
    assert!(
        body.reeling() && !body.untouchable(),
        "thrown, it can still be juggled once"
    );
    body.hit(Blow {
        power: 10.0,
        dir: -1.0,
        freeze: false,
        flying: false,
    });
    assert!(body.untouchable(), "juggled once, blows pass through it");

    let mut no_quick_rise = || 0.99;
    let mut poses = Vec::new();
    let mut lands = 0;
    let mut steps = 0;
    while body.reeling() || body.wake > 0.0 {
        let (pose, events) = run(&mut body, &mut no_quick_rise);
        assert!(
            body.untouchable() || body.down.is_none(),
            "untouchable from landing until up"
        );
        lands += events
            .iter()
            .filter(|e| matches!(e, Event::Land { .. }))
            .count();
        if poses.last() != Some(&pose) {
            poses.push(pose);
        }
        steps += 1;
        assert!(steps < 600, "it gets up");
    }
    assert_eq!(
        poses,
        [
            Pose::Launched,
            Pose::Fall,
            Pose::Down,
            Pose::Sit,
            Pose::Kneel,
            Pose::Crouch,
            Pose::Land,
            Pose::Idle
        ]
    );
    assert_eq!(
        lands, 2,
        "the floor is felt on the bounce and on the landing"
    );
    assert!(
        !body.untouchable() && body.open_in() == 0.0,
        "after the grace it can be hit again"
    );
    assert!(body.x < 100.0, "thrown the way the blow pushed");
}

#[test]
fn a_light_blow_staggers_in_place_and_bodies_against_a_wall_part_the_other_way() {
    let mut body = Body::new(100.0);
    body.hit(Blow {
        power: 8.0,
        dir: 1.0,
        freeze: false,
        flying: false,
    });
    assert!(body.down.is_none(), "a light blow keeps it on its feet");
    assert!((body.out_for() - (0.24 + 8.0 * 0.012)).abs() < 1e-12);
    let (pose, _) = run(&mut body, &mut || 0.0);
    assert_eq!(pose, Pose::Hurt);
    assert!(body.x > 100.0, "pushed back");

    let s = BodySpec::default();
    let mut a = Body::new(s.half_width);
    let mut b = Body::new(s.half_width + 4.0);
    b.vx = -50.0;
    separate(&mut a, &mut b, &ARENA);
    assert_eq!(a.x, s.half_width, "the wall holds the one against it");
    assert_eq!(
        b.x - a.x,
        2.0 * (s.half_width + s.room),
        "the other takes the whole push"
    );
    assert_eq!(b.vx, 0.0, "and stops walking into it");
}

#[test]
fn a_body_built_lighter_and_floatier_is_thrown_by_a_blow_that_only_staggers_the_stock_one_and_flies_longer()
 {
    let blow = Blow {
        power: 8.0,
        dir: 1.0,
        freeze: false,
        flying: false,
    };
    let stock = BodySpec::default();
    let light = BodySpec {
        launch_damage: 6.0,
        gravity: stock.gravity / 2.0,
        ..stock
    };
    let mut a = Body::new(100.0);
    let mut b = Body::with_spec(100.0, light);
    a.hit(blow);
    b.hit(blow);
    assert!(
        a.down.is_none() && b.down.is_some(),
        "only the lighter one is thrown"
    );
    assert!(blow.heavy(&light) && !blow.heavy(&stock));

    // the same jump hangs longer under less gravity
    let airtime = |spec: BodySpec| {
        let mut body = Body::with_spec(100.0, spec);
        body.vy = spec.jump_speed;
        let mut steps = 0;
        while steps == 0 || body.air > 0.0 {
            body.step(DT, &standing(150.0), &ARENA, &mut || 0.0);
            steps += 1;
        }
        steps
    };
    assert!(airtime(light) > airtime(stock) + 20);
}
