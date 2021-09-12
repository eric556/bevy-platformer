use animation::{AnimationPlugin, Col, Row, SpriteSheetDefinition};
use bevy::{math::Vec3Swizzles, prelude::*};
use bevy_canvas::{
    common_shapes::{self, Circle, Rectangle},
    Canvas, DrawMode,
};
use bevy_rapier2d::{physics::{
        ColliderBundle, ColliderPositionSync, NoUserData, RapierConfiguration, RapierPhysicsPlugin,
        RigidBodyBundle,
    }, prelude::{ColliderFlags, ColliderPosition, ColliderShape, ColliderType, InteractionGroups, RigidBodyForces, RigidBodyMassPropsFlags, RigidBodyType, RigidBodyVelocity}};
use fastapprox::fast::ln;
use player::PlayerPlugin;

use crate::{animation::{AnimatedSpriteBundle, AnimationDefinition}, player::{Health, PlayerBundle, PlayerStats}};

pub mod player;
pub mod animation;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
enum GameState {
    MainMenu,
    InGame,
    Paused,
    GameOver,
}

#[derive(Default)]
struct GameResource {
    score: u32,
}

struct MainCamera;
struct CameraTarget;

pub static GROUND_GROUP: u32 = 0b0001;
pub static ENTITY_GROUP: u32 = 0b0010;
pub static SHAPE_CAST_GROUP: u32 = 0b0100;

fn sprite_flip(mut sprite_query: Query<(&RigidBodyVelocity, &mut TextureAtlasSprite)>) {
    for (vel, mut sprite) in sprite_query.iter_mut() {
        if vel.linvel.x < 0.0 {
            sprite.flip_x = true;
        } else if vel.linvel.x > 0.0 {
            sprite.flip_x = false;
        }
    }
}

fn move_camera(
    target_query: Query<&Transform, With<CameraTarget>>,
    mut camera_query: Query<&mut Transform, (With<MainCamera>, Without<CameraTarget>)>,
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
        // println!("{}", t.clamp(0.0, 1.0));
        let new_position = transform.translation.xy().lerp(centorid, t.clamp(0.0, 1.0));
        transform.translation = Vec3::new(new_position.x, new_position.y, z);
        // }
    }
}

fn debug_colliders(
    mut canvas: ResMut<Canvas>,
    rapier_params: Res<RapierConfiguration>,
    collider_shapes: Query<(&ColliderPosition, &ColliderShape)>,
) {
    for (col_pos, col_shape) in collider_shapes.iter() {
        if let Some(ball) = col_shape.as_ball() {
            canvas.draw(
                &Circle {
                    center: Vec2::from(col_pos.0.translation) * rapier_params.scale,
                    radius: ball.radius * rapier_params.scale,
                },
                DrawMode::stroke_1px(),
                Color::RED,
            );
        }

        if let Some(cuboid) = col_shape.as_cuboid() {
            canvas.draw(
                &Rectangle {
                    origin: Vec2::from(col_pos.0.translation) * rapier_params.scale,
                    extents: Vec2::from(cuboid.half_extents) * rapier_params.scale * 2.0,
                    anchor_point: common_shapes::RectangleAnchor::Center,
                },
                DrawMode::stroke_1px(),
                Color::RED,
            );
        }
    }
}

