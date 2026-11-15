//! The crate's value types and its seeded random.

use pixfx_mind::Rng;

#[test]
fn rng_replays_a_seed() {
    let mut a = Rng::new(7);
    let mut b = Rng::new(7);
    let mut c = Rng::new(8);
    let mut differs = false;
    for _ in 0..100 {
        let (x, y, z) = (a.next(), b.next(), c.next());
        assert_eq!(x, y);
        assert!((0.0..1.0).contains(&x));
        differs |= x != z;
    }
    assert!(differs);
}

/// The duel's own generator (`pixfx_duel::rng`), bit for bit.
#[test]
fn rng_matches_mulberry32() {
    assert_eq!(Rng::new(1).next(), 0.6270739405881613);
}

#[cfg(feature = "serde")]
mod serde_shapes {
    use pixfx_mind::{ActPhase, Choice, Doing, Sense, SkillKind};

    const SENSE: &str = r#"{
        "time": 1.5,
        "me": {"x": 40, "air": 0, "face": 1, "hp": 80, "qi": 60, "rage": 1, "rageFull": 4, "flying": false,
               "hover": 0, "vx": 0, "canAct": true, "silenced": false, "sealed": false, "barrier": false,
               "shield": false, "seekerOut": false},
        "foe": {"x": 60, "air": 0, "flying": false, "hp": 70, "qi": 50,
                "doing": {"type": "act", "kind": "melee", "phase": "windup", "t": 0.1, "windup": 0.2, "recover": 0.32, "reach": 24, "damage": 5},
                "outFor": 0, "openIn": 0, "frozen": 0, "shocked": 0, "barrier": false, "ready": [true]},
        "arena": {"left": 0, "right": 220, "top": 96},
        "threats": [{"age": 0.3, "damage": 13, "shape": {"type": "projectile", "x": 10, "y": 14, "vx": 230}}],
        "ready": [true, false]
    }"#;

    #[test]
    fn sense_reads_camel_case() {
        let sense: Sense = serde_json::from_str(SENSE).unwrap_or_else(|e| unreachable!("{e}"));
        assert!(sense.me.can_act);
        assert_eq!(
            sense.foe.doing,
            Doing::Act {
                kind: SkillKind::Melee,
                phase: ActPhase::Windup,
                t: 0.1,
                windup: 0.2,
                recover: 0.32,
                reach: 24.0,
                damage: 5.0
            }
        );
        assert_eq!(sense.ready, vec![true, false]);
    }

    #[test]
    fn a_misspelt_field_is_refused() {
        let bad = SENSE.replace("canAct", "canAkt");
        assert!(serde_json::from_str::<Sense>(&bad).is_err());
    }

    #[test]
    fn choice_writes_tagged() {
        let json = serde_json::to_string(&Choice::Move { dir: -1.0 }).unwrap_or_default();
        assert_eq!(json, r#"{"type":"move","dir":-1.0}"#);
        let cast = serde_json::to_string(&Choice::Cast {
            skill: "bolt".into(),
            target: None,
        })
        .unwrap_or_default();
        assert_eq!(cast, r#"{"type":"cast","skill":"bolt","target":null}"#);
    }
}
