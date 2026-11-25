//! A picture Rust painted outside the effects (fighters, the scene's light), handed to a shell as the part of the
//! frame that shows anything, its colour and its light as RGBA bytes, for the shell to draw where it says.

use js_sys::Uint8ClampedArray;
use pixfx_core::{PixelBox, color_rgba_in, light_rgba_in};
use pixfx_draw::Frame;
use wasm_bindgen::prelude::*;

/// The lit part of a frame: `x`, `y`, `w` and `h` are in texels. Draw its colour normally (none when the frame
/// has only light), then its light added.
#[wasm_bindgen]
pub struct Picture {
    b: PixelBox,
    color: Option<Vec<u8>>,
    light: Vec<u8>,
    density: f64,
}

#[wasm_bindgen]
impl Picture {
    #[wasm_bindgen(getter)]
    pub fn x(&self) -> u32 {
        self.b.x
    }

    #[wasm_bindgen(getter)]
    pub fn y(&self) -> u32 {
        self.b.y
    }

    #[wasm_bindgen(getter)]
    pub fn w(&self) -> u32 {
        self.b.w
    }

    #[wasm_bindgen(getter)]
    pub fn h(&self) -> u32 {
        self.b.h
    }

    #[wasm_bindgen(getter, js_name = gameX)]
    pub fn game_x(&self) -> f64 {
        f64::from(self.b.x) / self.density
    }

    #[wasm_bindgen(getter, js_name = gameY)]
    pub fn game_y(&self) -> f64 {
        f64::from(self.b.y) / self.density
    }

    #[wasm_bindgen(getter, js_name = gameWidth)]
    pub fn game_width(&self) -> f64 {
        f64::from(self.b.w) / self.density
    }

    #[wasm_bindgen(getter, js_name = gameHeight)]
    pub fn game_height(&self) -> f64 {
        f64::from(self.b.h) / self.density
    }

    /// `w * h * 4` bytes, straight alpha; none for a picture of light alone.
    #[wasm_bindgen(getter)]
    pub fn color(&self) -> Option<Uint8ClampedArray> {
        self.color.as_deref().map(Uint8ClampedArray::from)
    }

    /// Lit pixels the light over transparency (alpha its brightest channel), unlit `0, 0, 0, 0`, to draw added.
    #[wasm_bindgen(getter)]
    pub fn light(&self) -> Uint8ClampedArray {
        Uint8ClampedArray::from(self.light.as_slice())
    }
}

impl Picture {
    /// The part of `frame` that shows anything, or none when nothing shows; `color` false for a frame of light
    /// alone.
    pub(crate) fn of(frame: &Frame, color: bool) -> Option<Picture> {
        let b = PixelBox::lit(frame)?;
        let len = b.w as usize * b.h as usize * 4;
        let color = color.then(|| {
            let mut bytes = vec![0; len];
            color_rgba_in(frame, b, &mut bytes);
            bytes
        });
        let mut light = vec![0; len];
        light_rgba_in(frame, b, &mut light);
        Some(Picture {
            b,
            color,
            light,
            density: f64::from(frame.density().factor()),
        })
    }
}
