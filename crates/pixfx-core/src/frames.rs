//! The frames hosts share: one drawn layer per [`FrameId`], kept in whatever form the host shows (a scratch
//! canvas, a GPU image), under a byte budget. A host shows an effect's frame from the store, and only the first
//! effect to reach a frame draws it ([`crate::Player::shown`] says which frame that is).

use std::collections::HashMap;

use crate::effect::Layer;
use crate::key::FrameId;

/// Bytes a store holds before the least recently shown frames go, unless set: 64 MiB.
pub const FRAME_BUDGET: u64 = 64 << 20;

struct Held<S> {
    surface: S,
    bytes: u64,
    /// The host's draw count when it was last shown.
    shown: u64,
}

/// Drawn layers by frame, least recently shown dropped first once over the budget.
pub struct FrameStore<S> {
    held: HashMap<(FrameId, Layer), Held<S>>,
    bytes: u64,
    budget: u64,
}

impl<S> Default for FrameStore<S> {
    fn default() -> Self {
        Self {
            held: HashMap::new(),
            bytes: 0,
            budget: FRAME_BUDGET,
        }
    }
}

impl<S> FrameStore<S> {
    /// The surface holding `layer` of frame `id`, marked shown at `now` (the host's draw count). When the
    /// store lacks it, `make` draws it and returns the surface and how many bytes it holds.
    pub fn show<E>(
        &mut self,
        id: &FrameId,
        layer: Layer,
        now: u64,
        make: impl FnOnce() -> Result<(S, u64), E>,
    ) -> Result<&S, E> {
        let key = (id.clone(), layer);
        let held = match self.held.entry(key) {
            std::collections::hash_map::Entry::Occupied(e) => e.into_mut(),
            std::collections::hash_map::Entry::Vacant(e) => {
                let (surface, bytes) = make()?;
                self.bytes += bytes;
                e.insert(Held {
                    surface,
                    bytes,
                    shown: now,
                })
            }
        };
        held.shown = now;
        Ok(&held.surface)
    }

    /// Drop the least recently shown surfaces until the store fits its budget, keeping every one shown at
    /// `now`; returns the dropped ones (a host frees what they hold).
    pub fn trim(&mut self, now: u64) -> Vec<S> {
        let mut dropped = Vec::new();
        while self.bytes > self.budget {
            let Some(oldest) = self
                .held
                .iter()
                .filter(|(_, h)| h.shown < now)
                .min_by_key(|(_, h)| h.shown)
                .map(|(k, _)| k.clone())
            else {
                break;
            };
            if let Some(h) = self.held.remove(&oldest) {
                self.bytes -= h.bytes;
                dropped.push(h.surface);
            }
        }
        dropped
    }

    /// Bytes the store may hold ([`FRAME_BUDGET`] unless set); takes effect at the next [`FrameStore::trim`].
    pub fn set_budget(&mut self, bytes: u64) {
        self.budget = bytes;
    }

    /// Bytes the held surfaces take.
    pub fn bytes(&self) -> u64 {
        self.bytes
    }
}
