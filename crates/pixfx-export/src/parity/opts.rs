//! The `key=value` options of a parity check: parsed once, then given to the port as [`Options`] and to the
//! reference script as the JSON it takes in `--opts`, so both sides render the same instance.

use pixfx_core::effect::{OptValue, Options};

/// Largest integer both sides keep exactly (JS `Number.MAX_SAFE_INTEGER`).
const MAX_SAFE_INTEGER: f64 = 9_007_199_254_740_991.0;

/// The options of one run, in the order given. `seed` and `flip` are known; every other key is a number
/// for the effect's own option of that name.
#[derive(Clone, Debug, PartialEq)]
pub struct ParityOpts {
    seed: Option<u64>,
    flip: bool,
    extra: Vec<(String, f64)>,
}

impl ParityOpts {
    /// Parse `key=value` words: `seed` a whole number, `flip` 0 or 1, anything else a number.
    pub fn parse(words: &[String]) -> Result<Self, String> {
        let mut out = Self {
            seed: None,
            flip: false,
            extra: Vec::new(),
        };
        for w in words {
            let (key, val) = w
                .split_once('=')
                .ok_or_else(|| format!("option {w:?} is not key=value"))?;
            if key.is_empty() || !key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                return Err(format!("option name {key:?} is not a plain identifier"));
            }
            let n: f64 = val
                .parse()
                .ok()
                .filter(|n: &f64| n.is_finite())
                .ok_or_else(|| format!("option {key}: {val:?} is not a number"))?;
            match key {
                "seed" if n.fract() == 0.0 && (0.0..=MAX_SAFE_INTEGER).contains(&n) => {
                    out.seed = Some(n as u64);
                }
                "seed" => return Err(format!("option seed: {val:?} is not a whole number >= 0")),
                "flip" if n == 0.0 || n == 1.0 => out.flip = n == 1.0,
                "flip" => return Err(format!("option flip: {val:?} is not 0 or 1")),
                "x" | "y" => {
                    return Err(format!(
                        "option {key} is not supported: an effect sits at its own origin"
                    ));
                }
                _ => out.extra.push((key.to_string(), n)),
            }
        }
        Ok(out)
    }

    /// The options for the port.
    pub fn options(&self) -> Options {
        let mut o = Options {
            seed: self.seed,
            flip: self.flip,
            ..Options::default()
        };
        for (k, v) in &self.extra {
            o.extra.insert(k.clone(), OptValue::Num(*v));
        }
        o
    }

    /// The JSON `pixelfx-ref.mjs --opts` takes: `{"seed": n, "options": {...}}`, `seed` left out when unset.
    pub fn to_json(&self) -> String {
        let mut fields: Vec<String> = Vec::new();
        if self.flip {
            fields.push("\"flip\":true".to_string());
        }
        fields.extend(self.extra.iter().map(|(k, v)| format!("\"{k}\":{v}")));
        let options = format!("\"options\":{{{}}}", fields.join(","));
        match self.seed {
            Some(s) => format!("{{\"seed\":{s},{options}}}"),
            None => format!("{{{options}}}"),
        }
    }
}
