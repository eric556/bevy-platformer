use bevy::{prelude::*, sprite::collide_aabb::Collision};

#[cfg(target_arch = "x86_64")]
use bevy_canvas::{
    common_shapes::{self, Rectangle},
    Canvas, DrawMode,
};

use bevy_egui::{EguiContext, egui::Window};

use crate::{animation::{AnimatedSpriteBundle, Col, Row, SpriteSheetDefinition}, physics::{PhysicsStages, StepSystemLabels, body::{Acceleration, BodyBundle, Velocity}, collision::{AABB, CollisionResult}}};
use macros::animation_graph;

animation_graph!(
    Player,
    {}, // No resources needed
    {vel: crate::physics::body::Velocity},
    Jump {
		Fall -> vel.0.y <= 0.0,
	},
	Fall {
		Idle -> vel.0.y == 0.0,
        Jump -> vel.0.y > 0.0
	},
	Idle {
		Jump -> vel.0.y != 0.0 && vel.0.y > 0.0,
		Fall -> vel.0.y != 0.0 && vel.0.y < 0.0,
		Run ->  vel.0.x != 0.0
	},
	Run {
		Jump -> vel.0.y != 0.0 && vel.0.y > 0.0,
		Fall -> vel.0.y != 0.0 && vel.0.y < 0.0,
		Idle -> vel.0.x == 0.0
	}
);

pub enum JumpState {
    Grounded,
    Jumping,
    Rising,
    Falling
}


impl Default for Player::PlayerAnimationUpdate {
    fn default() -> Self {
        Self::Idle
    }
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

#[derive(Debug, Default)]
pub struct PlayerWalkParams {
    pub walk_accel: f32,
    pub max_walk_speed: f32,
}

#[derive(Debug, Default)]
pub struct PlayerJumpParams {
    pub gravity: Vec2,
    pub rising_gravity: Vec2,
    pub jump_acceleration: f32,
    pub max_jump_duration: f32,
    pub max_fall_speed: f32,
    pub jump_timer: Timer
}

#[derive(Bundle, Default)]
pub struct PlayerBundle {
    pub health: Health,
    #[bundle]
    pub body_bundle: BodyBundle,
    pub collider: AABB,
    #[bundle]
    pub animation: AnimatedSpriteBundle,
    pub input: PlayerInput,
    pub action: Player::PlayerAnimationUpdate,
    pub player_walk_params: PlayerWalkParams,
    pub player_jump_params: PlayerJumpParams,
    pub acceleration: Acceleration
}

fn update_player_animation(
    mut player_query: Query<
        (
            &Player::PlayerAnimationUpdate,
            &SpriteSheetDefinition,
            &mut Timer,
            &mut Row,
            &mut Col
        ),
        Changed<Player::PlayerAnimationUpdate>,
    >,
) {
    for (player_action, sprite_sheet_def, mut timer, mut row, mut col) in player_query.iter_mut()
    {
        row.0 = match player_action {
            Player::PlayerAnimationUpdate::Idle => 5,
            Player::PlayerAnimationUpdate::Run => 1,
            Player::PlayerAnimationUpdate::Fall => 6,
            Player::PlayerAnimationUpdate::Jump => 7,
            _ => todo!("Implement rest of player state animations"),
        };

        // reset the timer
        let def = &sprite_sheet_def.animation_definitions[row.0];
        *timer = Timer::from_seconds(def.frame_time, def.repeating);

        // reset to begining of animation
        col.0 = 0;
    }
}

fn integrate_movement(
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

fn collision_check(
    mut commands: Commands,
    mut jump_state_query: Query<(Entity, &CollisionResult), Added<CollisionResult>>
) {
    for (entity, collision_result) in jump_state_query.iter_mut() {
        commands.entity(entity).remove::<CollisionResult>();
    }
}

fn move_player(
    keys: Res<Input<KeyCode>>,
    mut player_query: Query<(
        &PlayerInput,
        &PlayerWalkParams,
        &mut Velocity,
        &mut Acceleration
    )>,
) {
    for (p_input, player_walk_params, mut vel, mut accel) in
        player_query.iter_mut()
    {
        if (!keys.pressed(p_input.left) && !keys.pressed(p_input.right))
            || (keys.pressed(p_input.left) && keys.pressed(p_input.right))
        {
            vel.0.x = 0.0;
        } else if keys.pressed(p_input.left) {
            accel.0.x += -player_walk_params.walk_accel;
        } else if keys.pressed(p_input.right) {
            accel.0.x += player_walk_params.walk_accel;
        }

        // if keys.pressed(p_input.jump) {
        //     accel.0.y += player_walk_params.walk_accel;
        // }
        // else if keys.pressed(KeyCode::S) {
        //     accel.0.y += -player_walk_params.walk_accel;
        // }
    }
}

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app
            .add_system_to_stage(PhysicsStages::PreStep, move_player.system().label("MOVE_PLAYER"))
            .add_system_to_stage(PhysicsStages::Step, integrate_movement.system().label("INTEGRATE_PLAYER").before(StepSystemLabels::MoveActors))
            .add_system_to_stage(PhysicsStages::PostStep, collision_check.system().label("COLLISION_CHECK"))

            .add_system(update_player_animation.system().after("player_animation_update"))
            .add_system(Player::player_animation_update.system().label("player_animation_update"));
    }
}