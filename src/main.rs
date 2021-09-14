use animation::{AnimationPlugin, Col, Row, SpriteSheetDefinition};
use bevy::{math::Vec3Swizzles, prelude::*};
use bevy_canvas::{
    common_shapes::{self, Circle, Rectangle},
    Canvas, DrawMode,
};
use heron::prelude::*;
use fastapprox::fast::ln;

use player::PlayerPlugin;
use crate::{animation::{AnimatedSpriteBundle, AnimationDefinition}, player::{Health, PlayerBundle, PlayerInput, PlayerStats}};

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

#[derive(PhysicsLayer)]
enum Layer {
    World,
    Player,
    Enemies,
}

fn sprite_flip(mut sprite_query: Query<(&Velocity, &mut TextureAtlasSprite)>) {
    for (vel, mut sprite) in sprite_query.iter_mut() {
        if vel.linear.x < 0.0 {
            sprite.flip_x = true;
        } else if vel.linear.x > 0.0 {
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

fn update_position(
    grav: Res<Gravity>,
    mut query: Query<(&mut Velocity, &mut Transform)>
) {
    for (mut vel, mut trans) in query.iter_mut() {
        // println!("Vel: {:?}, transform: {:?}", vel, trans);
        // trans.translation += vel.linear;
    }
}

fn setup_game(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>
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
    let collider_size_x = sprite_size_x * sprite_scale;
    let collider_size_y = sprite_size_y * sprite_scale;

    // println!("Collider size: {}, {}", collider_size_x, collider_size_y);

    commands
        .spawn_bundle(PlayerBundle{
            rigid_body_type: RigidBody::KinematicVelocityBased,
            velocity: Velocity::from_linear(Vec3::ZERO),
            collision_shape: CollisionShape::Cuboid {
                half_extends: Vec3::new(collider_size_x / 2.0, collider_size_y / 2.0, 0.0),
                border_radius: None
            },
            collision_layers: CollisionLayers::none().with_group(Layer::Player).with_mask(Layer::World),
            health: Health(10u32),
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
            player_stats: PlayerStats {
                max_run_speed: 50000.0,
                speed_up: 100.0,
            },
            ..Default::default()
        })
        // .insert(RotationConstraints::lock())
        .insert(CameraTarget);

    let size = Vec2::new(400.0, 10.0);
    commands
    // Spawn a bundle that contains at least a `GlobalTransform`
    .spawn_bundle(SpriteBundle {
        sprite: Sprite::new(size),
        material: materials.add(Color::WHITE.into()),
        transform: Transform::from_translation(Vec3::new(0.0, -300.0, 0.0)),
        ..Default::default()
    })
    // Make it a rigid body
    .insert(RigidBody::KinematicVelocityBased)
    // Attach a collision shape
    .insert(CollisionShape::Cuboid {
        half_extends: size.extend(0.0) / 2.0,
        border_radius: None,
    })
    // Define restitution (so that it bounces)
    .insert(PhysicMaterial {
        restitution: 0.5,
        ..Default::default()
    });
    // commands.spawn_bundle(ColliderBundle {
    //     collider_type: ColliderType::Solid,
    //     shape: ColliderShape::cuboid(40.0, 0.5),
    //     flags: ColliderFlags {
    //         collision_groups: InteractionGroups::new(GROUND_GROUP, ENTITY_GROUP | SHAPE_CAST_GROUP),
    //         ..Default::default()
    //     },
    //     ..Default::default()
    // });

    // commands.spawn_bundle(ColliderBundle {
    //     collider_type: ColliderType::Solid,
    //     shape: ColliderShape::cuboid(1.0, 1.0),
    //     position: [-1.0, 1.5].into(),
    //     flags: ColliderFlags {
    //         collision_groups: InteractionGroups::new(GROUND_GROUP, ENTITY_GROUP | SHAPE_CAST_GROUP),
    //         ..Default::default()
    //     },
    //     ..Default::default()
    // });

    // commands.spawn_bundle(ColliderBundle {
    //     collider_type: ColliderType::Solid,
    //     shape: ColliderShape::cuboid(1.0, 0.5),
    //     position: [-4.0, 4.5].into(),
    //     flags: ColliderFlags {
    //         collision_groups: InteractionGroups::new(GROUND_GROUP, ENTITY_GROUP | SHAPE_CAST_GROUP),
    //         ..Default::default()
    //     },
    //     ..Default::default()
    // });

    // commands.spawn_bundle(ColliderBundle {
    //     collider_type: ColliderType::Solid,
    //     shape: ColliderShape::cuboid(1.0, 1.0),
    //     position: [-7.0, 1.5].into(),
    //     flags: ColliderFlags {
    //         collision_groups: InteractionGroups::new(GROUND_GROUP, ENTITY_GROUP | SHAPE_CAST_GROUP),
    //         ..Default::default()
    //     },
    //     ..Default::default()
    // });
}

fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .add_plugin(bevy_canvas::CanvasPlugin)
        .add_plugin(PhysicsPlugin::default())
        .insert_resource(Gravity::from(Vec3::new(0.0, -9.81, 0.0)))
        .add_plugin(AnimationPlugin)
        .add_plugin(PlayerPlugin)
        .add_startup_system(setup_game.system())
        // .add_system(move_camera.system())
        .add_system(update_position.system())
        // .add_system(debug_colliders.system())
        // .add_system(sprite_flip.system())
        .run();
}
