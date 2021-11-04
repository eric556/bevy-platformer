use bevy::{ecs::schedule::GraphNode, prelude::*, sprite::collide_aabb::Collision};

#[cfg(target_arch = "x86_64")]
use bevy_canvas::{
    common_shapes::{self, Rectangle},
    Canvas, DrawMode,
};

use bevy_egui::{EguiContext, egui::{self, Window}};

use crate::{animation::{AnimatedSpriteBundle, Col, Row, SpriteSheetDefinition}, physics::{PhysicsStages, StepSystemLabels, body::{Acceleration, BodyBundle, Velocity}, collision::{AABB, CollisionResult}}};
use macros::animation_graph;

pub mod player_animation;
pub mod player_physics;

use self::{player_animation::{update_player_animation, Player::{PlayerAnimationUpdate, player_animation_update}}, player_physics::{PlayerJumpParams, PlayerWalkParams, collision_check, gravity, integrate_movement}};

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

#[derive(Bundle, Default)]
pub struct PlayerBundle {
    pub health: Health,
    #[bundle]
    pub body_bundle: BodyBundle,
    pub collider: AABB,
    #[bundle]
    pub animation: AnimatedSpriteBundle,
    pub input: PlayerInput,
    pub action: PlayerAnimationUpdate,
    pub player_walk_params: PlayerWalkParams,
    pub player_jump_params: PlayerJumpParams,
    pub acceleration: Acceleration
}



fn move_player(
    time: Res<Time>,
    keys: Res<Input<KeyCode>>,
    mut player_query: Query<(
        &PlayerInput,
        &PlayerWalkParams,
        &mut PlayerJumpParams,
        &mut Velocity,
        &mut Acceleration
    )>,
) {
    for (p_input, player_walk_params, mut player_jump_params, mut vel, mut accel) in
        player_query.iter_mut()
    {
        if vel.0.y != 0.0 {
            player_jump_params.grounded = false;
        }

        if (!keys.pressed(p_input.left) && !keys.pressed(p_input.right))
            || (keys.pressed(p_input.left) && keys.pressed(p_input.right))
        {
            vel.0.x = 0.0;
        } else if keys.pressed(p_input.left) {
            accel.0.x += -player_walk_params.walk_accel;
        } else if keys.pressed(p_input.right) {
            accel.0.x += player_walk_params.walk_accel;
        }

        if player_jump_params.grounded && keys.just_pressed(p_input.jump) {
            player_jump_params.is_jumping = true;
            player_jump_params.grounded = false;
            player_jump_params.jump_timer = Timer::from_seconds(player_jump_params.max_jump_duration, false);
        }

        if keys.pressed(p_input.jump) && player_jump_params.is_jumping {
            if !player_jump_params.jump_timer.finished() {
                accel.0.y += player_jump_params.jump_acceleration;
                player_jump_params.jump_timer.tick(time.delta());
            } else {
                player_jump_params.is_jumping = false;
            }
        }

        if keys.just_released(p_input.jump) {
            player_jump_params.is_jumping = false;
        }
    }
}

fn debug_player_params(
    mut egui_ctx: ResMut<EguiContext>,
    mut player_params_query: Query<(&mut PlayerJumpParams, &mut PlayerWalkParams)>,
) {
    Window::new("Bodies").scroll(true).show(egui_ctx.ctx(), |ui| {
        let mut i = 0u32;
        for (mut jump_params, mut walk_params) in player_params_query.iter_mut() {
            ui.collapsing(format!("Player {}", i), |ui| {
                egui::Grid::new(format!("Player {} prams", i)).show(ui, |ui|{
                    ui.label("Walk Accel");
                    ui.add_sized([40.0, 20.0], egui::DragValue::new(&mut walk_params.walk_accel));
                    ui.end_row();
                    ui.label("Max Walk Speed");
                    ui.add_sized([40.0, 20.0], egui::DragValue::new(&mut walk_params.max_walk_speed));
                    ui.end_row();
                    ui.separator();
                    ui.end_row();
                    ui.label("Gravity");
                    ui.add_sized([40.0, 20.0], egui::DragValue::new(&mut jump_params.gravity.x));
                    ui.add_sized([40.0, 20.0], egui::DragValue::new(&mut jump_params.gravity.y));
                    ui.end_row();
                    ui.label("Jump Acceleration");
                    ui.add_sized([40.0, 20.0], egui::DragValue::new(&mut jump_params.jump_acceleration));
                    ui.end_row();
                    ui.label("Max Jump Duration");
                    ui.add_sized([40.0, 20.0], egui::DragValue::new(&mut jump_params.max_jump_duration));
                    ui.end_row();
                    ui.label("Max Fall Speed");
                    ui.add_sized([40.0, 20.0], egui::DragValue::new(&mut jump_params.max_fall_speed));
                    ui.end_row();
                    ui.checkbox(&mut jump_params.grounded, "Grounded");
                    ui.end_row();
                    ui.checkbox(&mut jump_params.is_jumping, "Is Jumping");
                    ui.end_row();

                });
            });
            ui.separator();
            i += 1;
        }
    });
}

pub struct PlayerDebugPlugin;

impl Plugin for PlayerDebugPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_system(debug_player_params.system());
    }
}

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app
            .add_system_to_stage(PhysicsStages::PreStep, move_player.system().label("MOVE_PLAYER"))
            .add_system_to_stage(PhysicsStages::PreStep, gravity.system().after("MOVE_PLAYER"))
            .add_system_to_stage(PhysicsStages::Step, integrate_movement.system().label("INTEGRATE_PLAYER").before(StepSystemLabels::MoveActors))
            .add_system_to_stage(PhysicsStages::PostStep, collision_check.system().label("COLLISION_CHECK"))

            .add_system(update_player_animation.system().after("player_animation_update"))
            .add_system(player_animation_update.system().label("player_animation_update"));
    }
}