fn setup_game(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
    mut rapier_config: ResMut<RapierConfiguration>,
) {
    let hero_char_texture_sheet_handle = asset_server.load("herochar_spritesheet.png");
    let hero_char_atlas = TextureAtlas::from_grid(hero_char_texture_sheet_handle, Vec2::new(16.0, 16.0), 8, 15);
    let hero_char_texture_atalas_handle = texture_atlases.add(hero_char_atlas);

    let hero_char_animation_definitions: Vec<AnimationDefinition> = vec![
        AnimationDefinition { name: String::from("death"), number_of_frames: 8, frame_time: 0.0, repeating: true },
        AnimationDefinition { name: String::from("run"), number_of_frames: 6, frame_time: 0.07, repeating: true },
        AnimationDefinition { name: String::from("pushing"), number_of_frames: 6, frame_time: 0.1, repeating: true },
        AnimationDefinition { name: String::from("attack_no_slash"), number_of_frames: 4, frame_time: 0.1, repeating: false },
        // ? What should we do about long boy animations (multiframe)
        AnimationDefinition { name: String::from("attack_slash"), number_of_frames: 8, frame_time: 0.1, repeating: false },
        AnimationDefinition { name: String::from("idle"), number_of_frames: 4, frame_time: 0.1, repeating: true },
        AnimationDefinition { name: String::from("falling"), number_of_frames: 3, frame_time: 0.07, repeating: true },
        AnimationDefinition { name: String::from("jumping"), number_of_frames: 3, frame_time: 0.07, repeating: true },
    ];

    // let map_handle: Handle<TiledMap> = asset_server.load("test_map.tmx");
    // let map_entity = commands.spawn().id();
    // commands.entity(map_entity)
    //     .insert_bundle(TiledMapBundle {
    //         tiled_map: map_handle,
    //         map: Map::new(0u16, map_entity),
    //         transform:  Transform::from_scale(Vec3::splat(4.0)),
    //         ..Default::default()
    //     });

    commands.insert_resource(GameResource { score: 0u32 });
    commands
        .spawn_bundle(OrthographicCameraBundle::new_2d())
        .insert(MainCamera);

    let sprite_size_x = 16.0;
    let sprite_size_y = 16.0;
    let sprite_scale = 4.0;
    rapier_config.scale = 8.0 * sprite_scale;
    let collider_size_x = (sprite_size_x * sprite_scale) / rapier_config.scale;
    let collider_size_y = (sprite_size_y * sprite_scale) / rapier_config.scale;

    println!("Collider size: {}, {}", collider_size_x, collider_size_y);

    commands
        .spawn_bundle(PlayerBundle {
            rigid_body: RigidBodyBundle {
                body_type: RigidBodyType::Dynamic,
                forces: RigidBodyForces {
                    gravity_scale: 5.0,
                    ..Default::default()
                },
                position: Vec2::new(20.0, 10.0).into(),
                mass_properties: (RigidBodyMassPropsFlags::ROTATION_LOCKED).into(),
                ..Default::default()
            },
            collider: ColliderBundle {
                shape: ColliderShape::cuboid(collider_size_x / 2.0, collider_size_y / 2.0),
                flags: ColliderFlags {
                    collision_groups: InteractionGroups::new(ENTITY_GROUP, GROUND_GROUP),
                    ..Default::default()
                },
                position: [collider_size_x / 2.0, collider_size_y / 2.0].into(),
                ..Default::default()
            },
            health: Health(10u32),
            player_stats: PlayerStats {
                max_run_speed: 20.0,
                speed_up: 5.0,
            },
            animation: AnimatedSpriteBundle {
                sprite_sheet: SpriteSheetBundle {
                    texture_atlas: hero_char_texture_atalas_handle,
                    transform: Transform::from_scale(Vec3::splat(sprite_scale)),
                    ..Default::default()
                },                
                sprite_sheet_definitions: SpriteSheetDefinition { animation_definitions: hero_char_animation_definitions, rows: 15, columns: 8 },
                animation_timer: Timer::from_seconds(0.1, true),
                current_row: Row(5), // Set it up as the idle animation right away
                current_col: Col(0)
            },
            ..Default::default()
        })
        .insert(CameraTarget)
        .insert(ColliderPositionSync::Discrete);

    commands.spawn_bundle(ColliderBundle {
        collider_type: ColliderType::Solid,
        shape: ColliderShape::cuboid(40.0, 0.5),
        flags: ColliderFlags {
            collision_groups: InteractionGroups::new(GROUND_GROUP, ENTITY_GROUP | SHAPE_CAST_GROUP),
            ..Default::default()
        },
        ..Default::default()
    });

    commands.spawn_bundle(ColliderBundle {
        collider_type: ColliderType::Solid,
        shape: ColliderShape::cuboid(1.0, 1.0),
        position: [-1.0, 1.5].into(),
        flags: ColliderFlags {
            collision_groups: InteractionGroups::new(GROUND_GROUP, ENTITY_GROUP | SHAPE_CAST_GROUP),
            ..Default::default()
        },
        ..Default::default()
    });

    commands.spawn_bundle(ColliderBundle {
        collider_type: ColliderType::Solid,
        shape: ColliderShape::cuboid(1.0, 0.5),
        position: [-4.0, 4.5].into(),
        flags: ColliderFlags {
            collision_groups: InteractionGroups::new(GROUND_GROUP, ENTITY_GROUP | SHAPE_CAST_GROUP),
            ..Default::default()
        },
        ..Default::default()
    });

    commands.spawn_bundle(ColliderBundle {
        collider_type: ColliderType::Solid,
        shape: ColliderShape::cuboid(1.0, 1.0),
        position: [-7.0, 1.5].into(),
        flags: ColliderFlags {
            collision_groups: InteractionGroups::new(GROUND_GROUP, ENTITY_GROUP | SHAPE_CAST_GROUP),
            ..Default::default()
        },
        ..Default::default()
    });
}

fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup_game.system())
        .add_plugin(bevy_canvas::CanvasPlugin)
        .add_plugin(RapierPhysicsPlugin::<NoUserData>::default())
        .add_plugin(AnimationPlugin)
        .add_plugin(PlayerPlugin)
        .add_system(move_camera.system())
        .add_system(debug_colliders.system())
        .add_system(sprite_flip.system())
        .run();
}
