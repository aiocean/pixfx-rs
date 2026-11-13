//! A frame store draws each frame once, shares it, and drops the least recently shown frames past its budget,
//! never one shown in the current draw.

use pixfx_core::{ClipKey, FrameId, FrameStore, Layer, Options, Placement};

fn id(index: u64) -> FrameId {
    let place = Placement {
        x: 0.0,
        y: 0.0,
        flip: false,
        angle: 0.0,
        light: 1.0,
    };
    FrameId {
        key: ClipKey::new("fire/candle", &Options::default(), 0, place),
        index,
    }
}

/// Show `layer` of frame `index` at draw `now`; returns whether it had to be made.
fn show(store: &mut FrameStore<u64>, index: u64, layer: Layer, now: u64) -> Result<bool, ()> {
    let mut made = false;
    let got = *store.show(&id(index), layer, now, || {
        made = true;
        Ok::<_, ()>((index, 100))
    })?;
    assert_eq!(got, index);
    Ok(made)
}

#[test]
fn a_frame_is_made_once_per_layer_and_then_shared() -> Result<(), ()> {
    let mut store = FrameStore::default();
    assert!(show(&mut store, 0, Layer::Back, 1)?);
    assert!(!show(&mut store, 0, Layer::Back, 1)?, "shared");
    assert!(
        show(&mut store, 0, Layer::Front, 1)?,
        "another layer is its own"
    );
    assert_eq!(store.bytes(), 200);
    Ok(())
}

#[test]
fn past_the_budget_the_least_recently_shown_go_but_never_one_shown_now() -> Result<(), ()> {
    let mut store = FrameStore::default();
    store.set_budget(250);
    show(&mut store, 0, Layer::Back, 1)?;
    show(&mut store, 1, Layer::Back, 2)?;
    show(&mut store, 2, Layer::Back, 3)?;
    assert_eq!(store.trim(3), vec![0], "frame 0 was shown longest ago");
    assert_eq!(store.bytes(), 200);
    // everything shown at draw 4 stays, even over the budget
    store.set_budget(0);
    show(&mut store, 1, Layer::Back, 4)?;
    show(&mut store, 2, Layer::Back, 4)?;
    assert!(store.trim(4).is_empty());
    assert_eq!(store.trim(5).len(), 2, "once no longer shown, they go");
    assert!(
        show(&mut store, 1, Layer::Back, 6)?,
        "a dropped frame is made again"
    );
    Ok(())
}
