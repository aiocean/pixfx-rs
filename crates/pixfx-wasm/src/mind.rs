//! `pixfx-mind` for JS: a fighter's brain ([`Mind`]). Every object JS passes is read once, strictly, into the Rust
//! types ([`crate::js::parse`]): a misspelt, missing or mistyped field throws, naming it.

use crate::js::{parse, to_js};
use pixfx_mind::{self as m, BodySize, Outcome, Profile, Sense, Skill};
use wasm_bindgen::prelude::*;

/// A fighter's brain. Hand it a sense every tick; it answers with a thought when it is time to think.
#[wasm_bindgen]
pub struct Mind {
    inner: m::Mind,
}

#[wasm_bindgen]
impl Mind {
    /// A mind for a fighter with this temperament, body and skills, rolling its dice from `seed`.
    #[wasm_bindgen(constructor)]
    pub fn new(
        #[wasm_bindgen(unchecked_param_type = "Profile")] profile: &JsValue,
        #[wasm_bindgen(unchecked_param_type = "BodySize")] body: &JsValue,
        #[wasm_bindgen(unchecked_param_type = "Skill[]")] skills: &JsValue,
        seed: u32,
    ) -> Result<Mind, JsError> {
        let profile: Profile = parse("profile", profile)?;
        let body: BodySize = parse("body", body)?;
        let skills: Vec<Skill> = parse("skills", skills)?;
        Ok(Mind {
            inner: m::Mind::new(profile, body, skills, seed),
        })
    }

    /// Take in this tick's sense, `dt` seconds after the last: a thought, or null when it is not time to think.
    #[wasm_bindgen(unchecked_return_type = "Thought | null")]
    pub fn think(
        &mut self,
        #[wasm_bindgen(unchecked_param_type = "Sense")] sense: &JsValue,
        dt: f64,
    ) -> Result<JsValue, JsError> {
        let sense: Sense = parse("sense", sense)?;
        match self.inner.think(&sense, dt) {
            Some(thought) => to_js(&thought),
            None => Ok(JsValue::NULL),
        }
    }

    /// What the game saw happen to a skill: `{type: "landed" | "punished", skill}`.
    pub fn tell(
        &mut self,
        #[wasm_bindgen(unchecked_param_type = "Outcome")] outcome: &JsValue,
    ) -> Result<(), JsError> {
        let outcome: Outcome = parse("outcome", outcome)?;
        self.inner.tell(&outcome);
        Ok(())
    }

    /// A new round: it forgets what it learnt and held.
    pub fn reset(&mut self) {
        self.inner.reset();
    }

    /// No thought for `seconds`.
    pub fn hold(&mut self, seconds: f64) {
        self.inner.hold(seconds);
    }

    /// Fill every thought's `why`.
    pub fn explain(&mut self, on: bool) {
        self.inner.explain(on);
    }

    /// The posture it is in.
    #[wasm_bindgen(getter, unchecked_return_type = "Posture")]
    pub fn posture(&self) -> Result<JsValue, JsError> {
        to_js(&self.inner.posture())
    }
}
