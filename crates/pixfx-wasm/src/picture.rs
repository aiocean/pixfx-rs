//! A picture Rust painted outside the effects (fighters, the scene's light), handed to a shell as the part of the
//! frame that shows anything, its colour and its light as RGBA bytes, for the shell to draw where it says.

use js_sys::Uint8ClampedArray;
use pixfx_core::{PixelBox, color_rgba_in, light_rgba_in};
use pixfx_draw::Frame;
use wasm_bindgen::prelude::*;

/// The lit part of a frame: drawn at (`x`, `y`) game pixels, `w` x `h`, its colour normally (none when the frame
/// has only light), then its light added.
#[wasm_bindgen]
pub struct Picture {
    b: PixelBox,
    color: Option<Vec<u8>>,
    light: Vec<u8>,
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

    /// `w * h * 4` bytes, straight alpha; none for a picture of light alone.
    #[wasm_bindgen(getter)]
    pub fn color(&self) -> Option<Uint8ClampedArray> {
        self.color.as_deref().map(Uint8ClampedArray::from)
    }

    /// Lit pixels `r, g, b, 255`, unlit `0, 0, 0, 0`, to draw added.
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
        Some(Picture { b, color, light })
    }
}
