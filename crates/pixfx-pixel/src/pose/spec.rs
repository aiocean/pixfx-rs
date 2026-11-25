//! A pose sheet written as JSON, for an agent that would rather write a file than Rust:
//!
//! ```json
//! {
//!   "body": { "height": 38, "heads": 5.4 },
//!   "cell": [64, 64], "cols": 4, "gap": 8, "scale": 8, "ground": 2,
//!   "cells": [
//!     { "caption": "IDLE: standing at ease", "preset": "idle" },
//!     { "caption": "WALK: right foot forward", "preset": "walk", "phase": 0.25, "facing": 45 },
//!     { "caption": "GUARD, head turned to the viewer", "preset": "guard", "pose": { "head": { "turn": -40 } } },
//!     { "caption": "IDLE", "preset": "idle", "turnaround": true }
//!   ]
//! }
//! ```
//!
//! Everything but `cells` may be left out (see [`PoseSheet::default`]). A cell's `pose` is laid over its `preset`
//! field by field (or over the rest pose without one), so it names only what changes. `turnaround` puts the cell
//! in all eight facings, each captioned `<caption>: <facing>`.

use serde::Deserialize;
use serde_json::Value;

use super::presets;
use super::{Cell, PoseSheet, turnaround};
use pixfx_core::body::Body;
use pixfx_core::body::Pose;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BodySpec {
    height: f64,
    heads: f64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CellSpec {
    #[serde(default)]
    caption: String,
    preset: Option<String>,
    #[serde(default)]
    phase: f64,
    pose: Option<Value>,
    #[serde(default)]
    facing: f64,
    #[serde(default)]
    turnaround: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SheetSpec {
    body: Option<BodySpec>,
    cell: Option<(u32, u32)>,
    cols: Option<u32>,
    gap: Option<u32>,
    scale: Option<u32>,
    ground: Option<u32>,
    cells: Vec<CellSpec>,
}

/// `over` laid onto `base`: objects merge key by key, anything else replaces.
fn overlay(base: &mut Value, over: Value) {
    match (base, over) {
        (Value::Object(b), Value::Object(o)) => {
            for (k, v) in o {
                match b.get_mut(&k) {
                    Some(slot) if slot.is_object() && v.is_object() => overlay(slot, v),
                    _ => {
                        b.insert(k, v);
                    }
                }
            }
        }
        (b, o) => *b = o,
    }
}

fn pose_of(c: &CellSpec, n: usize) -> Result<Pose, String> {
    let base = match &c.preset {
        Some(name) => presets::by_name(name, c.phase).ok_or_else(|| {
            format!(
                "cell {n}: no preset \"{name}\"; presets are {}",
                presets::NAMES.join(", ")
            )
        })?,
        None => Pose::default(),
    };
    let Some(over) = &c.pose else {
        return Ok(base);
    };
    let mut v = serde_json::to_value(base).map_err(|e| format!("cell {n}: {e}"))?;
    // A sword named over a preset without one starts from the default grip.
    if let (Some(Value::Null), Some(Value::Object(_))) = (v.get("sword"), over.get("sword"))
        && let Some(obj) = v.as_object_mut()
    {
        let grip = serde_json::to_value(pixfx_core::body::Sword::default())
            .map_err(|e| format!("cell {n}: {e}"))?;
        obj.insert("sword".into(), grip);
    }
    overlay(&mut v, over.clone());
    serde_json::from_value(v).map_err(|e| format!("cell {n} pose: {e}"))
}

/// A sheet from its JSON. Fails naming the cell and the field on an unknown preset, an unknown field or a value of
/// the wrong type.
pub fn parse(json: &str) -> Result<PoseSheet, String> {
    let spec: SheetSpec = serde_json::from_str(json).map_err(|e| format!("pose sheet: {e}"))?;
    let d = PoseSheet::default();
    let mut cells = Vec::new();
    for (i, c) in spec.cells.iter().enumerate() {
        let pose = pose_of(c, i + 1)?;
        if c.turnaround {
            cells.extend(turnaround(&c.caption, &pose));
        } else {
            cells.push(Cell {
                caption: c.caption.clone(),
                pose,
                facing: c.facing,
            });
        }
    }
    Ok(PoseSheet {
        body: spec.body.map_or(d.body, |b| Body::new(b.height, b.heads)),
        cell: spec.cell.unwrap_or(d.cell),
        cols: spec.cols.unwrap_or(d.cols),
        gap: spec.gap.unwrap_or(d.gap),
        scale: spec.scale.unwrap_or(d.scale),
        ground: spec.ground.unwrap_or(d.ground),
        cells,
    })
}
