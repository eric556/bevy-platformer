use core::panic;
use std::collections::HashMap;

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
use ldtk_rust::{EntityInstance, Project, TileInstance};
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

#[derive(Clone)]
struct LdtkMapAssets(HashMap<i32, Handle<TextureAtlas>>);

struct Map {
    ldtk_file: Project,
    redraw: bool,
    current_level: usize,
}

#[derive(Clone, Copy)]
struct LayerInfo {
    grid_width: i32,
    _grid_height: i32,
    grid_cell_size: i32,
    z_index: i32,
    px_width: f32,
    px_height: f32,
}

#[derive(Copy, Clone)]
struct ExtraEntDefs {
    __tile_id: i32,
    __width: i32,
    __height: i32,
    __scale: f32,
}

impl ExtraEntDefs {
    fn new() -> Self {
        Self {
            __tile_id: 0,
            __width: 0,
            __height: 0,
            __scale: 1.0,
        }
    }
}

struct MainCamera;
struct CameraTarget;

pub static GROUND_GROUP: u32 = 0b0001;
pub static ENTITY_GROUP: u32 = 0b0010;
pub static SHAPE_CAST_GROUP: u32 = 0b0100;

// LDtk provides pixel locations starting in the top left. For Bevy we need to
// flip the Y axis and offset from the center of the screen.
fn convert_to_world(
    width: f32,
    height: f32,
    grid_size: i32,
    scale: f32,
    x: i32,
    y: i32,
    z: i32,
) -> Vec3 {
    let world_x = (x as f32 * scale) + (grid_size as f32 * scale / 2.) - (width / 2.);
    let world_y = -(y as f32 * scale) - (grid_size as f32 * scale / 2.) + (height / 2.);
    let world_z = z as f32;
    Vec3::new(world_x, world_y, world_z)
}

// Bevy doesn't have sprite flipping built in, so if tile needs to flip
//  on either axis, flip it
fn flip(x: bool, y: bool) -> Quat {
    let mut q1 = Quat::default();
    let mut q2 = Quat::default();
    if x {
        q1 = Quat::from_rotation_y(std::f32::consts::PI);
    }
    if y {
        q2 = Quat::from_rotation_x(std::f32::consts::PI);
    }
    q1 * q2
}

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

fn setup_tilemap(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>
) {

    // Load up the map
    let map = Map {
        ldtk_file: Project::new(String::from("assets/test-world.ldtk")),
        redraw: true,
        current_level: 0,
    };

    // Go through and grab all the map tile sets
    let mut map_assets = LdtkMapAssets(HashMap::new());
    for tileset in map.ldtk_file.defs.tilesets.iter() {
        let texture_handle = asset_server.load(&tileset.rel_path[..]);

        let texture_atlas = TextureAtlas::from_grid(
            texture_handle,
            Vec2::new(tileset.tile_grid_size as f32, tileset.tile_grid_size as f32),
            (tileset.px_wid / tileset.tile_grid_size) as usize,
            (tileset.px_hei / tileset.tile_grid_size) as usize,
        );
        let texture_atlas_handle = texture_atlases.add(texture_atlas);
        map_assets.0.insert(tileset.uid as i32, texture_atlas_handle);
    }

    // Slap these bad boys into resources
    commands.insert_resource(map);
    commands.insert_resource(map_assets);
}

