use core::panic;
use std::collections::HashMap;

use animation::{AnimationPlugin, Col, Row, SpriteSheetDefinition};
use bevy::{math::Vec3Swizzles, prelude::*};
use bevy_mod_debugdump::schedule_graph::schedule_graph_dot;
use fastapprox::fast::ln;
use ldtk_rust::{Project, TileInstance};
use physics::{body::Velocity, DebugAABBPlugin, PhysicsPlugin};
use player::PlayerPlugin;

use crate::{animation::{AnimatedSpriteBundle, AnimationDefinition}, camera::{CameraPlugin, CameraTarget, MainCamera}, physics::{
        body::{BodyBundle, BodyType, Position},
        collision::AABB,
    }, player::{Health, PlayerBundle, PlayerStats}};

pub mod animation;
pub mod physics;
pub mod player;
pub mod camera;

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

fn sprite_flip(mut sprite_query: Query<(&Velocity, &mut TextureAtlasSprite)>) {
    for (vel, mut sprite) in sprite_query.iter_mut() {
        if vel.0.x < 0.0 {
            sprite.flip_x = true;
        } else if vel.0.x > 0.0 {
            sprite.flip_x = false;
        }
    }
}

fn setup_tilemap(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
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
        map_assets
            .0
            .insert(tileset.uid as i32, texture_atlas_handle);
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
    scale: &Scale
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
                scale.0,
                (tile.px[0] as f32 + level_world_pos.x) as i32,
                (tile.px[1] as f32 + level_world_pos.y) as i32,
                layer_info.z_index,
            ),
            rotation: flip(flip_x, flip_y),
            scale: Vec3::splat(scale.0),
        },
        sprite: TextureAtlasSprite::new(tile.t as u32),
        texture_atlas: handle,
        ..Default::default()
    });
}

pub fn convert_ldtk_entity_to_bevy(
    position: Vec2,
    size: Vec2,
    layer_size: Vec2,
    scale: f32,
) -> (Vec2, Vec2) {
    let ldtk_half_extents = size / 2.0;
    let ldtk_position = position + ldtk_half_extents;

    (
        Vec2::new(
            (ldtk_position.x * scale) - (layer_size.x / 2.),
            -(ldtk_position.y * scale) + (layer_size.y / 2.),
        ),
        ldtk_half_extents * scale,
    )
}

