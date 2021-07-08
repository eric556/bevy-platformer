use bevy::prelude::*;
use bevy_canvas::{
    common_shapes::{self, Rectangle},
    Canvas, DrawMode,
};
use bevy_rapier2d::{
    physics::{
        ColliderBundle, QueryPipelineColliderComponentsQuery, QueryPipelineColliderComponentsSet,
        RapierConfiguration, RigidBodyBundle,
    },
    prelude::{
        ColliderPosition, ColliderShape, Cuboid, InteractionGroups, QueryPipeline, RigidBodyForces,
        RigidBodyMassProps, RigidBodyVelocity,
    },
};

use crate::{GROUND_GROUP, SHAPE_CAST_GROUP};
pub struct PlayerTextureAtlasHandles {
    pub idle_texture_atlas: Handle<TextureAtlas>,
    pub run_texture_atlas: Handle<TextureAtlas>,
    pub pre_jump_texture_atlaas: Handle<TextureAtlas>,
    pub jump_up_texture_atlas: Handle<TextureAtlas>,
    pub jump_down_texture_atlas: Handle<TextureAtlas>,
}

#[derive(Default)]
pub struct Health(pub u32);

pub struct PlayerInput {
    pub left: KeyCode,
    pub right: KeyCode,
    pub jump: KeyCode,
    pub crouch: KeyCode,
}

impl Default for PlayerInput {
    fn default() -> Self {
        PlayerInput {
            left: KeyCode::A,
            right: KeyCode::D,
            jump: KeyCode::Space,
            crouch: KeyCode::S,
        }
    }
}

#[derive(PartialEq, Debug)]
pub enum PlayerAction {
    Idle,
    Running,
    Jumping,
    Falling,
}

impl Default for PlayerAction {
    fn default() -> Self {
        Self::Idle
    }
}

#[derive(Default)]
pub struct PlayerState {
    pub grounded: bool,
}

#[derive(Default)]
pub struct PlayerStats {
    pub max_run_speed: f32,
    pub speed_up: f32,
}

#[derive(Bundle, Default)]
pub struct PlayerBundle {
    pub health: Health,
    #[bundle]
    pub rigid_body: RigidBodyBundle,
    #[bundle]
    pub collider: ColliderBundle,
    #[bundle]
    pub sprite_sheet: SpriteSheetBundle,
    pub animation_timer: Timer,
    pub input: PlayerInput,
    pub state: PlayerState,
    pub action: PlayerAction,
    pub player_stats: PlayerStats,
}

fn update_player_texture_atlas(
    player_texture_handles: Res<PlayerTextureAtlasHandles>,
    texture_atlases: Res<Assets<TextureAtlas>>,
    mut player_query: Query<
        (
            &PlayerAction,
            &mut Timer,
            &mut TextureAtlasSprite,
            &mut Handle<TextureAtlas>,
        ),
        Changed<PlayerAction>,
    >,
) {
    for (player_action, mut timer, mut sprite, mut current_atlas_handle) in player_query.iter_mut()
    {
        match player_action {
            PlayerAction::Idle => {
                *current_atlas_handle = player_texture_handles.idle_texture_atlas.clone_weak();
                *timer = Timer::from_seconds(0.1, true);
            }
            PlayerAction::Running => {
                *current_atlas_handle = player_texture_handles.run_texture_atlas.clone_weak();
                *timer = Timer::from_seconds(0.07, true);
            }
            PlayerAction::Falling => {
                *current_atlas_handle = player_texture_handles.jump_down_texture_atlas.clone_weak();
                *timer = Timer::from_seconds(0.07, true);
            }
            PlayerAction::Jumping => {
                *current_atlas_handle = player_texture_handles.jump_up_texture_atlas.clone_weak();
                *timer = Timer::from_seconds(0.07, true);
            }
            _ => todo!("Implement rest of player state animations"),
        }

        if let Some(current_atlas) = texture_atlases.get(current_atlas_handle.clone_weak()) {
            if sprite.index as usize > current_atlas.len() {
                sprite.index = 0;
            }
        }
    }
}

