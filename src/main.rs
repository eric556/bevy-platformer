use core::panic;
use std::collections::HashMap;

use animation::{AnimationPlugin, Col, Row, SpriteSheetDefinition};
use bevy::{math::Vec3Swizzles, prelude::*};
use bevy_canvas::{
    common_shapes::{self, Circle, Rectangle},
    Canvas, DrawMode,
};

use fastapprox::fast::ln;
use ldtk_rust::{EntityInstance, Project, TileInstance};
use physics::{DebugAABBPlugin, PhysicsPlugin, body::Velocity};
use player::PlayerPlugin;

use crate::{animation::{AnimatedSpriteBundle, AnimationDefinition}, physics::{body::{BodyBundle, BodyType, Position}, collision::AABB}};

pub mod player;
pub mod animation;
pub mod physics;

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

// fn sprite_flip(mut sprite_query: Query<(&RigidBodyVelocity, &mut TextureAtlasSprite)>) {
//     for (vel, mut sprite) in sprite_query.iter_mut() {
//         if vel.linvel.x < 0.0 {
//             sprite.flip_x = true;
//         } else if vel.linvel.x > 0.0 {
//             sprite.flip_x = false;
//         }
//     }
// }

fn gravity(
    keys: Res<Input<KeyCode>>,
    mut actor_query: Query<(&mut Velocity, &BodyType)> 
) {
    for (mut vel, body_type) in actor_query.iter_mut() {
        if *body_type == BodyType::Actor {
            if keys.pressed(KeyCode::A) {
                vel.0.x -= 2.0;
            }
            if keys.pressed(KeyCode::D) {
                vel.0.x += 2.0;
            }
            if keys.pressed(KeyCode::S) {
                vel.0.y -= 2.0;
            }
            if keys.pressed(KeyCode::W) {
                vel.0.y += 2.0;
            }
            // vel.0.y -= 9.81;
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
        // transform.translation = Vec3::new(new_position.x, new_position.y, z);
        // }
    }
}

fn setup_tilemap(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>
) {

    // Load up the map
    let map = Map {
        // ldtk_file: Project::new(String::from("assets/test-world.ldtk")),
        ldtk_file: Project::new(String::from("assets/physics-testing.ldtk")),
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

// Spawn a tile. Check to see if it needs to flip on the x and/or y axis before spawning.
fn display_tile(
    layer_info: LayerInfo,
    tile: &TileInstance,
    level_world_pos: Vec2,
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
                (tile.px[0] as f32 + level_world_pos.x) as i32,
                (tile.px[1] as f32 + level_world_pos.y) as i32,
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
    // rapier_config.scale = 8.0 * sprite_scale;
    // let collider_size_x = (sprite_size_x * sprite_scale) / rapier_config.scale;
    // let collider_size_y = (sprite_size_y * sprite_scale) / rapier_config.scale;

    // println!("Collider size: {}, {}", collider_size_x, collider_size_y);

    commands.insert_resource(ClearColor(
        Color::hex(&map.ldtk_file.levels[0].bg_color[1..]).unwrap(),
    ));

    for i in 0..map.ldtk_file.levels.len() {
        let level_ldtk_world_pos = Vec2::new(map.ldtk_file.levels[i].world_x as f32, map.ldtk_file.levels[i].world_y as f32);
        println!("World LDTKPos({:?})", level_ldtk_world_pos);
        for (idx, layer) in map.ldtk_file.levels[i].layer_instances.as_ref().unwrap().iter().enumerate().rev() {
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
                    if let Some(layer_tileset_def_uid) = layer.tileset_def_uid {
                        println!("Generating IntGrid Layer w/ Tiles: {}", layer.identifier);
                        for tile in layer.grid_tiles.iter() {
                            display_tile(
                                layer_info,
                                tile,
                                level_ldtk_world_pos,
                                &mut commands,
                                map_assets.0[&(layer_tileset_def_uid as i32)].clone()
                            )
                        } 
                    }
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
                                level_ldtk_world_pos,
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
                                    Vec2::new(entity.px[0] as f32, entity.px[1] as f32) + level_ldtk_world_pos, 
                                    Vec2::new(entity.width as f32, entity.height as f32), 
                                    Vec2::new(layer_info.px_width, layer_info.px_height), 
                                    4.0
                                );

                                println!("Creating collider Size({:?}) Position({:?})", bevy_half_extent, bevy_pos);

                                commands.spawn_bundle(BodyBundle {
                                    position: Position(bevy_pos),
                                    ..Default::default()
                                }).insert(AABB {
                                    position: IVec2::ZERO,
                                    half_size: IVec2::new(bevy_half_extent.x.round() as i32, bevy_half_extent.y.round() as i32),
                                });
                            }
                        }
                        "Entities" => {
                            for entity in layer.entity_instances.iter() {
                                println!("Entity: {}", entity.identifier);
    
                                let (mut bevy_pos, bevy_half_extent) = convert_ldtk_entity_to_bevy(
                                    Vec2::new(entity.px[0] as f32, entity.px[1] as f32) + level_ldtk_world_pos, 
                                    Vec2::new(entity.width as f32, entity.height as f32), 
                                    Vec2::new(layer_info.px_width, layer_info.px_height), 
                                    4.0
                                );
    
                                println!("Spawning at position: {:?} {:?}", bevy_pos, bevy_half_extent);
    
                                match &entity.identifier[..] {
                                    "Player" => {
                                        commands.spawn_bundle(BodyBundle {
                                            body_type: BodyType::Actor,
                                            position: Position(bevy_pos),
                                            ..Default::default()
                                        }).insert(AABB {
                                            position: IVec2::ZERO,
                                            half_size: IVec2::new(bevy_half_extent.x.round() as i32, bevy_half_extent.y.round() as i32),
                                        }).insert(CameraTarget);
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
    }


    map.redraw = false;
}

fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        // .add_startup_system(setup_game.system())
        .add_startup_system(setup_tilemap.system())
        .add_plugin(bevy_canvas::CanvasPlugin)
        .add_plugin(AnimationPlugin)
        .add_plugin(PlayerPlugin)
        .add_plugin(PhysicsPlugin)
        .add_plugin(DebugAABBPlugin)
        .add_system(update_ldtk_map.system())
        .add_system(move_camera.system())
        .add_system(gravity.system().before("MOVE_ACTORS"))
        // .add_system(debug_colliders.system())
        // .add_system(sprite_flip.system())
        .run();
}
