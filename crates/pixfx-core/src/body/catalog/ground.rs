//! Grounded movement: standing, walking, running, stopping, turning, crouching.

use super::super::clip::{Clip, Contact, Ease, Key, Mode};
use super::super::pose::{Head, Limb, Pose};
use super::super::rig::Limb4;
use super::{flat, foot_at, leg_length};

pub(super) fn by_name(name: &str) -> Option<Clip> {
    Some(match name {
        "idle" => idle(),
        "walk" => walk(),
        "run" => run(),
        "run_start" => run_start(),
        "run_stop" => run_stop(),
        "sprint" => sprint(),
        "skid" => skid(),
        "turn" => turn(),
        "crouch" => crouch(),
        "crouch_walk" => crouch_walk(),
        _ => return None,
    })
}

fn key(at: u32, pose: Pose, ease: Ease) -> Key {
    Key { at, pose, ease }
}

fn clip(frames: u32, mode: Mode, keys: Vec<Key>) -> Clip {
    Clip {
        frames,
        mode,
        keys,
        ..Clip::default()
    }
}

/// Standing, legs a little apart.
fn stand() -> Pose {
    flat(Pose {
        right_arm: Limb::new(5.0, 6.0, 12.0),
        left_arm: Limb::new(5.0, 6.0, 12.0),
        right_leg: Limb::new(0.0, 4.0, 0.0),
        left_leg: Limb::new(0.0, 4.0, 0.0),
        ..Pose::default()
    })
}

/// Breathing: the chest rises, the head dips, the arms ease.
fn idle() -> Clip {
    let breath = Pose {
        lean: 2.0,
        head: Head {
            nod: 4.0,
            turn: 0.0,
        },
        right_arm: Limb::new(6.0, 6.0, 18.0),
        left_arm: Limb::new(6.0, 6.0, 18.0),
        ..stand()
    };
    clip(
        8,
        Mode::Loop,
        vec![key(0, stand(), Ease::InOut), key(4, breath, Ease::InOut)],
    )
}

/// A stride: heel strike with the right foot `s` ahead and the left `s` behind, `h` below the pelvis, every foot
/// flat. `swing` is the free leg passing under the body.
struct Stride {
    h: f64,
    s: f64,
    lean: f64,
    arm_back: Limb,
    arm_front: Limb,
    swing: Limb,
}

impl Stride {
    fn contact(&self) -> Pose {
        let base = Pose {
            lean: self.lean,
            right_arm: self.arm_back,
            left_arm: self.arm_front,
            ..Pose::default()
        };
        let p = foot_at(base, Limb4::RightLeg, self.s, self.h);
        foot_at(p, Limb4::LeftLeg, -self.s, self.h)
    }
    /// The right foot pushing off `s` behind, the left swung through ahead.
    fn toe_off(&self) -> Pose {
        let base = Pose {
            lean: self.lean,
            right_arm: self.arm_front,
            left_arm: self.arm_back,
            left_leg: Limb::new(self.swing.raise + 15.0, 0.0, self.swing.bend),
            ..Pose::default()
        };
        foot_at(base, Limb4::RightLeg, -self.s, self.h)
    }
    /// The right foot under the pelvis, the left swinging past.
    fn passing(&self) -> Pose {
        let mid = Limb::new(0.0, 6.0, 15.0);
        let base = Pose {
            lean: self.lean,
            right_arm: mid,
            left_arm: mid,
            left_leg: self.swing,
            ..Pose::default()
        };
        foot_at(base, Limb4::RightLeg, 0.0, self.h)
    }
}

