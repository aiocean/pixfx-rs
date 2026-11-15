//! The policy network: a weights file read strictly, a forward pass worked by hand, sampling that obeys the mask.

use pixfx_mind::policy::{ACTIONS, Net, OBS, sample};

/// A seeded network as bytes, with tensor `name` replaced by `f(old)`, or dropped when `f` gives `None`.
fn edited(name: &str, f: impl Fn(Vec<f32>) -> Option<Vec<f32>>) -> Vec<u8> {
    let mut tensors = Net::seeded(7).tensors();
    tensors.retain_mut(|(n, _, data)| {
        if n != name {
            return true;
        }
        match f(std::mem::take(data)) {
            Some(d) => {
                *data = d;
                true
            }
            None => false,
        }
    });
    Net::write(&tensors)
}

fn obs(k: f32) -> [f32; OBS] {
    let mut o = [0.0; OBS];
    for (i, v) in o.iter_mut().enumerate() {
        *v = libm::sinf(i as f32 * 0.37 + k) * 2.0;
    }
    o
}

#[test]
fn a_seeded_network_round_trips_through_its_bytes() {
    let net = Net::seeded(7);
    let back = Net::load(&net.to_bytes());
    assert!(
        back.as_ref()
            .is_ok_and(|b| b.logits(&obs(1.0)) == net.logits(&obs(1.0))),
        "{:?}",
        back.err()
    );
}

#[test]
fn a_file_with_a_missing_tensor_is_refused_by_name() {
    let err = Net::load(&edited("sem.w", |_| None))
        .err()
        .map(|e| e.to_string())
        .unwrap_or_default();
    assert!(err.contains("sem.w"), "{err}");
}

#[test]
fn a_tensor_of_the_wrong_shape_is_refused_by_name() {
    let bytes = edited("pi.b", |mut d| {
        d.pop();
        Some(d)
    });
    let err = Net::load(&bytes)
        .err()
        .map(|e| e.to_string())
        .unwrap_or_default();
    assert!(err.contains("pi.b"), "{err}");
}

#[test]
fn a_truncated_or_foreign_file_is_refused() {
    let bytes = Net::seeded(7).to_bytes();
    assert!(Net::load(&bytes[..bytes.len() - 3]).is_err());
    assert!(Net::load(&bytes[..10]).is_err());
    assert!(Net::load(b"PK\x03\x04 not a network").is_err());
}

#[test]
fn hand_set_weights_give_the_logits_worked_by_hand() {
    // the policy head reads only its bias, and its first row sums the 16 simplices (each sums to 1)
    let zero = |d: Vec<f32>| Some(vec![0.0; d.len()]);
    let mut tensors = Net::seeded(7).tensors();
    for (name, _, data) in tensors.iter_mut() {
        if name == "pi.w" {
            *data = zero(std::mem::take(data)).unwrap_or_default();
            for v in data.iter_mut().take(128) {
                *v = 1.0;
            }
        }
        if name == "pi.b" {
            *data = (0..ACTIONS).map(|i| i as f32).collect();
        }
    }
    let net = Net::load(&Net::write(&tensors));
    let logits = net
        .map(|n| n.logits(&obs(0.3)))
        .unwrap_or([f32::NAN; ACTIONS]);
    assert!((logits[0] - 16.0).abs() < 1e-4, "{}", logits[0]);
    for (i, l) in logits.iter().enumerate().skip(1) {
        assert_eq!(*l, i as f32);
    }
}

#[test]
fn sampling_never_picks_a_masked_action() {
    let net = Net::seeded(3);
    for m in 0..40_u32 {
        let mut legal = [false; ACTIONS];
        for (i, l) in legal.iter_mut().enumerate() {
            *l = (i as u32 * 7 + m * 13).is_multiple_of(5);
        }
        legal[0] = true;
        let logits = net.logits(&obs(m as f32));
        for k in 0..1000 {
            let i = sample(&logits, &legal, k as f32 / 1000.0);
            assert!(legal[i], "mask {m}, u {k}: picked {i}");
        }
    }
}

#[test]
fn sampling_with_one_legal_action_picks_it_even_against_huge_logits() {
    let mut logits = [80.0_f32; ACTIONS];
    logits[5] = -1e30;
    let mut legal = [false; ACTIONS];
    legal[5] = true;
    for u in [0.0, 0.5, 0.999_999] {
        assert_eq!(sample(&logits, &legal, u), 5);
    }
    assert_eq!(
        sample(&logits, &[false; ACTIONS], 0.5),
        0,
        "nothing legal: wait"
    );
}

#[test]
fn sampling_follows_the_softmax() {
    let mut logits = [0.0_f32; ACTIONS];
    logits[1] = libm::logf(3.0);
    let mut legal = [false; ACTIONS];
    legal[0] = true;
    legal[1] = true;
    // action 0 has a quarter of the mass
    assert_eq!(sample(&logits, &legal, 0.24), 0);
    assert_eq!(sample(&logits, &legal, 0.26), 1);
}

#[test]
fn logits_are_pinned_for_a_seeded_network() {
    let net = Net::seeded(11);
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for k in 0..8 {
        for l in net.logits(&obs(k as f32)) {
            for b in l.to_bits().to_le_bytes() {
                h = (h ^ u64::from(b)).wrapping_mul(0x100_0000_01b3);
            }
        }
    }
    assert_eq!(h, PINNED, "{h:#x}");
}

const PINNED: u64 = 0xa165_f683_57db_859e;
