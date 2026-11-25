// One layer of a pixfx frame on a quad: the texel as Rust painted it, sampled nearest. The colour layer is drawn
// with straight alpha, the light layer added (its blend is set in `LightLayer::specialize`). Tonemapping and the
// compositing space follow the camera, as they do for the game's own sprites.
import bevy_sprite_render::mesh2d::{
    vertex_output::VertexOutput,
    view_bindings::view,
};

@if(TONEMAP_IN_SHADER)
import bevy_core_pipeline::tonemapping;
@if(SRGB_OUTPUT)
import bevy_render::color_operations::linear_to_srgb;
@if(OKLAB_OUTPUT)
import bevy_render::color_operations::linear_rgb_to_oklab;

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var texture: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(1) var texture_sampler: sampler;

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    var out = textureSample(texture, texture_sampler, in.uv);
@if(TONEMAP_IN_SHADER)
    out = tonemapping::tone_mapping(out, view.color_grading);
@if(SRGB_OUTPUT)
    out = vec4(linear_to_srgb(out.rgb), out.a);
@if(OKLAB_OUTPUT)
    out = vec4(linear_rgb_to_oklab(out.rgb), out.a);
    return out;
}
