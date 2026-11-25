//! Bevy's side of [`Shell`]: a frame becomes two textures cut to its lit box, each behind a material (colour drawn
//! normally, light added), and a shown surface is queued as a [`Shown`] for the pool of quads to follow.

use bevy::asset::{Asset, Assets, Handle, RenderAssetUsages, embedded_path};
use bevy::image::{Image, ImageSampler};
use bevy::mesh::MeshVertexBufferLayoutRef;
use bevy::reflect::TypePath;
use bevy::render::render_resource::{
    AsBindGroup, BlendComponent, BlendFactor, BlendOperation, BlendState, Extent3d,
    RenderPipelineDescriptor, SpecializedMeshPipelineError, TextureDimension, TextureFormat,
};
use bevy::shader::ShaderRef;
use bevy::sprite_render::{AlphaMode2d, Material2d, Material2dKey, Material2dPipeline};
use pixfx_core::{PixelBox, color_rgba_in, light_rgba_in};
use pixfx_draw::Frame;
use pixfx_scene::{Pass, Shell};

fn shader() -> ShaderRef {
    ShaderRef::Path(
        bevy::asset::AssetPath::from_path_buf(embedded_path!("pixfx.wesl")).with_source("embedded"),
    )
}

/// A frame's colour layer: straight-alpha RGBA, drawn over what is behind it.
#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct ColorLayer {
    #[texture(0)]
    #[sampler(1)]
    pub texture: Handle<Image>,
}

impl Material2d for ColorLayer {
    fn fragment_shader() -> ShaderRef {
        shader()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }
}

/// A frame's light layer as [`light_rgba_in`] makes it, added to what is behind it (`"lighter"`): rgb times alpha is
/// the light, and the alpha behind is kept.
#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct LightLayer {
    #[texture(0)]
    #[sampler(1)]
    pub texture: Handle<Image>,
}

impl Material2d for LightLayer {
    fn fragment_shader() -> ShaderRef {
        shader()
    }

    // transparent, so it sorts by z with the game's sprites; its blend is replaced below
    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }

    fn specialize(
        _pipeline: &Material2dPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        _key: Material2dKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        let targets = descriptor.fragment.iter_mut().flat_map(|f| f.targets.iter_mut());
        for target in targets.flatten() {
            target.blend = Some(BlendState {
                color: BlendComponent {
                    src_factor: BlendFactor::SrcAlpha,
                    dst_factor: BlendFactor::One,
                    operation: BlendOperation::Add,
                },
                alpha: BlendComponent {
                    src_factor: BlendFactor::Zero,
                    dst_factor: BlendFactor::One,
                    operation: BlendOperation::Add,
                },
            });
        }
        Ok(())
    }
}

/// One layer of a surface: its material and texture, and where the texture sits on the frame.
#[derive(Clone, Debug)]
pub struct Part<M: Asset> {
    pub material: Handle<M>,
    pub image: Handle<Image>,
    /// The lit box the texture holds, in pixels from the frame's top-left.
    pub cut: PixelBox,
}

/// One drawn layer of one effect, as textures: colour and light, each `None` when it holds nothing. The handles are
/// strong, so a surface the scene frees leaves the GPU once no quad shows it either.
#[derive(Clone, Debug, Default)]
pub struct Surface {
    pub color: Option<Part<ColorLayer>>,
    pub light: Option<Part<LightLayer>>,
    /// The frame's width and height: a pass's rect spans them.
    pub frame: (u32, u32),
}

/// A surface shown this frame, in the order the scene showed it.
#[derive(Clone, Debug)]
pub struct Shown {
    pub pass: Pass,
    pub surface: Surface,
}

/// The assets surfaces live in, and the surfaces shown this frame.
pub struct BevyShell<'a> {
    pub images: &'a mut Assets<Image>,
    pub colors: &'a mut Assets<ColorLayer>,
    pub lights: &'a mut Assets<LightLayer>,
    pub shown: &'a mut Vec<Shown>,
}

/// The pixels of `cut` as a texture Bevy samples nearest; sRGB, as Rust painted them.
fn texture(cut: PixelBox, rgba: Vec<u8>) -> Image {
    let mut image = Image::new(
        Extent3d {
            width: cut.w,
            height: cut.h,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        rgba,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.sampler = ImageSampler::nearest();
    image
}

/// `frame`'s colour and light in `cut` as RGBA bytes, each `None` when no pixel of it shows.
fn layers(frame: &Frame, cut: PixelBox) -> (Option<Vec<u8>>, Option<Vec<u8>>) {
    let n = cut.w as usize * cut.h as usize * 4;
    let (mut color, mut light) = (vec![0; n], vec![0; n]);
    color_rgba_in(frame, cut, &mut color);
    light_rgba_in(frame, cut, &mut light);
    let any = |rgba: &[u8]| rgba.as_chunks::<4>().0.iter().any(|p| p[3] != 0);
    let color = any(&color).then_some(color);
    let light = any(&light).then_some(light);
    (color, light)
}

impl BevyShell<'_> {
    fn surface(&mut self, frame: &Frame) -> Surface {
        let mut surface = Surface {
            frame: (frame.w, frame.h),
            ..Surface::default()
        };
        let Some(cut) = PixelBox::lit(frame) else {
            return surface;
        };
        let (color, light) = layers(frame, cut);
        surface.color = color.map(|rgba| {
            let image = self.images.add(texture(cut, rgba));
            let material = self.colors.add(ColorLayer {
                texture: image.clone(),
            });
            Part {
                material,
                image,
                cut,
            }
        });
        surface.light = light.map(|rgba| {
            let image = self.images.add(texture(cut, rgba));
            let material = self.lights.add(LightLayer {
                texture: image.clone(),
            });
            Part {
                material,
                image,
                cut,
            }
        });
        surface
    }
}

impl Shell for BevyShell<'_> {
    type Surface = Surface;
    type Error = std::convert::Infallible;

    fn make(&mut self, frame: &Frame) -> Result<Surface, Self::Error> {
        Ok(self.surface(frame))
    }

    fn refill(&mut self, surface: &mut Surface, frame: &Frame) -> Result<(), Self::Error> {
        // the old textures leave once no quad shows them
        *surface = self.surface(frame);
        Ok(())
    }

    fn show(&mut self, pass: Pass, surface: &Surface) -> Result<(), Self::Error> {
        if surface.color.is_some() || surface.light.is_some() {
            self.shown.push(Shown {
                pass,
                surface: surface.clone(),
            });
        }
        Ok(())
    }

    fn free(&mut self, surface: Surface) -> Result<(), Self::Error> {
        drop(surface);
        Ok(())
    }
}
