//! A Bevy window playing pixfx: a power-up rises on its stand-in body, a square the arrow keys move leaves a trail
//! behind it, and a click blows a stand-in body apart where it lands, with its shake, hit-stop and flash.
//!
//! cargo run --manifest-path crates/pixfx-bevy/Cargo.toml --example play
//! cargo run --manifest-path crates/pixfx-bevy/Cargo.toml --example play -- --shot out/bevy.png
//!
//! `--shot PATH` plays a body burst 1.6 s in, saves the window 0.4 s later and quits.

use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use pixfx_bevy::{Depth, PixfxCamera, PixfxEvent, PixfxPlugin, Pixfx};
use pixfx_core::Options;

/// Screen pixels per game pixel.
const ZOOM: f32 = 4.0;

#[derive(Component)]
struct Hero;

#[derive(Resource)]
struct Shot {
    path: Option<String>,
    secs: f32,
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let path = args
        .iter()
        .position(|a| a == "--shot")
        .and_then(|i| args.get(i + 1).cloned());
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "pixfx in Bevy".into(),
                resolution: (960, 540).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(PixfxPlugin)
        .insert_resource(ClearColor(Color::srgb_u8(0x1a, 0x1c, 0x2c)))
        .insert_resource(Shot { path, secs: 0.0 })
        .add_systems(Startup, setup)
        .add_systems(Update, (walk, click, listen, shoot, debug_dump))
        .run();
}

fn play(pixfx: &mut Pixfx, id: &str, at: Vec2) -> Option<pixfx_bevy::Fx> {
    let effect = pixfx_effects::effect(id)?;
    pixfx.play(effect, Options::default(), at).ok()
}

fn setup(mut commands: Commands, mut pixfx: ResMut<Pixfx>) {
    commands.spawn((
        Camera2d,
        Projection::Orthographic(OrthographicProjection {
            scale: 1.0 / ZOOM,
            ..OrthographicProjection::default_2d()
        }),
        PixfxCamera::default(),
    ));
    // the ground the effects stand on
    commands.spawn((
        Sprite::from_color(Color::srgb_u8(0x33, 0x3c, 0x57), Vec2::new(240.0, 4.0)),
        Transform::from_xyz(0.0, -2.0, -1.0),
    ));
    let hero = commands
        .spawn((
            Hero,
            Sprite::from_color(Color::srgb_u8(0xf4, 0xf4, 0xf4), Vec2::new(8.0, 16.0)),
            Transform::from_xyz(-40.0, 8.0, 0.5),
        ))
        .id();
    play(&mut pixfx, "feedback/power-up", Vec2::new(50.0, 0.0));
    if let Some(trail) = play(&mut pixfx, "projectiles/trail", Vec2::ZERO) {
        // behind the hero's sprite, following it about: it draws the way the hero went
        pixfx.set_depth(trail, Depth { back: 0.0, front: 1.0 });
        pixfx.follow(trail, Some(hero), Vec2::ZERO);
    }
}

fn walk(keys: Res<ButtonInput<KeyCode>>, time: Res<Time>, pixfx: Res<Pixfx>, mut hero: Query<&mut Transform, With<Hero>>) {
    let mut d = Vec2::ZERO;
    for (key, step) in [
        (KeyCode::ArrowLeft, Vec2::NEG_X),
        (KeyCode::ArrowRight, Vec2::X),
        (KeyCode::ArrowUp, Vec2::Y),
        (KeyCode::ArrowDown, Vec2::NEG_Y),
    ] {
        if keys.pressed(key) {
            d += step;
        }
    }
    // the world freezes with the effects in a hit-stop
    let dt = time.delta_secs() * pixfx.time_scale() as f32;
    for mut t in &mut hero {
        t.translation += (d * 60.0 * dt).extend(0.0);
    }
}

fn click(
    buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    cameras: Query<(&Camera, &GlobalTransform)>,
    mut pixfx: ResMut<Pixfx>,
) {
    if !buttons.just_pressed(MouseButton::Left) {
        return;
    }
    let (Ok(window), Ok((camera, at))) = (windows.single(), cameras.single()) else {
        return;
    };
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    if let Ok(p) = camera.viewport_to_world_2d(at, cursor) {
        play(&mut pixfx, "explosions/body-burst", p.round());
    }
}

/// What the effects say: sounds are the game's to play.
fn listen(mut events: MessageReader<PixfxEvent>) {
    for PixfxEvent(e) in events.read() {
        info!("{e:?}");
    }
}

fn shoot(
    mut commands: Commands,
    mut shot: ResMut<Shot>,
    mut pixfx: ResMut<Pixfx>,
    time: Res<Time>,
    mut exit: MessageWriter<AppExit>,
) {
    let Some(path) = shot.path.clone() else {
        return;
    };
    let before = shot.secs;
    shot.secs += time.delta_secs();
    // the shaders compile in the background over the first frames
    if before < 1.6 && shot.secs >= 1.6 {
        play(&mut pixfx, "explosions/body-burst", Vec2::new(-80.0, 20.0));
    }
    if before < 2.0 && shot.secs >= 2.0 {
        commands.spawn(Screenshot::primary_window()).observe(save_to_disk(path));
    }
    if shot.secs >= 3.0 {
        exit.write(AppExit::Success);
    }
}

fn debug_dump(pixfx: Res<Pixfx>, q: Query<(&Transform, &ViewVisibility, &MeshMaterial2d<pixfx_bevy::ColorLayer>)>, mats: Res<Assets<pixfx_bevy::ColorLayer>>, images: Res<Assets<Image>>, mut n: Local<u32>) {
    *n += 1;
    if *n % 30 != 5 { return; }
    eprintln!("DBG shown={} quads={}", pixfx.shown().len(), q.iter().count());
    for (t, vv, m) in q.iter().take(4) {
        let mat = mats.get(&m.0);
        let img = mat.and_then(|m| images.get(&m.texture));
        let alpha = img.and_then(|i| i.data.as_ref()).map(|d| d.chunks(4).filter(|p| p[3] != 0).count());
        eprintln!("DBG t={:?} s={:?} vv={vv:?} mat={} img={} alpha={alpha:?}", t.translation, t.scale, mat.is_some(), img.is_some());
    }
}
