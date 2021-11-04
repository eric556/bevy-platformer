use bevy::{core::{Time, Timer}, math::Vec2, prelude::{Added, Commands, Entity, Query, Res}};
use crate::physics::{body::{Acceleration, Velocity}, collision::CollisionResult};

#[derive(Debug, Default)]
pub struct PlayerWalkParams {
    pub walk_accel: f32,
    pub max_walk_speed: f32,
}

#[derive(Debug, Default)]
pub struct PlayerJumpParams {
    pub gravity: Vec2,
    pub jump_acceleration: f32,
    pub max_jump_duration: f32,
    pub max_fall_speed: f32,
    pub jump_timer: Timer,
    pub grounded: bool,
    pub is_jumping: bool
}

pub fn integrate_movement(
    time: Res<Time>,
    mut body_query: Query<(&mut Velocity, &mut Acceleration, &PlayerWalkParams, &PlayerJumpParams)>
) {
    for (mut velocity, mut acceleration, player_walk_params, player_jump_params) in body_query.iter_mut() {
        let added_velocity = acceleration.0 * time.delta_seconds();
        let temp_velocity = if velocity.0.x.signum() == added_velocity.x.signum() || added_velocity.x == 0.0f32 {
            added_velocity + velocity.0
        } else {
            Vec2::new(added_velocity.x, added_velocity.y + velocity.0.y)
        };

        // Clamp the player speed
        velocity.0 = Vec2::new(
            temp_velocity.x.clamp(-player_walk_params.max_walk_speed, player_walk_params.max_walk_speed), 
            temp_velocity.y.max(player_jump_params.max_fall_speed)
        );
        acceleration.0 = Vec2::ZERO;
    }
}

pub fn gravity(
    mut body_query: Query<(&mut Acceleration, &PlayerJumpParams)>
) {
    for (mut accel, player_jump_params) in body_query.iter_mut() {
        accel.0 += player_jump_params.gravity;
    }
}

pub fn collision_check(
    mut commands: Commands,
    mut jump_state_query: Query<(Entity, &mut PlayerJumpParams, &CollisionResult), Added<CollisionResult>>
) {
    for (entity, mut jump_params, collision_result) in jump_state_query.iter_mut() {
        if !jump_params.grounded && collision_result.y_collision_body.is_some(){
            jump_params.grounded = true;
        }
        commands.entity(entity).remove::<CollisionResult>();
    }
}