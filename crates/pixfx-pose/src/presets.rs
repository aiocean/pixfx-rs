//! Ready poses for a sword fighter, the sword in her right hand. Start from one and change a few angles rather than
//! writing a pose from the rest pose. Cyclic ones (`walk`, `run`) take a phase: 0 and 1 are the same moment, 0.5
//! the same moment on the other foot.

use crate::pose::{Head, Limb, Pose, Side, Sword};

/// Every preset name [`by_name`] knows.
pub const NAMES: [&str; 14] = [
    "rest", "idle", "walk", "run", "guard", "windup", "slash", "thrust", "hurt", "kneel", "crouch",
    "jump", "down", "cast",
];

/// A preset by name, `phase` used by the cyclic ones; `None` for a name not in [`NAMES`].
pub fn by_name(name: &str, phase: f64) -> Option<Pose> {
    Some(match name {
        "rest" => Pose::default(),
        "idle" => idle(),
        "walk" => walk(phase),
        "run" => run(phase),
        "guard" => guard(),
        "windup" => windup(),
        "slash" => slash(),
        "thrust" => thrust(),
        "hurt" => hurt(),
        "kneel" => kneel(),
        "crouch" => crouch(),
        "jump" => jump(),
        "down" => down(),
        "cast" => cast(),
        _ => return None,
    })
}

const fn sword(angle: f64) -> Option<Sword> {
    Some(Sword {
        hand: Side::Right,
        angle,
    })
}

/// Standing at ease, the sword held low and forward.
pub fn idle() -> Pose {
    Pose {
        right_arm: Limb::new(10.0, 8.0, 20.0),
        left_arm: Limb::new(-5.0, 10.0, 10.0),
        right_leg: Limb::new(0.0, 4.0, 0.0),
        left_leg: Limb::new(0.0, 4.0, 0.0),
        sword: sword(35.0),
        ..Pose::default()
    }
}

fn phase_angle(phase: f64) -> f64 {
    let p = if phase.is_finite() { phase } else { 0.0 };
    p.rem_euclid(1.0) * core::f64::consts::TAU
}

/// A walk cycle: at phase 0 the right leg is passing under the body swinging forward, at 0.25 it is furthest
/// forward.
pub fn walk(phase: f64) -> Pose {
    let t = phase_angle(phase);
    let leg = |t: f64| {
        let (s, c) = libm::sincos(t);
        Limb::new(25.0 * s, 3.0, 8.0 + 30.0 * c.max(0.0))
    };
    let s = libm::sin(t);
    Pose {
        lean: 4.0,
        right_leg: leg(t),
        left_leg: leg(t + core::f64::consts::PI),
        right_arm: Limb::new(10.0 - 15.0 * s, 8.0, 20.0),
        left_arm: Limb::new(25.0 * s, 8.0, 15.0),
        sword: sword(35.0),
        ..Pose::default()
    }
}

/// A run cycle, the same phases as [`walk`], longer strides and both feet off the ground mid-stride.
pub fn run(phase: f64) -> Pose {
    let t = phase_angle(phase);
    let leg = |t: f64| {
        let (s, c) = libm::sincos(t);
        Limb::new(45.0 * s, 3.0, 20.0 + 70.0 * c.max(0.0))
    };
    let s = libm::sin(t);
    Pose {
        lean: 15.0,
        lift: 3.0 * libm::sin(2.0 * t).abs(),
        right_leg: leg(t),
        left_leg: leg(t + core::f64::consts::PI),
        right_arm: Limb::new(20.0 - 35.0 * s, 10.0, 70.0),
        left_arm: Limb::new(40.0 * s, 10.0, 80.0),
        sword: sword(0.0),
        ..Pose::default()
    }
}

/// On guard: the sword raised in front, the free hand before the chest, feet apart.
pub fn guard() -> Pose {
    Pose {
        lean: 5.0,
        right_arm: Limb::new(110.0, 15.0, 30.0),
        left_arm: Limb::new(40.0, 5.0, 70.0),
        right_leg: Limb::new(15.0, 6.0, 15.0),
        left_leg: Limb::new(-15.0, 6.0, 10.0),
        sword: sword(20.0),
        ..Pose::default()
    }
}

