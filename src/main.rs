use core::panic;
use std::{collections::HashMap, default};

use animation::{AnimationPlugin, Col, Row, SpriteSheetDefinition};
use bevy::{math::Vec3Swizzles, prelude::*};
use bevy_egui::EguiPlugin;
use bevy_mod_debugdump::schedule_graph::schedule_graph_dot;
use fastapprox::fast::ln;
use ldtk::ldtk_json::{Project, TileInstance};
use physics::{DebugPhysicsPlugin, PhysicsPlugin, body::{Velocity}};
use player::{PlayerJumpParams, PlayerPlugin, PlayerWalkParams};

use crate::{animation::{AnimatedSpriteBundle, AnimationDefinition}, camera::{CameraPlugin, CameraTarget, MainCamera}, ldtk::LdtkLoaderPlugin, physics::{
        body::{BodyBundle, BodyType, Position},
        collision::AABB,
    }, player::{Health, PlayerBundle}};

pub mod animation;
pub mod physics;
pub mod player;
pub mod camera;
pub mod ldtk;

#[derive(Clone)]
struct LdtkMapAssets(HashMap<i32, Handle<TextureAtlas>>);

struct Map {
    ldtk_file: Handle<Project>,
    redraw: bool,
    current_level: usize,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
enum AppState {
    Loading,
    InGame
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

#[derive(Debug, Default)]
pub struct Scale(pub f32);

pub struct PlayerAnimationsAssets {
    pub texture_atlas: Handle<TextureAtlas>,
    pub animation_definitions: Vec<AnimationDefinition>
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

// Spawn a tile. Check to see if it needs to flip on the x and/or y axis before spawning.
fn spawn_tile(
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

fn sprite_flip(mut sprite_query: Query<(&Velocity, &mut TextureAtlasSprite)>) {
    for (vel, mut sprite) in sprite_query.iter_mut() {
        if vel.0.x < 0.0 {
            sprite.flip_x = true;
        } else if vel.0.x > 0.0 {
            sprite.flip_x = false;
        }
    }
}

fn spawn_collider(
    commands: &mut Commands,
    position: Vec2,
    half_extents: Vec2,

) {
    commands.spawn_bundle(BodyBundle {
        position: Position(position),
        ..Default::default()
    })
    .insert(AABB {
        position: IVec2::ZERO,
        half_size: IVec2::new(
            half_extents.x.round() as i32,
            half_extents.y.round() as i32,
        ),
    });
}

fn spawn_player(
    commands: &mut Commands,
    player_animations: &PlayerAnimationsAssets,
    position: Vec2,
    half_extents: Vec2,
    scale: f32
) {
    commands
    .spawn_bundle(PlayerBundle {
        health: Health(10u32),
        body_bundle: BodyBundle {
            body_type: BodyType::Actor,
            position: Position(position),
            ..Default::default()
        },
        collider: AABB {
            position: IVec2::ZERO,
            half_size: IVec2::new(
                half_extents.x.round() as i32,
                half_extents.y.round() as i32,
            ),
        },
        animation: AnimatedSpriteBundle {
            sprite_sheet: SpriteSheetBundle {
                texture_atlas:
                player_animations.texture_atlas.clone(),
                transform: Transform::from_scale(
                    Vec3::splat(scale),
                ),
                ..Default::default()
            },
            sprite_sheet_definitions:
                SpriteSheetDefinition {
                    animation_definitions:
                    player_animations.animation_definitions.clone(),
                    rows: 15,
                    columns: 8,
                },
            animation_timer: Timer::from_seconds(0.1, true),
            current_row: Row(5), // Set it up as the idle animation right away
            current_col: Col(0),
        },
        player_walk_params: PlayerWalkParams {
            walk_accel: 6000f32,
            max_walk_speed: 700f32,
            ..Default::default()
        },
        player_jump_params: PlayerJumpParams {
            gravity: Vec2::new(0f32, -3000f32),
            rising_gravity: Vec2::new(0f32, -3000f32),
            jump_acceleration: 7000f32,
            max_jump_duration: 0.08f32,
            max_fall_speed: -700f32,
            jump_timer: Timer::from_seconds(0.08, false),
        },
        ..Default::default()
    })
    .insert(CameraTarget);
}

fn setup_animation_assets(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
) {
    let hero_char_texture_sheet_handle = asset_server.load("herochar_spritesheet.png");
    let hero_char_atlas = TextureAtlas::from_grid(hero_char_texture_sheet_handle, Vec2::new(16.0, 16.0), 8, 15);

    let player_animation_assets = PlayerAnimationsAssets {
        texture_atlas: texture_atlases.add(hero_char_atlas),
        animation_definitions: vec![
            AnimationDefinition {name: String::from("death"), number_of_frames: 8, frame_time: 0.0, repeating: true},
            AnimationDefinition {name: String::from("run"), number_of_frames: 6, frame_time: 0.07, repeating: true},
            AnimationDefinition {name: String::from("pushing"), number_of_frames: 6, frame_time: 0.1, repeating: true},
            AnimationDefinition {name: String::from("attack_no_slash"), number_of_frames: 4, frame_time: 0.1, repeating: false},
            // ? What should we do about long boy animations (multiframe)
            AnimationDefinition {name: String::from("attack_slash"), number_of_frames: 8, frame_time: 0.1, repeating: false},
            AnimationDefinition {name: String::from("idle"), number_of_frames: 4, frame_time: 0.1, repeating: true},
            AnimationDefinition {name: String::from("falling"), number_of_frames: 3, frame_time: 0.07, repeating: true},
            AnimationDefinition {name: String::from("jumping"), number_of_frames: 3, frame_time: 0.07, repeating: true},
        ],
    };

    commands.insert_resource(player_animation_assets);
}

fn load_tilemap(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    ldtk_maps: Res<Assets<Project>>
) {
    // Load up the map
    let map = Map {
        ldtk_file: asset_server.load("test-world.ldtk"),
        // ldtk_file: Project::new(String::from("assets/physics-testing.ldtk")),
        redraw: true,
        current_level: 0,
    };

    // Slap these bad boys into resources
    commands.insert_resource(map);
}

fn load_tilesets(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
    map: Res<Map>,
    ldtk_maps: Res<Assets<Project>>,
    mut state: ResMut<State<AppState>>
) {
    // Go through and grab all the map tile sets

    if let Some(ldtk_file) = ldtk_maps.get(&map.ldtk_file) {
        let mut map_assets = LdtkMapAssets(HashMap::new());

        for tileset in ldtk_file.defs.tilesets.iter() {
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

        commands.insert_resource(map_assets);
        state.set(AppState::InGame);
    }

}

fn update_ldtk_map(
    mut commands: Commands,
    mut map: ResMut<Map>,
    map_assets: Res<LdtkMapAssets>,
    ldtk_maps: Res<Assets<Project>>,
    scale: Res<Scale>,
    player_animations: Res<PlayerAnimationsAssets>
) {
    if !map.redraw {
        return;
    }

    commands.spawn_bundle(OrthographicCameraBundle::new_2d()).insert(MainCamera);

    if let Some(ldtk_file) = ldtk_maps.get(&map.ldtk_file) {
        commands.insert_resource(ClearColor(
            Color::hex(&ldtk_file.levels[0].bg_color[1..]).unwrap(),
        ));

        for i in 0..ldtk_file.levels.len() {
            let level_ldtk_world_pos = Vec2::new(
                ldtk_file.levels[i].world_x as f32,
                ldtk_file.levels[i].world_y as f32,
            );
            println!("World LDTKPos({:?})", level_ldtk_world_pos);
            for (idx, layer) in ldtk_file.levels[i]
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
                                spawn_tile(
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
                                spawn_tile(
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

                                    spawn_collider(&mut commands, bevy_pos, bevy_half_extent);
                                }
                            }
                            "Entities" => {
                                for entity in layer.entity_instances.iter() {
                                    println!("Entity: {}", entity.identifier);

                                    let (bevy_pos, bevy_half_extent) = convert_ldtk_entity_to_bevy(
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
                                        "Player" => spawn_player(&mut commands, &player_animations, bevy_pos, bevy_half_extent, scale.0),
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

    } else {
        map.redraw = true;
    }
}

fn main() {
    let mut app = App::build();
    // Resources
    app.insert_resource(Scale(4.0))
        .insert_resource(WindowDescriptor {
            width: 1920.0,
            height: 1080.0,
            ..Default::default()
        });

    // Plugins
    app.add_plugins(DefaultPlugins)
        .add_plugin(EguiPlugin)
        .add_plugin(LdtkLoaderPlugin)
        .add_plugin(PhysicsPlugin)
        .add_plugin(AnimationPlugin)
        .add_plugin(PlayerPlugin)
        .add_plugin(CameraPlugin)
        .add_plugin(DebugPhysicsPlugin);

    // states
    app.add_state(AppState::Loading);

    // Loading state
    app.add_system_set(SystemSet::on_enter(AppState::Loading).with_system(load_tilemap.system()));
    app.add_system_set(SystemSet::on_update(AppState::Loading).with_system(load_tilesets.system()));
    
    // InGame state
    app.add_system_set(SystemSet::on_enter(AppState::InGame).with_system(setup_animation_assets.system()));
    app.add_system_set(SystemSet::on_update(AppState::InGame).with_system(update_ldtk_map.system()));
    app.add_system_set(SystemSet::on_update(AppState::InGame).with_system(sprite_flip.system()));
        
    // Dumping the schedule as a graphviz graph
    // println!("{}", schedule_graph_dot(&app.app.schedule));

    #[cfg(target_arch = "x86_64")]
    app.add_plugin(bevy_canvas::CanvasPlugin);

    #[cfg(target_arch = "wasm32")]
    app.add_plugin(bevy_webgl2::WebGL2Plugin);

    app.run();
}