fn update_player_action(mut player_query: Query<(&RigidBodyVelocity, &mut PlayerAction)>) {
    for (vel, mut player_action) in player_query.iter_mut() {
        match *player_action {
            PlayerAction::Idle => {
                if vel.linvel.y != 0.0 {
                    if vel.linvel.y.signum() > 0.0 {
                        *player_action = PlayerAction::Jumping;
                    } else if vel.linvel.y.signum() < 0.0 {
                        *player_action = PlayerAction::Falling;
                    }
                }

                if vel.linvel.x != 0.0 {
                    *player_action = PlayerAction::Running;
                }
            }
            PlayerAction::Running => {
                if vel.linvel.y != 0.0 {
                    if vel.linvel.y.signum() > 0.0 {
                        *player_action = PlayerAction::Jumping;
                    } else if vel.linvel.y.signum() < 0.0 {
                        *player_action = PlayerAction::Falling;
                    }
                }

                if vel.linvel.x == 0.0 {
                    *player_action = PlayerAction::Idle;
                }
            }
            PlayerAction::Jumping => {
                if vel.linvel.y == 0.0 {
                    if vel.linvel.x != 0.0 {
                        *player_action = PlayerAction::Running;
                    } else {
                        *player_action = PlayerAction::Idle;
                    }
                } else if vel.linvel.y.signum() < 0.0 {
                    *player_action = PlayerAction::Falling;
                }
            }
            PlayerAction::Falling => {
                if vel.linvel.y == 0.0 {
                    if vel.linvel.x != 0.0 {
                        *player_action = PlayerAction::Running;
                    } else {
                        *player_action = PlayerAction::Idle;
                    }
                } else if vel.linvel.y.signum() > 0.0 {
                    *player_action = PlayerAction::Jumping;
                }
            }
        }
    }
}

fn update_player_grounded(
    query_pipeline: Res<QueryPipeline>,
    collider_query: QueryPipelineColliderComponentsQuery,
    rapier_params: Res<RapierConfiguration>,
    mut canvas: ResMut<Canvas>,
    mut player_query: Query<(&ColliderPosition, &ColliderShape, &mut PlayerState)>,
) {
    let collider_set = QueryPipelineColliderComponentsSet(&collider_query);

    for (col_pos, col_shape, mut state) in player_query.iter_mut() {
        let bounds = col_shape.compute_aabb(&col_pos.0);
        let bounds_half_extents: Vec2 = bounds.half_extents().into();
        let collider_position = Vec2::from(col_pos.translation);
        // same width, half the hight of the bounding boxs
        let shape_down_up_width_adjust = 0.1 * bounds_half_extents.x;
        let shape_down_up = Cuboid::new(
            Vec2::new(
                bounds_half_extents.x - shape_down_up_width_adjust,
                bounds_half_extents.y / 4.0,
            )
            .into(),
        );
        let shape_down_pos = [
            collider_position.x,
            collider_position.y - (bounds_half_extents.y),
        ]
        .into();
        let shape_vel = Vec2::new(0.0, -0.1).into();
        let max_toi = 4.0;
        let groups = InteractionGroups::new(SHAPE_CAST_GROUP, GROUND_GROUP);
        let filter = None;

        if let Some((handle, hit)) = query_pipeline.cast_shape(
            &collider_set,
            &shape_down_pos,
            &shape_vel,
            &shape_down_up,
            max_toi,
            groups,
            filter,
        ) {
            // The first collider hit has the handle `handle`. The `hit` is a
            // structure containing details about the hit configuration.
            // println!("Hit the entity with the configuration: {:?}", hit);
            state.grounded = true;
        } else {
            state.grounded = false;
        }

        canvas.draw(
            &Rectangle {
                origin: Vec2::from(shape_down_pos.translation) * rapier_params.scale,
                extents: Vec2::from(shape_down_up.half_extents) * rapier_params.scale * 2.0,
                anchor_point: common_shapes::RectangleAnchor::Center,
            },
            DrawMode::stroke_1px(),
            Color::BLUE,
        );
    }
}

fn move_player(
    keys: Res<Input<KeyCode>>,
    mut player_query: Query<(
        &PlayerInput,
        &PlayerStats,
        &mut RigidBodyVelocity,
        &mut RigidBodyForces,
        &RigidBodyMassProps,
        &mut PlayerAction,
        &PlayerState,
    )>,
) {
    for (p_input, player_stats, mut vel, mut forces, mass, mut player_action, state) in
        player_query.iter_mut()
    {
        let prev_vel_sign = vel.linvel.x.signum();

        if (!keys.pressed(p_input.left) && !keys.pressed(p_input.right))
            || (keys.pressed(p_input.left) && keys.pressed(p_input.right))
        {
            vel.linvel.x = 0.0;
        } else if keys.pressed(p_input.left) {
            if prev_vel_sign > 0.0 {
                vel.linvel.x = 0.0;
            }
            vel.linvel.x -= player_stats.speed_up;
            vel.linvel.x = vel.linvel.x.max(-player_stats.max_run_speed);
        } else if keys.pressed(p_input.right) {
            if prev_vel_sign < 0.0 {
                vel.linvel.x = 0.0;
            }
            vel.linvel.x += player_stats.speed_up;
            vel.linvel.x = vel.linvel.x.min(player_stats.max_run_speed);
        }

        if keys.pressed(p_input.jump) && state.grounded {
            // if vel.linvel.y < 10.0 {
            //     vel.linvel.y += 1.0;
            // }
            vel.apply_impulse(mass, Vec2::new(0.0, 5.0).into());
        }
    }
}

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_system(move_player.system())
            .add_system(update_player_texture_atlas.system())
            .add_system(update_player_grounded.system())
            .add_system(update_player_action.system());
    }
}
