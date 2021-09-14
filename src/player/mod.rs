use bevy::prelude::*;
use bevy_canvas::{Canvas, DrawMode, common_shapes::{self, Line, Rectangle}};
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

use crate::{GROUND_GROUP, SHAPE_CAST_GROUP, animation::{AnimatedSpriteBundle, Col, Row, SpriteSheetDefinition}};

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

#[derive(PartialEq, Debug, PartialOrd)]
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
    pub time_to_apex: f32,
    pub min_jump_height: f32,
    pub max_jump_height: f32,
    pub max_run_speed: f32,
    pub speed_up: f32,
}

impl PlayerStats {
    pub fn apex(&self, gravity: f32) -> f32 {
        return ((-2.0 * self.max_jump_height) / gravity).sqrt();
    }

    pub fn max_jump_velocity(&self, gravity: f32) -> f32 {
        return gravity.abs() * self.apex(gravity)
    }

    pub fn min_jump_velocity(&self, gravity: f32) -> f32 {
        return (2.0 * gravity.abs() * self.min_jump_height).sqrt();
    }
}

#[derive(Bundle, Default)]
pub struct PlayerBundle {
    pub health: Health,
    #[bundle]
    pub rigid_body: RigidBodyBundle,
    #[bundle]
    pub collider: ColliderBundle,
    #[bundle]
    pub animation: AnimatedSpriteBundle,
    pub input: PlayerInput,
    pub state: PlayerState,
    pub action: PlayerAction,
    pub player_stats: PlayerStats,
}

