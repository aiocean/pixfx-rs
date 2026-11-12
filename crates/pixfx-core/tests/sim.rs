//! Simulation constants (pixelfx `src/core/sim.js`) the fire effects use.

use pixfx_core::sim::SMOKE_RAMP;

/// pixelfx `SMOKE_RAMP` as packed words, printed by `tests/golden/smoke_ramp.mjs` at pixelfx 044e12a.
const PIXELFX_SMOKE_RAMP: [u32; 5] = [0xff726f7b, 0xff60565d, 0xff483d43, 0xff30262b, 0xff1e171a];

#[test]
fn smoke_ramp_matches_pixelfx() {
    let packed: Vec<u32> = SMOKE_RAMP.iter().map(|c| c.pack()).collect();
    assert_eq!(packed, PIXELFX_SMOKE_RAMP);
}
