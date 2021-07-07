use bevy::{ecs::storage::TableMoveResult, input::mouse::MouseMotion, math::{Vec3Swizzles, Vec4Swizzles}, prelude::*, render::camera::Camera};
use bevy_canvas::{Canvas, CanvasPlugin, DrawMode, common_shapes::{Circle, Line}};
use bevy_easings::{Ease, EaseFunction, EasingsPlugin};
use bevy_ecs_tilemap::prelude::*;
use kinematic::{PHYSICS_UPDATE, colliders::{Collider, DebugCollidersPlugin}, kinematic::{KinematicsPlugin, Velocity}};
use fastapprox::fast::{self, exp, ln, sigmoid};

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
struct CameraTarget;

struct PlayerTextureAtlasHandles {
    idle_texture_atlas: Handle<TextureAtlas>,
    run_texture_atlas: Handle<TextureAtlas>,
    pre_jump_texture_atlaas: Handle<TextureAtlas>,
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
    Jumping,
    Falling
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
                if vel.0.x.signum() > 0.0 {
                    sprite.flip_x = false;
                } else {
                    sprite.flip_x = true;
                }
            },
            _ => todo!("Implement rest of player state animations")
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
        let prev_vel_sign = vel.0.x.signum();

        if (!keys.pressed(p_input.left) && !keys.pressed(p_input.right)) || (keys.pressed(p_input.left) && keys.pressed(p_input.right)) {
            vel.0.x = 0.0;
        } else if keys.pressed(p_input.left) {
            if prev_vel_sign > 0.0 { vel.0.x = 0.0; }
            vel.0.x -= player_stats.speed_up;
            vel.0.x = vel.0.x.max(-player_stats.max_run_speed);
        } else if keys.pressed(p_input.right) {
            if prev_vel_sign < 0.0 { vel.0.x = 0.0; }
            vel.0.x += player_stats.speed_up;
            vel.0.x = vel.0.x.min(player_stats.max_run_speed);
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

                // reset the running animation
                if vel.0.x.signum() != prev_vel_sign {
                    *state = PlayerState::Running;
                }
            },
            _ => todo!("Implement rest of player state")
        }

        if keys.pressed(p_input.jump) {
            if vel.0.y < 400.0 {
                vel.0.y += 100.0;
            }
        }
    }
}

fn move_camera (
    target_query: Query<&Transform, With<CameraTarget>>,
    mut camera_query: Query<&mut Transform, (With<MainCamera>, Without<CameraTarget>)>
) {
    let mut centorid = Vec2::ZERO;
    let mut n = 0.0;
    for transform in target_query.iter() {
        centorid += transform.translation.xy();
        n += 1.0;
    }
    centorid /= n;

    for mut transform in camera_query.iter_mut() {
        let distance = centorid.distance(transform.translation.xy());
        let z = transform.translation.z;

        // let k = 1.5f32;
        // let b = -0.02f32;
        // let a = -0.1f32;
        // let c = -0.8f32;
        // let mut t = (k / (1.0f32 + exp(a+(distance * b)))) + c;

        let g = 0.0004f32;
        let l = -1.0f32;
        let t = ln(g * distance - l);

        // if t < 0.01 {
        //     println!("Not moving {}", t.clamp(0.0, 1.0));
        //     transform.translation = Vec3::new(centorid.x, centorid.y, z);
        // } else {
            println!("{}", t.clamp(0.0, 1.0));
            let new_position  = transform.translation.xy().lerp(centorid, t.clamp(0.0, 1.0));
            transform.translation = Vec3::new(new_position.x, new_position.y, z);
        // }


    }
}

fn load_animation(
    asset_server: Res<AssetServer>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
    texture_path: &str,    
    tile_size: Vec2,
    columns: usize,
    rows: usize
) -> Handle<TextureAtlas> {
    let texture_sheet_handle = asset_server.load(texture_path);
    let texture_atlas = TextureAtlas::from_grid(texture_sheet_handle, tile_size, columns, rows);
    let texture_handle = texture_atlases.add(texture_atlas);
    return texture_handle.clone();
}

fn setup_game(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>
) {

    let texture_handles = PlayerTextureAtlasHandles {
        idle_texture_atlas: load_animation(
            asset_server, 
            texture_atlases, 
            "herochar_idle_anim_strip_4.png", 
            Vec2::new(16.0, 16.0), 4, 1
        ),
        run_texture_atlas: load_animation(
            asset_server, 
            texture_atlases, 
            "herochar_run_anim_strip_6.png", 
            Vec2::new(16.0, 16.0), 6, 1
        ),
        pre_jump_texture_atlaas: load_animation(
            asset_server, 
            texture_atlases, 
            "herochar_before_or_after_jump_srip_2.png", 
            Vec2::new(16.0, 16.0), 2, 1
        ),
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
    }).insert(CameraTarget);

    commands.spawn_bundle(PlayerBundle{
        kbody: KinematicBundle {
            position: Position(Vec2::new(100.0, 50.0)),
            ..Default::default()
        },
        health: Health(10u32),
        animation_timer: Timer::from_seconds(0.1, true),
        bounding_box: Collider::Box(BoxCollider { position: Vec2::new(0.0, 0.0), half_size: Vec2::new(32f32, 32f32)}),
        sprite_sheet: SpriteSheetBundle {
            texture_atlas: texture_handles.idle_texture_atlas.clone_weak(),
            transform: Transform::from_scale(Vec3::splat(4.0)),
            ..Default::default()
        },
        player_stats: PlayerStats {
            max_run_speed: 500.0,
            speed_up: 100.0,
        },
        input: PlayerInput {
            left:   KeyCode::J,
            right:  KeyCode::L,
            jump:   KeyCode::I,
            crouch: KeyCode::K
        },
        ..Default::default()
    }).insert(CameraTarget);

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
    .add_plugin(EasingsPlugin)
    .add_system(animate_sprite_system.system())
    .add_system(move_player.system().before(PHYSICS_UPDATE))
    .add_system(gravity.system().before(PHYSICS_UPDATE))
    .add_system(update_player_animation.system().after(PHYSICS_UPDATE))
    .add_system(move_camera.system())
    .run();
}