/// The sword drawn back over the shoulder before a slash.
pub fn windup() -> Pose {
    Pose {
        lean: -5.0,
        twist: -20.0,
        right_arm: Limb::new(150.0, 10.0, 60.0),
        left_arm: Limb::new(50.0, 10.0, 40.0),
        right_leg: Limb::new(10.0, 6.0, 10.0),
        left_leg: Limb::new(-15.0, 6.0, 10.0),
        sword: sword(30.0),
        ..Pose::default()
    }
}

/// The end of a downward slash: a deep lunge, the blade low in front.
pub fn slash() -> Pose {
    Pose {
        lean: 20.0,
        twist: 25.0,
        right_arm: Limb::new(60.0, 10.0, 0.0),
        left_arm: Limb::new(-40.0, 15.0, 10.0),
        right_leg: Limb::new(45.0, 4.0, 50.0),
        left_leg: Limb::new(-35.0, 4.0, 5.0),
        sword: sword(-10.0),
        ..Pose::default()
    }
}

/// A straight thrust: arm and blade level, lunging.
pub fn thrust() -> Pose {
    Pose {
        lean: 15.0,
        twist: 20.0,
        right_arm: Limb::new(90.0, 0.0, 0.0),
        left_arm: Limb::new(-60.0, 10.0, 10.0),
        right_leg: Limb::new(45.0, 4.0, 50.0),
        left_leg: Limb::new(-35.0, 4.0, 5.0),
        sword: sword(0.0),
        ..Pose::default()
    }
}

/// Struck in the chest: the upper body and head thrown back, the free arm flung out.
pub fn hurt() -> Pose {
    Pose {
        lean: -20.0,
        head: Head {
            nod: -15.0,
            turn: 0.0,
        },
        right_arm: Limb::new(20.0, 20.0, 30.0),
        left_arm: Limb::new(-40.0, 35.0, 30.0),
        right_leg: Limb::new(10.0, 5.0, 10.0),
        left_leg: Limb::new(-10.0, 5.0, 5.0),
        sword: sword(30.0),
        ..Pose::default()
    }
}

/// Down on the left knee, the right foot planted in front, the sword point on the ground.
pub fn kneel() -> Pose {
    Pose {
        right_arm: Limb::new(30.0, 10.0, 30.0),
        left_arm: Limb::new(20.0, 10.0, 60.0),
        right_leg: Limb::new(90.0, 4.0, 90.0),
        left_leg: Limb::new(0.0, 4.0, 90.0),
        sword: sword(60.0),
        ..Pose::default()
    }
}

/// Crouched low on both feet, ready to spring.
pub fn crouch() -> Pose {
    Pose {
        lean: 30.0,
        right_arm: Limb::new(40.0, 10.0, 40.0),
        left_arm: Limb::new(20.0, 15.0, 40.0),
        right_leg: Limb::new(75.0, 6.0, 120.0),
        left_leg: Limb::new(60.0, 6.0, 120.0),
        sword: sword(40.0),
        ..Pose::default()
    }
}

/// In the air, knees tucked, arms up.
pub fn jump() -> Pose {
    Pose {
        lift: 5.0,
        right_arm: Limb::new(100.0, 25.0, 40.0),
        left_arm: Limb::new(120.0, 30.0, 30.0),
        right_leg: Limb::new(45.0, 4.0, 80.0),
        left_leg: Limb::new(20.0, 4.0, 90.0),
        sword: sword(-20.0),
        ..Pose::default()
    }
}

/// Knocked down, lying on her back, arms out, the sword still in hand.
pub fn down() -> Pose {
    Pose {
        lie: 90.0,
        right_arm: Limb::new(15.0, 40.0, 10.0),
        left_arm: Limb::new(10.0, 35.0, 15.0),
        right_leg: Limb::new(5.0, 5.0, 10.0),
        left_leg: Limb::new(0.0, 5.0, 0.0),
        sword: sword(0.0),
        ..Pose::default()
    }
}

/// Both hands pushed forward casting a spell, no sword.
pub fn cast() -> Pose {
    Pose {
        lean: 8.0,
        right_arm: Limb::new(85.0, 8.0, 10.0),
        left_arm: Limb::new(80.0, 8.0, 15.0),
        right_leg: Limb::new(15.0, 6.0, 15.0),
        left_leg: Limb::new(-15.0, 6.0, 5.0),
        ..Pose::default()
    }
}
