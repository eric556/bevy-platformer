use bevy::prelude::*;
use bevy_canvas::{
    common_shapes::{self, Rectangle},
    Canvas, DrawMode,
};

use crate::{animation::{AnimatedSpriteBundle, Col, Row, SpriteSheetDefinition}, physics::{PhysicsStages, StepSystemLabels, body::{Acceleration, BodyBundle, BodyParams, BodyType, Position, Remainder, Velocity}, collision::AABB}};
use macros::animation_graph;

animation_graph!(
    Player,
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
    pub body_bundle: BodyBundle,
    pub collider: AABB,
    #[bundle]
    pub animation: AnimatedSpriteBundle,
    pub input: PlayerInput,
    pub state: PlayerState,
    pub action: Player::PlayerAnimationUpdate,
    pub player_stats: PlayerStats,
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
    mut body_query: Query<(&mut Velocity, &mut Acceleration, &BodyParams)>
) {
    for (mut velocity, mut acceleration, body_params) in body_query.iter_mut() {
        let added_velocity = acceleration.0 * time.delta_seconds();
        let temp_velocity = if velocity.0.x.signum() == added_velocity.x.signum() || added_velocity.x == 0.0f32 {
            added_velocity + velocity.0
        } else {
            Vec2::new(added_velocity.x, added_velocity.y + velocity.0.y)
        };
        let clamped_movement = if body_params.max_speed.is_some() {
            temp_velocity.clamp(-body_params.max_speed.unwrap(), body_params.max_speed.unwrap())
        } else {
            temp_velocity
        };

        velocity.0 = clamped_movement;
        acceleration.0 = Vec2::ZERO;
    }
}

fn move_player(
    keys: Res<Input<KeyCode>>,
    mut player_query: Query<(
        &PlayerInput,
        &PlayerStats,
        &PlayerState,
        &mut Velocity,
        &mut Acceleration
    )>,
) {
    for (p_input, player_stats, state, mut vel, mut accel) in
        player_query.iter_mut()
    {
        if (!keys.pressed(p_input.left) && !keys.pressed(p_input.right))
            || (keys.pressed(p_input.left) && keys.pressed(p_input.right))
        {
            vel.0.x = 0.0;
        } else if keys.pressed(p_input.left) {
            accel.0.x += -player_stats.speed_up;
        } else if keys.pressed(p_input.right) {
            accel.0.x += player_stats.speed_up;
        }

        if keys.pressed(p_input.jump) {
            accel.0.y += player_stats.speed_up;
        }
        else if keys.pressed(KeyCode::S) {
            accel.0.y += -player_stats.speed_up;
        }
    }
}

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app
            .add_system_to_stage(PhysicsStages::PreStep, move_player.system().label("MOVE_PLAYER"))
            .add_system_to_stage(PhysicsStages::Step, integrate_movement.system().before(StepSystemLabels::MoveActors))
            .add_system(update_player_animation.system().after("player_animation_update"))
            .add_system(Player::player_animation_update.system().label("player_animation_update"));
    }
}