fn update_ldtk_map(
    mut commands: Commands,
    mut map: ResMut<Map>,
    map_assets: Res<LdtkMapAssets>,
    asset_server: Res<AssetServer>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
    scale: Res<Scale>
) {
    if !map.redraw {
        return;
    }

    let hero_char_texture_sheet_handle = asset_server.load("herochar_spritesheet.png");
    let hero_char_atlas =
        TextureAtlas::from_grid(hero_char_texture_sheet_handle, Vec2::new(16.0, 16.0), 8, 15);
    let hero_char_texture_atalas_handle = texture_atlases.add(hero_char_atlas);

    let hero_char_animation_definitions: Vec<AnimationDefinition> = vec![
        AnimationDefinition {name: String::from("death"), number_of_frames: 8, frame_time: 0.0, repeating: true},
        AnimationDefinition {name: String::from("run"), number_of_frames: 6, frame_time: 0.07, repeating: true},
        AnimationDefinition {name: String::from("pushing"), number_of_frames: 6, frame_time: 0.1, repeating: true},
        AnimationDefinition {name: String::from("attack_no_slash"), number_of_frames: 4, frame_time: 0.1, repeating: false},
        // ? What should we do about long boy animations (multiframe)
        AnimationDefinition {name: String::from("attack_slash"), number_of_frames: 8, frame_time: 0.1, repeating: false},
        AnimationDefinition {name: String::from("idle"), number_of_frames: 4, frame_time: 0.1, repeating: true},
        AnimationDefinition {name: String::from("falling"), number_of_frames: 3, frame_time: 0.07, repeating: true},
        AnimationDefinition {name: String::from("jumping"), number_of_frames: 3, frame_time: 0.07, repeating: true},
    ];

    commands
        .spawn_bundle(OrthographicCameraBundle::new_2d())
        .insert(MainCamera);

    commands.insert_resource(ClearColor(
        Color::hex(&map.ldtk_file.levels[0].bg_color[1..]).unwrap(),
    ));

    for i in 0..map.ldtk_file.levels.len() {
        let level_ldtk_world_pos = Vec2::new(
            map.ldtk_file.levels[i].world_x as f32,
            map.ldtk_file.levels[i].world_y as f32,
        );
        println!("World LDTKPos({:?})", level_ldtk_world_pos);
        for (idx, layer) in map.ldtk_file.levels[i]
            .layer_instances
            .as_ref()
            .unwrap()
            .iter()
            .enumerate()
            .rev()
        {
            let tileset_uid = layer.tileset_def_uid.unwrap_or(-1) as i32;
            let layer_uid = layer.layer_def_uid as i32;

            let layer_info = LayerInfo {
                grid_width: layer.c_wid as i32,
                _grid_height: layer.c_hei as i32,
                grid_cell_size: layer.grid_size as i32,
                z_index: 50 - idx as i32,
                // todo gotta swap this over from a hard coded scale
                px_width: layer.c_wid as f32 * (layer.grid_size as f32 * scale.0),
                px_height: layer.c_hei as f32 * (layer.grid_size as f32 * scale.0),
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
                                map_assets.0[&(layer_tileset_def_uid as i32)].clone(),
                                &scale
                            )
                        }
                    }
                }
                "AutoLayer" => {}
                "IntGrid" => {
                    if let Some(layer_tileset_def_uid) = layer.tileset_def_uid {
                        println!("Generating IntGrid Layer w/ Tiles: {}", layer.identifier);
                        for tile in layer.auto_layer_tiles.iter() {
                            display_tile(
                                layer_info,
                                tile,
                                level_ldtk_world_pos,
                                &mut commands,
                                map_assets.0[&(layer_tileset_def_uid as i32)].clone(),
                                &scale
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
                                    Vec2::new(entity.px[0] as f32, entity.px[1] as f32)
                                        + level_ldtk_world_pos,
                                    Vec2::new(entity.width as f32, entity.height as f32),
                                    Vec2::new(layer_info.px_width, layer_info.px_height),
                                    scale.0,
                                );

                                println!(
                                    "Creating collider Size({:?}) Position({:?})",
                                    bevy_half_extent, bevy_pos
                                );

                                commands
                                    .spawn_bundle(BodyBundle {
                                        position: Position(bevy_pos),
                                        ..Default::default()
                                    })
                                    .insert(AABB {
                                        position: IVec2::ZERO,
                                        half_size: IVec2::new(
                                            bevy_half_extent.x.round() as i32,
                                            bevy_half_extent.y.round() as i32,
                                        ),
                                    });
                            }
                        }
                        "Entities" => {
                            for entity in layer.entity_instances.iter() {
                                println!("Entity: {}", entity.identifier);

                                let (mut bevy_pos, bevy_half_extent) = convert_ldtk_entity_to_bevy(
                                    Vec2::new(entity.px[0] as f32, entity.px[1] as f32)
                                        + level_ldtk_world_pos,
                                    Vec2::new(entity.width as f32, entity.height as f32),
                                    Vec2::new(layer_info.px_width, layer_info.px_height),
                                    scale.0,
                                );

                                println!(
                                    "Spawning at position: {:?} {:?}",
                                    bevy_pos, bevy_half_extent
                                );

                                match &entity.identifier[..] {
                                    "Player" => {
                                        commands
                                            .spawn_bundle(PlayerBundle {
                                                health: Health(10u32),
                                                body_bundle: BodyBundle {
                                                    body_type: BodyType::Actor,
                                                    position: Position(bevy_pos),
                                                    ..Default::default()
                                                },
                                                collider: AABB {
                                                    position: IVec2::ZERO,
                                                    half_size: IVec2::new(
                                                        bevy_half_extent.x.round() as i32,
                                                        bevy_half_extent.y.round() as i32,
                                                    ),
                                                },
                                                animation: AnimatedSpriteBundle {
                                                    sprite_sheet: SpriteSheetBundle {
                                                        texture_atlas:
                                                            hero_char_texture_atalas_handle.clone(),
                                                        transform: Transform::from_scale(
                                                            Vec3::splat(scale.0),
                                                        ),
                                                        ..Default::default()
                                                    },
                                                    sprite_sheet_definitions:
                                                        SpriteSheetDefinition {
                                                            animation_definitions:
                                                                hero_char_animation_definitions
                                                                    .clone(),
                                                            rows: 15,
                                                            columns: 8,
                                                        },
                                                    animation_timer: Timer::from_seconds(0.1, true),
                                                    current_row: Row(5), // Set it up as the idle animation right away
                                                    current_col: Col(0),
                                                },
                                                player_stats: PlayerStats {
                                                    max_run_speed: 200.0,
                                                    speed_up: 1000.0,
                                                },
                                                ..Default::default()
                                            })
                                            .insert(CameraTarget);
                                    }
                                    _ => {}
                                }
                            }
                        }
                        _ => {}
                    }
                }
                _ => panic!("AHHHHHHHHH"),
            }
        }
    }

    map.redraw = false;
}

#[derive(Debug, Default)]
pub struct Scale(pub f32);

fn main() {
    let mut app = App::build();
    app.add_plugins(DefaultPlugins)
        .add_plugin(bevy_canvas::CanvasPlugin)
        .insert_resource(Scale(16.0))
        .add_plugin(PhysicsPlugin)
        .add_plugin(AnimationPlugin)
        .add_plugin(PlayerPlugin)
        .add_plugin(CameraPlugin)
        .add_plugin(DebugAABBPlugin)
        .add_startup_system(setup_tilemap.system())
        .add_system(update_ldtk_map.system())
        .add_system(sprite_flip.system());

    // Dumping the schedule as a graphviz graph
    println!("{}", schedule_graph_dot(&app.app.schedule));

    app.run();
}
