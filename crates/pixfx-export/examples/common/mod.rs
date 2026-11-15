//! What the scripts in `examples/` share: their arguments, the effect they name and the options it renders with.
//! Each script uses part of it.
#![allow(dead_code)]

use std::path::PathBuf;
use std::process::ExitCode;

use pixfx_core::{Effect, Options};

/// A script's arguments, taken flag by flag; what is left are its plain words.
pub struct Args {
    words: Vec<String>,
    usage: &'static str,
}

impl Args {
    /// The arguments the script was run with; `usage` is shown with every mistake.
    pub fn new(usage: &'static str) -> Self {
        Self {
            words: std::env::args().skip(1).collect(),
            usage,
        }
    }

    /// The word after each `--name`, in order.
    pub fn values(&mut self, name: &str) -> Result<Vec<String>, String> {
        let mut found = Vec::new();
        while let Some(v) = self.value(name)? {
            found.push(v);
        }
        Ok(found)
    }

    /// The word after `--name`, when it was given.
    pub fn value(&mut self, name: &str) -> Result<Option<String>, String> {
        let Some(i) = self.words.iter().position(|w| w == name) else {
            return Ok(None);
        };
        self.words.remove(i);
        if i < self.words.len() {
            Ok(Some(self.words.remove(i)))
        } else {
            Err(format!("{name} needs a value\n{}", self.usage))
        }
    }

    /// The whole number after `--name`, at least 1, else `default`.
    pub fn count(&mut self, name: &str, default: u32) -> Result<u32, String> {
        match self.value(name)? {
            Some(v) => v.parse().ok().filter(|&n| n >= 1).ok_or(format!(
                "{name} expects a whole number of 1 or more: {v}\n{}",
                self.usage
            )),
            None => Ok(default),
        }
    }

    /// `--out DIR`, made if missing; `default` when left out.
    pub fn out(&mut self, default: &str) -> Result<PathBuf, String> {
        let out = PathBuf::from(self.value("--out")?.unwrap_or_else(|| default.to_string()));
        std::fs::create_dir_all(&out).map_err(|e| format!("creating {}: {e}", out.display()))?;
        Ok(out)
    }

    /// The options of a sheet, gif or bake: `--seed N` (a whole number; left out, the effect's own seed) and
    /// `--palette` (a preset: fire, ice, poison, holy, shadow, arcane, blood, electric, nature, gold). Left out, the
    /// effect looks as designed.
    pub fn look(&mut self) -> Result<Options, String> {
        let seed = match self.value("--seed")? {
            Some(v) => Some(
                v.parse::<u64>()
                    .map_err(|_| format!("--seed expects a whole number: {v}\n{}", self.usage))?,
            ),
            None => None,
        };
        let palette = self
            .value("--palette")?
            .map(|w| pixfx_draw::palette::parse(&w).map_err(|e| e.to_string()))
            .transpose()?;
        Ok(Options {
            seed,
            palette,
            ..Options::default()
        })
    }

    /// The one word left once every flag was taken: the effect id. A flag nobody took is a mistake.
    pub fn id(self) -> Result<String, String> {
        if let Some(flag) = self.words.iter().find(|w| w.starts_with("--")) {
            return Err(format!("unknown flag {flag}\n{}", self.usage));
        }
        match <[String; 1]>::try_from(self.words) {
            Ok([id]) => Ok(id),
            Err(_) => Err(self.usage.to_string()),
        }
    }
}

/// The ported effect `id`.
pub fn effect(id: &str) -> Result<&'static dyn Effect, String> {
    pixfx_effects::effect(id).ok_or_else(|| format!("no pixfx effect {id} (not ported yet?)"))
}

/// What files written for effect `id` are named: the id with `/` as `_`.
pub fn file_name(id: &str) -> String {
    id.replace('/', "_")
}

/// The exit code of a script's result: 0 when it ran and passed, 1 when it failed, 2 when it could not run.
pub fn exit(result: Result<bool, String>) -> ExitCode {
    match result {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::from(1),
        Err(e) => {
            eprintln!("{e}");
            ExitCode::from(2)
        }
    }
}
