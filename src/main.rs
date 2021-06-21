use bevy::{ecs::storage::TableMoveResult, prelude::*};
use bevy_canvas::CanvasPlugin;
use collision::{AABB, CollisionPlugin, DebugCollisionPlugin};
use kinematic::*;

pub mod kinematic;
pub mod collision;


#[derive(Debug, Clone, Eq, PartialEq, Hash)]
enum GameState {
    MainMenu,
    InGame,
    Paused,
    GameOver
}

#[derive(Default)]
struct GameResource {
    score: u32
}

struct PlayerTextureAtlasHandles {
    idle_texture_atlas: Handle<TextureAtlas>
}

#[derive(Default)]
struct Health(u32);

struct PlayerInput {
    left: KeyCode,
    right: KeyCode,
    jump: KeyCode,
    crouch: KeyCode
}

impl Default for PlayerInput {
    fn default() -> Self {
        PlayerInput {
            left: KeyCode::A,
            right: KeyCode::D,
            jump: KeyCode::W,
            crouch: KeyCode::S
        }
    }
}

#[derive(Bundle, Default)]
struct PlayerBundle {
    health: Health,
    #[bundle]
    kbody: KinematicBundle,
    #[bundle]
    sprite_sheet: SpriteSheetBundle,
    animation_timer: Timer,
    bounding_box: AABB,
    input: PlayerInput
}

fn animate_sprite_system(
    time: Res<Time>,
    texture_atlases: Res<Assets<TextureAtlas>>,
    mut query: Query<(&mut Timer, &mut TextureAtlasSprite, &Handle<TextureAtlas>)>,
) {
    for (mut timer, mut sprite, texture_atlas_handle) in query.iter_mut() {
        timer.tick(time.delta());
        if timer.finished() {
            let texture_atlas_option = texture_atlases.get(texture_atlas_handle);            
            if texture_atlas_option.is_some() {
                sprite.index = ((sprite.index as usize + 1) % texture_atlas_option.unwrap().textures.len()) as u32;

            }
        }
    }
}

fn gravity(
    mut kinematic_query: Query<&mut Acceleration>
) {
    for mut accel in kinematic_query.iter_mut() {
        accel.0.y -= 9.81;
    }
}

fn move_player(
    keys: Res<Input<KeyCode>>,
    mut player_query: Query<(&PlayerInput, &mut Acceleration)>
) {
    for (p_input, mut accel) in player_query.iter_mut() {
        if keys.pressed(p_input.left) {
            accel.0.x -= 2.0;
        }
        if keys.pressed(p_input.right) {
            accel.0.x += 2.0;
        }
        if keys.pressed(p_input.crouch) {
            accel.0.y -= 2.0;
        }
        if keys.pressed(p_input.jump) {
            accel.0.y += 2.0;
        }
    }
}

fn drag(
    mut accel_query: Query<(&Velocity, &mut Acceleration)>
) {
    for (vel, mut accel) in accel_query.iter_mut() {
        if vel.0.normalize_or_zero() != Vec2::ZERO {
            accel.0 -= vel.0.normalize_or_zero();
        }
    }
}

fn setup_game(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>
) {
    let idle_texture_sheet_handle = asset_server.load("herochar_idle_anim_strip_4.png");
    let idle_texture_atlas = TextureAtlas::from_grid(idle_texture_sheet_handle, Vec2::new(16.0, 16.0), 4, 1);
    let idle_texture_handle = texture_atlases.add(idle_texture_atlas);

    commands.insert_resource(GameResource{ score: 0u32 });
    commands.insert_resource(PlayerTextureAtlasHandles {
        idle_texture_atlas: idle_texture_handle.clone_weak()
    });
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());

    commands.spawn_bundle(PlayerBundle{
        health: Health(10u32),
        animation_timer: Timer::from_seconds(0.1, true),
        bounding_box: AABB { position: Vec2::new(0.0, 0.0), half_size: Vec2::new(32f32, 32f32)},
        sprite_sheet: SpriteSheetBundle {
            texture_atlas: idle_texture_handle.clone(),
            transform: Transform::from_scale(Vec3::splat(4.0)),
            ..Default::default()
        },
        ..Default::default()
    });

    commands.spawn_bundle(PlayerBundle{
        kbody: KinematicBundle {
            position: Position(Vec2::new(100.0, 50.0)),
            ..Default::default()
        },
        health: Health(10u32),
        animation_timer: Timer::from_seconds(0.1, true),
        bounding_box: AABB { position: Vec2::new(0.0, 0.0), half_size: Vec2::new(32f32, 32f32)},
        sprite_sheet: SpriteSheetBundle {
            texture_atlas: idle_texture_handle.clone(),
            transform: Transform::from_scale(Vec3::splat(4.0)),
            ..Default::default()
        },
        input: PlayerInput {
            left: KeyCode::J,
            right: KeyCode::L,
            jump: KeyCode::I,
            crouch: KeyCode::K
        },
        ..Default::default()
    });

    let temp = commands.spawn_bundle((
        Position(Vec2::new(0.0, -300.0)), 
        AABB {
            position: Vec2::new(0.0, 0.0),
            half_size: Vec2::new(250.0, 10.0)
        }
    )).id();

    println!("Spawned entity {:?}", temp);
}

fn destruct_game(
    mut commands: Commands
) {
    commands.remove_resource::<GameResource>();
}

fn main() {
    App::build()
    .add_plugins(DefaultPlugins)
    .add_startup_system(setup_game.system())
    .add_plugin(bevy_canvas::CanvasPlugin)
    .add_plugin(KinematicsPlugin)
    .add_plugin(CollisionPlugin)
    .add_plugin(DebugCollisionPlugin)
    .add_system(animate_sprite_system.system())
    .add_system(move_player.system())
    .add_system(drag.system())
    // .add_system(gravity.system())
    .run();
}