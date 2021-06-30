use bevy::{ecs::storage::TableMoveResult, input::mouse::MouseMotion, math::Vec4Swizzles, prelude::*};
use bevy_canvas::{Canvas, CanvasPlugin, DrawMode, common_shapes::{Circle, Line}};
use bevy_ecs_tilemap::prelude::*;
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
    idle_texture_atlas: Handle<TextureAtlas>,
    run_texture_atlas: Handle<TextureAtlas>
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

#[derive(PartialEq)]
enum PlayerState {
    Idle,
    Running,
}

impl Default for PlayerState {
    fn default() -> Self {
        Self::Idle
    }
}

#[derive(Default)]
pub struct PlayerStats {
    max_run_speed: f32,
    speed_up: f32
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
    input: PlayerInput,
    state: PlayerState,
    player_stats: PlayerStats
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

fn update_player_animation(
    player_texture_handles: Res<PlayerTextureAtlasHandles>,
    texture_atlases: Res<Assets<TextureAtlas>>,
    mut player_query: Query<(&Velocity, &PlayerState, &mut Timer, &mut TextureAtlasSprite, &mut Handle<TextureAtlas>), Changed<PlayerState>>
) {
    for (vel, state, mut timer, mut sprite, mut current_atlas_handle) in player_query.iter_mut() {
        match state {
            PlayerState::Idle => {
                *current_atlas_handle = player_texture_handles.idle_texture_atlas.clone_weak();
                *timer = Timer::from_seconds(0.1, true);
            },
            PlayerState::Running => {
                *current_atlas_handle = player_texture_handles.run_texture_atlas.clone_weak();
                *timer = Timer::from_seconds(0.07, true);
                if vel.0.x > 0.0 {
                    sprite.flip_x = false;
                } else {
                    sprite.flip_x = true;
                }
            },
        }

        if let Some(current_atlas) = texture_atlases.get(current_atlas_handle.clone_weak()) {
            if sprite.index as usize > current_atlas.len() {
                sprite.index = 0;
            }
        }
    }
}

fn move_player(
    keys: Res<Input<KeyCode>>,
    mut player_query: Query<(&PlayerInput, &PlayerStats, &mut Velocity, &mut PlayerState)>
) {
    for (p_input, player_stats, mut vel, mut state) in player_query.iter_mut() {
        if keys.pressed(p_input.left) {
            if vel.0.x > 0.0 { vel.0.x = 0.0; }
            vel.0.x -= player_stats.speed_up;
            vel.0.x = vel.0.x.max(-player_stats.max_run_speed);
        }

        if keys.pressed(p_input.right) {
            if vel.0.x < 0.0 {
                vel.0.x = 0.0;
            }
            vel.0.x += player_stats.speed_up;
            vel.0.x = vel.0.x.min(player_stats.max_run_speed);
        }

        if !keys.pressed(p_input.left) && !keys.pressed(p_input.right) {
            vel.0.x = 0.0;
        }

        match *state {
            PlayerState::Idle => {
                if vel.0.x != 0.0 {
                    *state = PlayerState::Running;
                }
            },
            PlayerState::Running => {
                if vel.0.x == 0.0 {
                    *state = PlayerState::Idle;
                }
            },
        }

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

    let run_texture_sheet_handle = asset_server.load("herochar_run_anim_strip_6.png");
    let run_texture_atlas = TextureAtlas::from_grid(run_texture_sheet_handle, Vec2::new(16.0, 16.0), 6, 1);
    let run_texture_handle = texture_atlases.add(run_texture_atlas);

    let texture_handles = PlayerTextureAtlasHandles {
        idle_texture_atlas: idle_texture_handle.clone(),
        run_texture_atlas: run_texture_handle.clone()
    };

    // let map_handle: Handle<TiledMap> = asset_server.load("test_map.tmx");
    // let map_entity = commands.spawn().id();
    // commands.entity(map_entity)
    //     .insert_bundle(TiledMapBundle {
    //         tiled_map: map_handle,
    //         map: Map::new(0u16, map_entity),
    //         transform:  Transform::from_scale(Vec3::splat(4.0)),
    //         ..Default::default()
    //     });

    commands.insert_resource(GameResource{ score: 0u32 });
    commands.spawn_bundle(OrthographicCameraBundle::new_2d()).insert(MainCamera);

    commands.spawn_bundle(PlayerBundle{
        kbody: KinematicBundle {
            ..Default::default()
        },
        health: Health(10u32),
        animation_timer: Timer::from_seconds(0.1, true),
        bounding_box: Collider::Box(BoxCollider { position: Vec2::new(0.0, 0.0), half_size: Vec2::new(32f32, 32f32)}),
        player_stats: PlayerStats {
            max_run_speed: 500.0,
            speed_up: 100.0,
        },
        sprite_sheet: SpriteSheetBundle {
            texture_atlas: texture_handles.idle_texture_atlas.clone_weak(),
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
            half_size: Vec2::new(500.0, 100.0)
        })
    )).id();

    commands.insert_resource(texture_handles);
}

fn main() {
    App::build()
    .add_plugins(DefaultPlugins)
    .add_startup_system(setup_game.system())
    .add_plugin(bevy_canvas::CanvasPlugin)
    .add_plugin(KinematicsPlugin)
    .add_plugin(DebugCollidersPlugin)
    .add_plugin(TilemapPlugin)
    .add_plugin(TiledMapPlugin)
    .add_system(animate_sprite_system.system())
    .add_system(move_player.system().before(PHYSICS_UPDATE))
    .add_system(gravity.system().before(PHYSICS_UPDATE))
    .add_system(update_player_animation.system().after(PHYSICS_UPDATE))
    .run();
}