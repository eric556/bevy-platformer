use bevy::{ecs::storage::TableMoveResult, input::mouse::MouseMotion, math::Vec4Swizzles, prelude::*};
use bevy_canvas::{Canvas, CanvasPlugin, DrawMode, common_shapes::{Circle, Line}};
use kinematic::{PHYSICS_UPDATE, colliders::{Collider, DebugCollidersPlugin}, kinematic::{KinematicsPlugin, Velocity}};

use crate::kinematic::{colliders::{BoxCollider}, kinematic::{KinematicBundle, Position}};

pub mod kinematic;

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

struct MainCamera;

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
            jump: KeyCode::Space,
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
    bounding_box: Collider,
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
    mut kinematic_query: Query<&mut Velocity>
) {
    for mut vel in kinematic_query.iter_mut() {
        vel.0.y -= 9.81;
    }
}

fn move_player(
    keys: Res<Input<KeyCode>>,
    mut player_query: Query<(&PlayerInput, &mut Velocity)>
) {
    for (p_input, mut vel) in player_query.iter_mut() {
        let mut movement = Vec2::ZERO;

        if keys.pressed(p_input.left) {
            movement.x = -300.0;
        } else if keys.pressed(p_input.right) {
            movement.x = 300.0;
        }

        vel.0.x = movement.x;

        if keys.pressed(p_input.jump) {
            if vel.0.y < 400.0 {
                vel.0.y += 100.0;
            }
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
    commands.spawn_bundle(OrthographicCameraBundle::new_2d()).insert(MainCamera);

    commands.spawn_bundle(PlayerBundle{
        kbody: KinematicBundle {
            ..Default::default()
        },
        health: Health(10u32),
        animation_timer: Timer::from_seconds(0.1, true),
        bounding_box: Collider::Box(BoxCollider { position: Vec2::new(0.0, 0.0), half_size: Vec2::new(32f32, 32f32)}),
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
        bounding_box: Collider::Box(BoxCollider { position: Vec2::new(0.0, 0.0), half_size: Vec2::new(32f32, 32f32)}),
        sprite_sheet: SpriteSheetBundle {
            texture_atlas: idle_texture_handle.clone(),
            transform: Transform::from_scale(Vec3::splat(4.0)),
            ..Default::default()
        },
        input: PlayerInput {
            left:   KeyCode::J,
            right:  KeyCode::L,
            jump:   KeyCode::I,
            crouch: KeyCode::K
        },
        ..Default::default()
    });

    let temp = commands.spawn_bundle((
        Position(Vec2::new(0.0, -300.0)), 
        Collider::Box(BoxCollider {
            position: Vec2::new(0.0, 0.0),
            half_size: Vec2::new(250.0, 200.0)
        })
    )).id();
}

fn main() {
    App::build()
    .add_plugins(DefaultPlugins)
    .add_startup_system(setup_game.system())
    .add_plugin(bevy_canvas::CanvasPlugin)
    .add_plugin(KinematicsPlugin)
    .add_plugin(DebugCollidersPlugin)
    .add_system(animate_sprite_system.system())
    .add_system(move_player.system().before(PHYSICS_UPDATE))
    .add_system(gravity.system().before(PHYSICS_UPDATE))
    .run();
}