//! Writes the TypeScript union of every registered effect id (`type EffectId = "fire/brazier" | ...`) from the
//! registry itself, so the declarations cannot drift from it.

use std::fmt::Write as _;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo::rerun-if-changed=build.rs");
    let mut ts =
        String::from("/** Every effect in the registry, by id. */\nexport type EffectId =");
    for e in pixfx_effects::EFFECTS {
        write!(ts, "\n    | {:?}", e.meta().id)?;
    }
    ts.push_str(";\n");
    let out = PathBuf::from(std::env::var("OUT_DIR")?).join("effect_id.d.ts");
    std::fs::write(out, ts)?;
    Ok(())
}
