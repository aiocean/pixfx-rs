//! Scene light for JS: the light a game's spells, blows and lamps throw on its scene. The game owns its lights as
//! plain values (`{key?, x, y, reach, color, strength, life?, age?}`); these age them, put them out and paint them.
//! Objects are read strictly: a missing, misspelt or mistyped field throws, naming it.

use pixfx_core::light::{self, Glow, Light, Style};
use pixfx_core::valid_dt;
use pixfx_draw::Frame;
use wasm_bindgen::prelude::*;

use crate::js::{parse, to_js};
use crate::picture::Picture;

/// `lights` `dt` seconds (finite, 0 or more) older; those whose life is over are gone.
#[wasm_bindgen(js_name = stepLights, unchecked_return_type = "Light[]")]
pub fn step_lights(
    #[wasm_bindgen(unchecked_param_type = "Light[]")] lights: &JsValue,
    dt: f64,
) -> Result<JsValue, JsError> {
    if !valid_dt(dt) {
        return Err(JsError::new(
            "dt must be a finite number of seconds, 0 or more",
        ));
    }
    let lights: Vec<Light> = parse("lights", lights)?;
    to_js(&light::step(lights, dt))
}

/// `light` set to go out at most `within` seconds from now.
#[wasm_bindgen(js_name = putOutLight, unchecked_return_type = "Light")]
pub fn put_out_light(
    #[wasm_bindgen(unchecked_param_type = "Light")] light: &JsValue,
    within: f64,
) -> Result<JsValue, JsError> {
    let light: Light = parse("light", light)?;
    to_js(&light.put_out(within))
}

/// The light on a `w` x `h` game-pixel scene, in the duel's style (`light::Style::default()`), painted
/// into a frame kept from one call to the next.
#[wasm_bindgen]
pub struct Lighting {
    style: Style,
    frame: Frame,
}

#[wasm_bindgen]
impl Lighting {
    #[wasm_bindgen(constructor)]
    pub fn new(w: u32, h: u32) -> Result<Lighting, JsError> {
        if w == 0 || h == 0 {
            return Err(JsError::new("the scene must be at least 1 x 1"));
        }
        Ok(Lighting {
            style: Style::default(),
            frame: Frame::new(w, h, 0, 0),
        })
    }

    /// The light of `lights` and `glows` (this instant's extra lights, such as flickering lamps:
    /// `[{x, y, reach, color, strength}]`): its lit part, to draw added, or none when nothing is lit.
    pub fn paint(
        &mut self,
        #[wasm_bindgen(unchecked_param_type = "Light[]")] lights: &JsValue,
        #[wasm_bindgen(unchecked_param_type = "Glow[]")] glows: &JsValue,
    ) -> Result<Option<Picture>, JsError> {
        let lights: Vec<Light> = parse("lights", lights)?;
        let mut all: Vec<Glow> = parse("glows", glows)?;
        all.extend(lights.iter().map(|l| l.glow(&self.style)));
        self.frame.clear();
        light::draw(&mut self.frame, &all, &self.style);
        Ok(Picture::of(&self.frame, false))
    }
}
