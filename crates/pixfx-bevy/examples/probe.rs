use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use pixfx_bevy::{ColorLayer, LightLayer, PixfxPlugin};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(PixfxPlugin)
        .add_systems(Startup, setup)
        .add_systems(Update, shoot).add_systems(PostUpdate, swap.after(bevy::transform::TransformSystems::Propagate).before(bevy::camera::visibility::VisibilitySystems::CheckVisibility))
        .run();
}

fn setup(mut commands: Commands, mut images: ResMut<Assets<Image>>, mut meshes: ResMut<Assets<Mesh>>, mut c: ResMut<Assets<ColorLayer>>, mut l: ResMut<Assets<LightLayer>>, mut cm: ResMut<Assets<ColorMaterial>>) {
    commands.spawn(Camera2d);
    let img = images.add(Image::new(Extent3d { width: 1, height: 1, depth_or_array_layers: 1 }, TextureDimension::D2, vec![255, 0, 0, 255], TextureFormat::Rgba8UnormSrgb, RenderAssetUsages::RENDER_WORLD));
    let quad = meshes.add(Rectangle::new(1.0, 1.0));
    commands.spawn((Mesh2d(quad.clone()), MeshMaterial2d(c.add(ColorLayer { texture: img.clone() })), Transform::from_xyz(-100.0, 0.0, 0.0).with_scale(Vec3::splat(80.0))));
    commands.spawn((Mesh2d(quad.clone()), MeshMaterial2d(l.add(LightLayer { texture: img.clone() })), Transform::from_xyz(0.0, 0.0, 0.0).with_scale(Vec3::splat(80.0))));
    commands.spawn((Mesh2d(quad.clone()), MeshMaterial2d(cm.add(ColorMaterial { texture: Some(img.clone()), ..default() })), Transform::from_xyz(100.0, 0.0, 0.0).with_scale(Vec3::splat(80.0))));
}

#[derive(Component)] struct A;
#[derive(Component)] struct B;
fn swap(mut n: Local<u32>, mut commands: Commands, mut a: Query<&mut MeshMaterial2d<ColorLayer>, With<A>>, mut b: Query<(&mut GlobalTransform, &mut Transform, &mut Visibility), With<B>>, mut c: ResMut<Assets<ColorLayer>>, mut images: ResMut<Assets<Image>>, mut meshes: ResMut<Assets<Mesh>>) {
    *n += 1;
    if *n == 2 {
        let quad = meshes.add(Rectangle::new(1.0, 1.0));
        commands.spawn((A, Mesh2d(quad.clone()), MeshMaterial2d::<ColorLayer>::default(), Transform::from_xyz(-100.0, 150.0, 0.0).with_scale(Vec3::splat(80.0)), Visibility::Hidden));
        let img = images.add(Image::new(Extent3d { width: 1, height: 1, depth_or_array_layers: 1 }, TextureDimension::D2, vec![0, 255, 0, 255], TextureFormat::Rgba8UnormSrgb, RenderAssetUsages::RENDER_WORLD));
        commands.spawn((B, Mesh2d(quad.clone()), MeshMaterial2d(c.add(ColorLayer { texture: img })), Transform::default(), Visibility::Hidden));
    }
    if *n == 10 {
        let img = images.add(Image::new(Extent3d { width: 1, height: 1, depth_or_array_layers: 1 }, TextureDimension::D2, vec![0, 0, 255, 255], TextureFormat::Rgba8UnormSrgb, RenderAssetUsages::RENDER_WORLD));
        for mut m in &mut a { m.0 = c.add(ColorLayer { texture: img.clone() }); }
        commands.queue(|w: &mut World| { for mut v in w.query_filtered::<&mut Visibility, With<A>>().iter_mut(w) { *v = Visibility::Inherited; } });
    }
    if *n == 12 {
        for (mut g, mut t, mut v) in &mut b {
            let at = Transform::from_xyz(100.0, 150.0, 0.0).with_scale(Vec3::splat(80.0));
            *t = at; *g = GlobalTransform::from(at); *v = Visibility::Inherited;
        }
    }
}
fn shoot(mut commands: Commands, mut n: Local<u32>, mut exit: MessageWriter<AppExit>) {
    *n += 1;
    if *n == 60 { commands.spawn(Screenshot::primary_window()).observe(save_to_disk("/tmp/probe.png")); }
    if *n == 90 { exit.write(AppExit::Success); }
}
