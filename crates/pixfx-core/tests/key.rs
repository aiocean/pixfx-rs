//! A clip key is equal exactly when everything that shapes an effect's frames is equal.

use std::collections::BTreeMap;

use pixfx_core::{ClipKey, OptValue, Options, Placement};
use pixfx_draw::Rgba;

fn place() -> Placement {
    Placement {
        x: 10.0,
        y: 20.0,
        flip: false,
        angle: 0.0,
        light: 1.0,
    }
}

fn key(opts: &Options, margin: u32, place: Placement) -> ClipKey {
    ClipKey::new("fire/candle", opts, margin, place)
}

#[test]
fn equal_inputs_give_equal_keys() {
    let opts = Options {
        palette: Some(vec![Rgba(1, 2, 3, 255)]),
        ..Options::default()
    };
    assert_eq!(key(&opts, 4, place()), key(&opts.clone(), 4, place()));
    assert_eq!(
        key(&opts, 4, place()).to_string(),
        key(&opts, 4, place()).to_string()
    );
}

#[test]
fn every_input_changes_the_key() -> Result<(), String> {
    let base = Options::default();
    let mut extra = BTreeMap::new();
    extra.insert("height".to_string(), OptValue::Num(2.0));
    let variants: Vec<(&str, Options)> = vec![
        (
            "x",
            Options {
                x: 1.0,
                ..base.clone()
            },
        ),
        (
            "y",
            Options {
                y: 1.0,
                ..base.clone()
            },
        ),
        (
            "flip",
            Options {
                flip: true,
                ..base.clone()
            },
        ),
        (
            "angle",
            Options {
                angle: 0.5,
                ..base.clone()
            },
        ),
        (
            "seed",
            Options {
                seed: Some(3),
                ..base.clone()
            },
        ),
        (
            "light",
            Options {
                light: 0.5,
                ..base.clone()
            },
        ),
        (
            "palette",
            Options {
                palette: Some(vec![Rgba(9, 9, 9, 255)]),
                ..base.clone()
            },
        ),
        (
            "extra",
            Options {
                extra,
                ..base.clone()
            },
        ),
    ];
    let k0 = key(&base, 0, place());
    for (name, opts) in &variants {
        if key(opts, 0, place()) == k0 {
            return Err(format!("{name} does not change the key"));
        }
    }
    if key(&base, 1, place()) == k0 {
        return Err("margin does not change the key".into());
    }
    if ClipKey::new("fire/campfire", &base, 0, place()) == k0 {
        return Err("the effect does not change the key".into());
    }
    let p = place();
    let places = [
        ("place x", Placement { x: 11.0, ..p }),
        ("place y", Placement { y: 21.0, ..p }),
        ("place flip", Placement { flip: true, ..p }),
        ("place angle", Placement { angle: 0.1, ..p }),
        ("place light", Placement { light: 0.5, ..p }),
    ];
    for (name, q) in places {
        if key(&base, 0, q) == k0 {
            return Err(format!("{name} does not change the key"));
        }
    }
    Ok(())
}

#[test]
fn a_key_reads_back_from_its_hex_form() {
    let opts = Options {
        palette: Some(vec![Rgba(1, 2, 3, 255)]),
        ..Options::default()
    };
    let k = key(&opts, 4, place());
    assert_eq!(k.to_string().parse::<ClipKey>(), Ok(k.clone()));
    assert!("".parse::<ClipKey>().is_err());
    assert!("0".parse::<ClipKey>().is_err());
    assert!("zz".parse::<ClipKey>().is_err());
    let other_version = format!("01{}", &k.to_string()[2..]);
    assert!(other_version.parse::<ClipKey>().is_err());
}
