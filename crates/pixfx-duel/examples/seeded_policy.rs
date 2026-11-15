//! Write a policy network of seeded random weights: the stand-in `weights/policy.bin` until a trained one exists.
//!
//! `cargo run -p pixfx-duel --example seeded_policy -- [--seed N] [--out crates/pixfx-duel/weights/policy.bin]`

use pixfx_mind::policy::Net;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let arg = |name: &str| args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).cloned();
    let seed: u32 = arg("--seed").map_or(Ok(1), |s| s.parse())?;
    let out = arg("--out").unwrap_or_else(|| "crates/pixfx-duel/weights/policy.bin".to_string());
    let bytes = Net::seeded(seed).to_bytes();
    std::fs::write(&out, &bytes)?;
    println!("wrote {out}: {} bytes, seed {seed}", bytes.len());
    Ok(())
}