/// A walking or running cycle from a stride: the right foot planted from frame 0 for `stance` frames, the left
/// from half the cycle; the root moves at the speed that keeps a planted foot still.
fn cycle(st: &Stride, frames: u32, stance: u32, flight: Option<Pose>) -> Clip {
    let half = frames / 2;
    let (contact, passing) = (st.contact(), st.passing());
    let mut keys = vec![
        key(0, contact, Ease::Linear),
        key(stance / 2, passing, Ease::Linear),
        key(half, contact.mirrored(), Ease::Linear),
        key(half + stance / 2, passing.mirrored(), Ease::Linear),
    ];
    if stance < half {
        keys.push(key(stance, st.toe_off(), Ease::Linear));
        keys.push(key(half + stance, st.toe_off().mirrored(), Ease::Linear));
    }
    if let Some(air) = flight {
        keys.push(key(half - 1, air, Ease::Linear));
        keys.push(key(frames - 1, air.mirrored(), Ease::Linear));
    }
    // a key on the same frame as an earlier one is dropped: the contact wins over a passing pose at frame 0
    keys.sort_by_key(|k| k.at);
    keys.dedup_by_key(|k| k.at);
    let speed = 2.0 * st.s / f64::from(stance);
    Clip {
        frames,
        mode: Mode::Loop,
        keys,
        root: vec![(0, 0.0, 0.0), (frames, speed * f64::from(frames), 0.0)],
        contacts: vec![
            Contact {
                limb: Limb4::RightLeg,
                from: 0,
                to: stance,
            },
            Contact {
                limb: Limb4::LeftLeg,
                from: half,
                to: (half + stance) % frames,
            },
        ],
        ..Clip::default()
    }
}

fn walk() -> Clip {
    let l = leg_length();
    let st = Stride {
        h: 0.93 * l,
        s: 0.33 * l,
        lean: 3.0,
        arm_back: Limb::new(-18.0, 8.0, 15.0),
        arm_front: Limb::new(18.0, 8.0, 20.0),
        swing: Limb::new(10.0, 0.0, 55.0),
    };
    cycle(&st, 8, 4, None)
}

fn run_stride() -> Stride {
    let l = leg_length();
    Stride {
        h: 0.9 * l,
        s: 0.28 * l,
        lean: 12.0,
        arm_back: Limb::new(-30.0, 10.0, 80.0),
        arm_front: Limb::new(40.0, 10.0, 90.0),
        swing: Limb::new(35.0, 0.0, 80.0),
    }
}

fn run() -> Clip {
    let st = run_stride();
    let air = Pose {
        lean: 12.0,
        lift: 2.0,
        right_arm: Limb::new(-20.0, 10.0, 80.0),
        left_arm: Limb::new(30.0, 10.0, 90.0),
        right_leg: Limb::new(-35.0, 0.0, 60.0),
        left_leg: Limb::new(40.0, 0.0, 50.0),
        ..Pose::default()
    };
    cycle(&st, 8, 2, Some(flat(air)))
}

fn sprint() -> Clip {
    let l = leg_length();
    let st = Stride {
        h: 0.88 * l,
        s: 0.33 * l,
        lean: 22.0,
        arm_back: Limb::new(-45.0, 10.0, 90.0),
        arm_front: Limb::new(60.0, 10.0, 100.0),
        swing: Limb::new(45.0, 0.0, 90.0),
    };
    let air = Pose {
        lean: 22.0,
        lift: 3.0,
        right_arm: Limb::new(-35.0, 10.0, 90.0),
        left_arm: Limb::new(50.0, 10.0, 100.0),
        right_leg: Limb::new(-40.0, 0.0, 70.0),
        left_leg: Limb::new(50.0, 0.0, 40.0),
        ..Pose::default()
    };
    cycle(&st, 6, 1, Some(flat(air)))
}

fn crouch_walk() -> Clip {
    let l = leg_length();
    let st = Stride {
        h: 0.72 * l,
        s: 0.3 * l,
        lean: 25.0,
        arm_back: Limb::new(20.0, 10.0, 50.0),
        arm_front: Limb::new(40.0, 10.0, 60.0),
        swing: Limb::new(40.0, 0.0, 100.0),
    };
    cycle(&st, 8, 4, None)
}

fn crouched() -> Pose {
    flat(Pose {
        lean: 30.0,
        right_arm: Limb::new(40.0, 10.0, 40.0),
        left_arm: Limb::new(20.0, 15.0, 40.0),
        right_leg: Limb::new(75.0, 6.0, 120.0),
        left_leg: Limb::new(60.0, 6.0, 120.0),
        ..Pose::default()
    })
}

