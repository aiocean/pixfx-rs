//! The effects a test names, as far as this build has them (`mod built;`). A test about a few named effects runs on
//! those its build turned on, so `--no-default-features --features <one effect>` tests that effect alone; with
//! `all` every named effect must be there, so a misspelt or removed id fails rather than being skipped.

use pixfx_core::Effect;

/// Effect `id` when this build has it; `Ok(None)` when its feature is off; an error under `all`, which has every one.
pub fn built(id: &str) -> Result<Option<&'static dyn Effect>, String> {
    match pixfx_effects::effect(id) {
        Some(fx) => Ok(Some(fx)),
        None if cfg!(feature = "all") => Err(format!("{id} is not registered")),
        None => Ok(None),
    }
}