fn setup_game(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
    mut rapier_config: ResMut<RapierConfiguration>,
) {




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

// Spawn a tile. Check to see if it needs to flip on the x and/or y axis before spawning.
fn display_tile(
    layer_info: LayerInfo,
    tile: &TileInstance,
    commands: &mut Commands,
    handle: Handle<TextureAtlas>,
) {
    let mut flip_x = false;
    let mut flip_y = false;
    match tile.f {
        1 => flip_x = true,
        2 => flip_y = true,
        3 => {
            flip_x = true;
            flip_y = true
        }
        _ => (),
    }
    commands.spawn().insert_bundle(SpriteSheetBundle {
        transform: Transform {
            translation: convert_to_world(
                layer_info.px_width,
                layer_info.px_height,
                layer_info.grid_cell_size,
                4.0,
                tile.px[0] as i32,
                tile.px[1] as i32,
                layer_info.z_index,
            ),
            rotation: flip(flip_x, flip_y),
            scale: Vec3::splat(4.0),
        },
        sprite: TextureAtlasSprite::new(tile.t as u32),
        texture_atlas: handle,
        ..Default::default()
    });
}

pub fn convert_ldtk_entity_to_bevy(position: Vec2, size: Vec2,  layer_size: Vec2, scale: f32) -> (Vec2, Vec2){
    let ldtk_half_extents = size / 2.0;
    let ldtk_position = position + ldtk_half_extents;

    (
        Vec2::new(
            (ldtk_position.x * scale) - (layer_size.x / 2.),
            -(ldtk_position.y * scale)  + (layer_size.y / 2.)
        ), 
        ldtk_half_extents * scale
    )
}

fn update_ldtk_map(
    mut commands: Commands,
    mut map: ResMut<Map>,
    map_assets: Res<LdtkMapAssets>,
    asset_server: Res<AssetServer>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
    mut rapier_config: ResMut<RapierConfiguration>,
) {
    if !map.redraw {
        return
    }

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

    commands.insert_resource(ClearColor(
        Color::hex(&map.ldtk_file.levels[0].bg_color[1..]).unwrap(),
    ));

    for (idx, layer) in map.ldtk_file.levels[map.current_level].layer_instances.as_ref().unwrap().iter().enumerate().rev() {
        let tileset_uid = layer.tileset_def_uid.unwrap_or(-1) as i32;
        let layer_uid = layer.layer_def_uid as i32;

        let layer_info = LayerInfo {
            grid_width: layer.c_wid as i32,
            _grid_height: layer.c_hei as i32,
            grid_cell_size: layer.grid_size as i32,
            z_index: 50 - idx as i32,
            // todo gotta swap this over from a hard coded scale
            px_width: layer.c_wid as f32 * (layer.grid_size as f32 * 4.0),
            px_height: layer.c_hei as f32 * (layer.grid_size as f32 * 4.0),
        };

        match &layer.layer_instance_type[..] {
            "Tiles" => {

            }
            "AutoLayer" => {

            }
            "IntGrid" => {
                if let Some(layer_tileset_def_uid) = layer.tileset_def_uid {
                    println!("Generating IntGrid Layer w/ Tiles: {}", layer.identifier);
                    for tile in layer.auto_layer_tiles.iter() {
                        display_tile(
                            layer_info,
                            tile,
                            &mut commands,
                            map_assets.0[&(layer_tileset_def_uid as i32)].clone()
                        )
                    } 
                }
            }
            "Entities" => {
                println!("Generating Entities Layer: {}", layer.identifier);
                match &layer.identifier[..] {
                    "Colliders" => {
                        for entity in layer.entity_instances.iter() {
                            let (bevy_pos, bevy_half_extent) = convert_ldtk_entity_to_bevy(
                                Vec2::new(entity.px[0] as f32, entity.px[1] as f32), 
                                Vec2::new(entity.width as f32, entity.height as f32), 
                                Vec2::new(layer_info.px_width, layer_info.px_height), 
                                4.0
                            );

                            let rapier_half_extents = bevy_half_extent / rapier_config.scale;
                            let rapier_position = bevy_pos / rapier_config.scale;

                            println!("Creating collider Size({:?}) Position({:?})", rapier_half_extents, rapier_position);

                            commands.spawn_bundle(ColliderBundle {
                                collider_type: ColliderType::Solid,
                                shape: ColliderShape::cuboid(rapier_half_extents.x, rapier_half_extents.y),
                                position: rapier_position.into(),
                                flags: ColliderFlags {
                                    collision_groups: InteractionGroups::new(GROUND_GROUP, ENTITY_GROUP | SHAPE_CAST_GROUP),
                                    ..Default::default()
                                },
                                ..Default::default()
                            });
                        }
                    }
                    "Entities" => {
                        for entity in layer.entity_instances.iter() {
                            println!("Entity: {}", entity.identifier);

                            let (mut bevy_pos, bevy_half_extent) = convert_ldtk_entity_to_bevy(
                                Vec2::new(entity.px[0] as f32, entity.px[1] as f32), 
                                Vec2::new(entity.width as f32, entity.height as f32), 
                                Vec2::new(layer_info.px_width, layer_info.px_height), 
                                4.0
                            );

                            // todo idk why but I have to undo the half extent subtraction?
                            bevy_pos = bevy_pos - bevy_half_extent;
                            let rapier_half_extents = bevy_half_extent / rapier_config.scale;
                            let rapier_position = bevy_pos / rapier_config.scale;

                            println!("Spawning at position: {:?} {:?}", bevy_pos, bevy_half_extent);

                            match &entity.identifier[..] {
                                "Player" => {
                                    commands
                                    .spawn_bundle(PlayerBundle {
                                        rigid_body: RigidBodyBundle {
                                            body_type: RigidBodyType::Dynamic,
                                            forces: RigidBodyForces {
                                                gravity_scale: 5.0,
                                                ..Default::default()
                                            },
                                            position: rapier_position.into(),
                                            mass_properties: (RigidBodyMassPropsFlags::ROTATION_LOCKED).into(),
                                            ..Default::default()
                                        },
                                        collider: ColliderBundle {
                                            shape: ColliderShape::cuboid(rapier_half_extents.x, rapier_half_extents.y),
                                            flags: ColliderFlags {
                                                collision_groups: InteractionGroups::new(ENTITY_GROUP, GROUND_GROUP),
                                                ..Default::default()
                                            },
                                            position: rapier_half_extents.into(),
                                            ..Default::default()
                                        },
                                        health: Health(10u32),
                                        player_stats: PlayerStats {
                                            max_run_speed: 20.0,
                                            speed_up: 5.0,
                                        },
                                        animation: AnimatedSpriteBundle {
                                            sprite_sheet: SpriteSheetBundle {
                                                texture_atlas: hero_char_texture_atalas_handle.clone(),
                                                transform: Transform::from_scale(Vec3::splat(sprite_scale)),
                                                ..Default::default()
                                            },                
                                            sprite_sheet_definitions: SpriteSheetDefinition { animation_definitions: hero_char_animation_definitions.clone(), rows: 15, columns: 8 },
                                            animation_timer: Timer::from_seconds(0.1, true),
                                            current_row: Row(5), // Set it up as the idle animation right away
                                            current_col: Col(0)
                                        },
                                        ..Default::default()
                                    })
                                    .insert(CameraTarget)
                                    .insert(ColliderPositionSync::Discrete);
                                }
                                _ => {}
                            }

                        }

                    }
                    _ => {}
                }

            }
            _ => panic!("AHHHHHHHHH")
        }
    }

    map.redraw = false;
}

fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup_game.system())
        .add_startup_system(setup_tilemap.system())
        .add_plugin(bevy_canvas::CanvasPlugin)
        .add_plugin(RapierPhysicsPlugin::<NoUserData>::default())
        .add_plugin(AnimationPlugin)
        .add_plugin(PlayerPlugin)
        .add_system(update_ldtk_map.system())
        .add_system(move_camera.system())
        .add_system(debug_colliders.system())
        .add_system(sprite_flip.system())
        .run();
}