fn crouch() -> Clip {
    let low = Pose {
        lean: 33.0,
        head: Head {
            nod: 3.0,
            turn: 0.0,
        },
        ..crouched()
    };
    clip(
        6,
        Mode::Loop,
        vec![key(0, crouched(), Ease::InOut), key(3, low, Ease::InOut)],
    )
}

fn run_start() -> Clip {
    let gather = flat(Pose {
        lean: 20.0,
        right_arm: Limb::new(-30.0, 10.0, 20.0),
        left_arm: Limb::new(-25.0, 10.0, 20.0),
        right_leg: Limb::new(30.0, 4.0, 50.0),
        left_leg: Limb::new(10.0, 4.0, 40.0),
        ..Pose::default()
    });
    let push = flat(Pose {
        lean: 15.0,
        right_arm: Limb::new(-30.0, 10.0, 80.0),
        left_arm: Limb::new(40.0, 10.0, 90.0),
        right_leg: Limb::new(45.0, 0.0, 70.0),
        left_leg: Limb::new(-25.0, 0.0, 5.0),
        ..Pose::default()
    });
    clip(
        4,
        Mode::Once,
        vec![
            key(0, stand(), Ease::InOut),
            key(1, gather, Ease::Linear),
            key(2, push, Ease::Linear),
            key(3, run_stride().contact(), Ease::Linear),
        ],
    )
}

fn run_stop() -> Clip {
    let brake = flat(Pose {
        lean: -8.0,
        right_arm: Limb::new(30.0, 15.0, 40.0),
        left_arm: Limb::new(20.0, 15.0, 40.0),
        right_leg: Limb::new(35.0, 4.0, 0.0),
        left_leg: Limb::new(-15.0, 4.0, 45.0),
        ..Pose::default()
    });
    let settle = flat(Pose {
        lean: -3.0,
        right_leg: Limb::new(15.0, 4.0, 5.0),
        left_leg: Limb::new(-5.0, 4.0, 20.0),
        ..stand()
    });
    let mut c = clip(
        5,
        Mode::Once,
        vec![
            key(0, run_stride().contact(), Ease::Out),
            key(1, brake, Ease::InOut),
            key(3, settle, Ease::InOut),
            key(4, stand(), Ease::Linear),
        ],
    );
    c.root = vec![(0, 0.0, 0.0), (1, 5.0, 0.0), (3, 7.0, 0.0), (4, 7.0, 0.0)];
    c
}

fn skid() -> Clip {
    let slide = |lean: f64| {
        flat(Pose {
            lean,
            right_arm: Limb::new(70.0, 25.0, 30.0),
            left_arm: Limb::new(50.0, 25.0, 30.0),
            right_leg: Limb::new(40.0, 4.0, 0.0),
            left_leg: Limb::new(-5.0, 4.0, 60.0),
            ..Pose::default()
        })
    };
    let rise = flat(Pose {
        lean: -10.0,
        right_leg: Limb::new(20.0, 4.0, 10.0),
        left_leg: Limb::new(-5.0, 4.0, 30.0),
        ..stand()
    });
    let mut c = clip(
        5,
        Mode::Once,
        vec![
            key(0, slide(-18.0), Ease::Linear),
            key(2, slide(-24.0), Ease::InOut),
            key(4, rise, Ease::Linear),
        ],
    );
    c.root = vec![(0, 0.0, 0.0), (2, 7.0, 0.0), (4, 9.0, 0.0)];
    c
}

/// Turning round on the spot: the game faces her the other way when it ends.
fn turn() -> Clip {
    let mid = flat(Pose {
        spin: 60.0,
        lean: 5.0,
        right_leg: Limb::new(5.0, 4.0, 15.0),
        left_leg: Limb::new(0.0, 4.0, 15.0),
        ..stand()
    });
    let done = Pose {
        spin: 180.0,
        ..stand()
    };
    clip(
        6,
        Mode::Once,
        vec![
            key(0, stand(), Ease::In),
            key(2, mid, Ease::Out),
            key(5, done, Ease::Linear),
        ],
    )
}