fn update_player_animation(
    mut player_query: Query<
        (
            &PlayerAction,
            &SpriteSheetDefinition,
            &mut Timer,
            &mut Row,
            &mut Col
        ),
        Changed<PlayerAction>,
    >,
) {
    for (player_action, sprite_sheet_def, mut timer, mut row, mut col) in player_query.iter_mut()
    {
        row.0 = match player_action {
            PlayerAction::Idle => 5,
            PlayerAction::Running => 1,
            PlayerAction::Falling => 6,
            PlayerAction::Jumping => 7,
            _ => todo!("Implement rest of player state animations"),
        };

        // reset the timer
        let def = &sprite_sheet_def.animation_definitions[row.0];
        *timer = Timer::from_seconds(def.frame_time, def.repeating);

        // reset to begining of animation
        col.0 = 0;
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
                } else if vel.linvel.x != 0.0 {
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
                } else if vel.linvel.x == 0.0 {
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


fn check_player_collision(
    time: Res<Time>,
    query_pipeline: Res<QueryPipeline>,
    collider_query: QueryPipelineColliderComponentsQuery,
    rapier_params: Res<RapierConfiguration>,
    mut canvas: ResMut<Canvas>,
    mut player_query: Query<(&ColliderPosition, &ColliderShape, &mut RigidBodyVelocity, &mut PlayerState)>,
) {
    let collider_set = QueryPipelineColliderComponentsSet(&collider_query);
    for (col_pos, col_shape, mut vel, mut state) in player_query.iter_mut() {
        let bounds = col_shape.compute_aabb(&col_pos.0);
        let bounds_half_extents: Vec2 = bounds.half_extents().into();
        let bounds_size = bounds_half_extents * 2.0;
        let collider_position = Vec2::from(col_pos.translation);

        // ? What skin width do I really want here
        let height_adjustment = 0.1 * bounds_half_extents.y;
        let width_adjustment = 0.1 * bounds_half_extents.x;

        let shape = Cuboid::new(
            Vec2::new(
                bounds_half_extents.x - width_adjustment,
                bounds_half_extents.y - height_adjustment,
            )
            .into(),
        );

        let max_toi = time.delta_seconds();

        let shape_vel = vel.linvel;
        let groups = InteractionGroups::new(SHAPE_CAST_GROUP, GROUND_GROUP);
        let filter = None;

        if let Some((collider_handle, toi)) = query_pipeline.cast_shape(
            &collider_set,
            col_pos,
            &shape_vel,
            &shape,
            max_toi,
            groups,
            filter,
        ) {
            // The first collider hit has the handle `handle`. The `hit` is a
            // structure containing details about the hit configuration.
            // println!("Hit the entity with the configuration: {:?}", toi);

            let hit_point: Vec2 = toi.witness1.into();
            let hit_point_scaled = hit_point * rapier_params.scale;
            let normal: Vec2 = toi.normal1.xy().into();
            println!("{:?}", toi.toi);

            if normal.normalize_or_zero().x.abs() != 0.0 {
                vel.linvel.x = 0.0;
                println!("Hitting wall {:?}", normal.normalize_or_zero());
            }

            canvas.draw(&Line(
                hit_point_scaled, hit_point_scaled + (normal.normalize_or_zero() * 100.0) 
            ), DrawMode::stroke_1px(), Color::GREEN);

        }

        canvas.draw(&Rectangle {
                origin: (collider_position + Vec2::from(shape_vel * max_toi)) * rapier_params.scale,
                extents: (bounds_half_extents - Vec2::new(width_adjustment, height_adjustment)) * rapier_params.scale * 2.0,
                anchor_point: common_shapes::RectangleAnchor::Center,
            },   
            DrawMode::stroke_1px(),
            Color::BLUE
        );

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
        let shape_vel = Vec2::new(0.0, -0.01).into();
        let max_toi = 0.005;
        let groups = InteractionGroups::new(SHAPE_CAST_GROUP, GROUND_GROUP);
        let filter = None;

        if let Some((_, _)) = query_pipeline.cast_shape(
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

        let shape_vel_normal = Vec2::new(0.0, -0.01) * max_toi;
        canvas.draw(
    &Line(
                Vec2::from(shape_down_pos.translation) * rapier_params.scale,  
                (Vec2::from(shape_down_pos.translation) + shape_vel_normal) * rapier_params.scale 
            ), 
            DrawMode::stroke_1px(),
            Color::BLUE,            
        );
    }
}

fn move_player(
    keys: Res<Input<KeyCode>>,
    time: Res<Time>,
    rapier_params: Res<RapierConfiguration>,
    mut player_query: Query<(
        &PlayerInput,
        &PlayerStats,
        &mut RigidBodyVelocity,
        &mut RigidBodyForces,
        &RigidBodyMassProps,
        &PlayerAction,
        &PlayerState,
    )>,
) {
    for (p_input, player_stats, mut vel, mut forces, mass, player_action, state) in
        player_query.iter_mut()
    {
        let prev_vel_sign = vel.linvel.x.signum();
        let dt = time.delta_seconds() * 60.0;
        // let dt = 1.0;

        if (!keys.pressed(p_input.left) && !keys.pressed(p_input.right))
            || (keys.pressed(p_input.left) && keys.pressed(p_input.right))
        {
            vel.linvel.x = 0.0;
        } else if keys.pressed(p_input.left) {
            if prev_vel_sign > 0.0 {
                vel.linvel.x = 0.0;
            }
            vel.linvel.x -= player_stats.speed_up * dt;
            vel.linvel.x = vel.linvel.x.max(-player_stats.max_run_speed);
        } else if keys.pressed(p_input.right) {
            if prev_vel_sign < 0.0 {
                vel.linvel.x = 0.0;
            }
            vel.linvel.x += player_stats.speed_up * dt;
            vel.linvel.x = vel.linvel.x.min(player_stats.max_run_speed);
        }

        let max_jump_vel = player_stats.max_jump_velocity(rapier_params.gravity.y) * dt;
        let min_jump_vel = player_stats.min_jump_velocity(rapier_params.gravity.y) * dt;

        if keys.pressed(p_input.jump) && state.grounded {
            vel.linvel.y = max_jump_vel;
            // vel.apply_impulse(mass, Vec2::new(0.0, 10.0).into());
        }
        if keys.just_released(p_input.jump) {
            if vel.linvel.y > min_jump_vel {
                vel.linvel.y = min_jump_vel;
            }
        }

        // vel.linvel = vel.linvel * time.delta_seconds() * 60.0;
    }
}

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_system(move_player.system().label("MOVE_PLAYER"))
            .add_system(update_player_animation.system().after("UPDATE_PLAYER_ACTION"))
            .add_system(update_player_grounded.system().after("MOVE_PLAYER"))
            .add_system(check_player_collision.system().after("MOVE_PLAYER"))
            .add_system(update_player_action.system().label("UPDATE_PLAYER_ACTION").after("MOVE_PLAYER"));
    }
}
